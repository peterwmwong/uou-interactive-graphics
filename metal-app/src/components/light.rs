use metal_types::f32x4x4;

use super::ui_ray::UIRay;
use crate::{ModifierKeys, UserEvent};
use std::simd::{f32x2, f32x4};

pub struct LightUpdate {
    pub position: f32x4,
}

pub struct Light {
    pub ray: UIRay,
}

impl Light {
    #[inline(always)]
    pub const fn new(
        init_distance: f32,
        init_rotation: f32x2,
        on_mouse_drag_modifier_keys: ModifierKeys,
        invert_drag: bool,
    ) -> Self {
        Self {
            ray: UIRay {
                distance_from_origin: init_distance,
                rotation_xy: init_rotation,
                on_mouse_drag_modifier_keys,
                invert_drag,
            },
        }
    }

    #[inline]
    pub fn on_event(&mut self, event: UserEvent, mut on_update: impl FnMut(LightUpdate)) {
        if self.ray.on_event(event) || matches!(event, UserEvent::WindowFocusedOrResized { .. }) {
            let &[rotx, roty] = self.ray.rotation_xy.as_array();
            let position = f32x4x4::rotate(rotx, roty, 0.)
                * f32x4::from_array([0., 0., -self.ray.distance_from_origin, 1.]);
            on_update(LightUpdate { position });
        }
    }
}
