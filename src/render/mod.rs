use crate::plugin::RapierContext;
use bevy::prelude::*;
use bevy::transform::TransformSystem;
use rapier::math::{Point, Real};
use rapier::pipeline::{DebugRenderBackend, DebugRenderObject, DebugRenderPipeline};
pub use rapier::pipeline::{DebugRenderMode, DebugRenderStyle};
use std::fmt::Debug;

/// The color of a collider when using the debug-renderer.
///
/// Insert this component alongside the collider component to
/// force to a specific value the color used to render the
/// collider.
#[derive(Copy, Clone, Component, PartialEq, Debug)]
pub struct ColliderDebugColor(pub Color);

/// Marker to draw debug lines for colliders if `global` in [`DebugRenderContext`]
/// is set to false.
#[derive(Copy, Clone, Component, PartialEq, Debug)]
pub struct ColliderDebug;

/// Plugin rensponsible for rendering (using lines) what Rapier "sees" when performing
/// its physics simulation. This is typically useful to check proper
/// alignment between colliders and your own visual assets.
pub struct RapierDebugRenderPlugin {
    /// Is the debug-rendering will be enabled for every entity? Or just for
    /// collider entities with [`ColliderDebug`] component.
    pub global: bool,
    /// Is the debug-rendering enabled?
    pub enabled: bool,
    /// Control some aspects of the render coloring.
    pub style: DebugRenderStyle,
    /// Flags to select what part of physics scene is rendered (by default
    /// everything is rendered).
    pub mode: DebugRenderMode,
}

#[allow(clippy::derivable_impls)] // The 3D impl can be derived, but not the 2D impl.
impl Default for RapierDebugRenderPlugin {
    #[cfg(feature = "dim2")]
    fn default() -> Self {
        Self {
            enabled: true,
            global: true,
            style: DebugRenderStyle {
                rigid_body_axes_length: 20.0,
                ..Default::default()
            },
            mode: DebugRenderMode::default(),
        }
    }
    #[cfg(feature = "dim3")]
    fn default() -> Self {
        Self {
            enabled: true,
            global: true,
            style: DebugRenderStyle::default(),
            mode: DebugRenderMode::default(),
        }
    }
}

impl RapierDebugRenderPlugin {
    /// Initialize the render plugin such that it is initially disabled.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Context to control some aspect of the debug-renderer after initialization.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct DebugRenderContext {
    /// Is the debug-rendering currently enabled?
    pub enabled: bool,
    /// Is the debug-rendering enabled for every entity? Otherwise it will only work
    /// for collider entities with [`ColliderDebug`] component.
    pub global: bool,
    /// Pipeline responsible for rendering. Access `pipeline.mode` and `pipeline.style`
    /// to modify the set of rendered elements, and modify the default coloring rules.
    #[reflect(ignore)]
    pub pipeline: DebugRenderPipeline,
}

impl Default for DebugRenderContext {
    fn default() -> Self {
        Self {
            enabled: true,
            global: true,
            pipeline: DebugRenderPipeline::default(),
        }
    }
}

impl Plugin for RapierDebugRenderPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<DebugRenderContext>();

        app.insert_resource(DebugRenderContext {
            enabled: self.enabled,
            global: self.global,
            pipeline: DebugRenderPipeline::new(self.style, self.mode),
        })
        .add_systems(
            PostUpdate,
            debug_render_scene.after(TransformSystem::TransformPropagate),
        );
    }
}

struct BevyLinesRenderBackend<'world, 'state, 'a, 'b, 'c> {
    physics_scale: f32,
    custom_colors: Query<'world, 'state, &'a ColliderDebugColor>,
    visible_colliders: Option<Query<'world, 'state, &'b ColliderDebug>>,
    context: &'c RapierContext,
    gizmos: Gizmos<'state>,
}

impl<'world, 'state> BevyLinesRenderBackend<'world, 'state, '_, '_, '_> {
    fn object_color(&self, object: DebugRenderObject, default: [f32; 4]) -> [f32; 4] {
        let color = match object {
            DebugRenderObject::Collider(h, ..) => self.context.colliders.get(h).and_then(|co| {
                self.custom_colors
                    .get(Entity::from_bits(co.user_data as u64))
                    .map(|co| co.0)
                    .ok()
            }),
            _ => None,
        };

        color.map(|co| co.as_hsla_f32()).unwrap_or(default)
    }

    fn drawing_enabled(&self, object: DebugRenderObject) -> bool {
        let Some(ref visible_colliders) = self.visible_colliders else { return true; };

        match object {
            DebugRenderObject::Collider(h, ..) => {
                let collider = self.context.colliders.get(h);
                collider
                    .map(|co| {
                        let entity = Entity::from_bits(co.user_data as u64);
                        visible_colliders.contains(entity)
                    })
                    .unwrap_or(false)
            }
            _ => true,
        }
    }
}

impl<'world, 'state> DebugRenderBackend for BevyLinesRenderBackend<'world, 'state, '_, '_, '_> {
    #[cfg(feature = "dim2")]
    fn draw_line(
        &mut self,
        object: DebugRenderObject,
        a: Point<Real>,
        b: Point<Real>,
        color: [f32; 4],
    ) {
        if !self.drawing_enabled(object) {
            return;
        }

        let scale = self.physics_scale;
        let color = self.object_color(object, color);
        self.gizmos.line(
            [a.x * scale, a.y * scale, 0.0].into(),
            [b.x * scale, b.y * scale, 0.0].into(),
            Color::hsla(color[0], color[1], color[2], color[3]),
        )
    }

    #[cfg(feature = "dim3")]
    fn draw_line(
        &mut self,
        object: DebugRenderObject,
        a: Point<Real>,
        b: Point<Real>,
        color: [f32; 4],
    ) {
        if !self.drawing_enabled(object) {
            return;
        }

        let scale = self.physics_scale;
        let color = self.object_color(object, color);
        self.gizmos.line(
            [a.x * scale, a.y * scale, a.z * scale].into(),
            [b.x * scale, b.y * scale, b.z * scale].into(),
            Color::hsla(color[0], color[1], color[2], color[3]),
        )
    }
}

fn debug_render_scene(
    rapier_context: Res<RapierContext>,
    mut render_context: ResMut<DebugRenderContext>,
    gizmos: Gizmos,
    custom_colors: Query<&ColliderDebugColor>,
    visible_colliders: Query<&ColliderDebug>,
) {
    if !render_context.enabled {
        return;
    }

    let visible_colliders = if render_context.global {
        None
    } else {
        Some(visible_colliders)
    };
    let mut backend = BevyLinesRenderBackend {
        physics_scale: rapier_context.physics_scale,
        custom_colors,
        visible_colliders,
        context: &rapier_context,
        gizmos,
    };

    let unscaled_style = render_context.pipeline.style;
    render_context.pipeline.style.rigid_body_axes_length /= rapier_context.physics_scale;
    render_context.pipeline.render(
        &mut backend,
        &rapier_context.bodies,
        &rapier_context.colliders,
        &rapier_context.impulse_joints,
        &rapier_context.multibody_joints,
        &rapier_context.narrow_phase,
    );
    render_context.pipeline.style = unscaled_style;
}
