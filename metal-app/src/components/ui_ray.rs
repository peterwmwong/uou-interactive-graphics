use crate::{ModifierKeys, MouseButton, UserEvent};
use std::simd::f32x2;

pub const ROTATE_MOUSE_BUTTON: MouseButton = MouseButton::Left;
pub const DISTANCE_MOUSE_BUTTON: MouseButton = MouseButton::Right;

pub struct UIRay {
    pub distance_from_origin: f32,
    pub rotation_xy: f32x2,
    pub on_mouse_drag_modifier_keys: ModifierKeys,
    pub invert_drag: bool,
}

impl UIRay {
    pub fn new(
        on_mouse_drag_modifier_keys: ModifierKeys,
        distance_from_origin: f32,
        rotation_xy: f32x2,
        invert_drag: bool,
    ) -> Self {
        Self {
            distance_from_origin,
            rotation_xy,
            on_mouse_drag_modifier_keys,
            invert_drag,
        }
    }

    #[inline(always)]
    // Returns `true` if event matches and caused a change in distance or rotation
    pub fn on_event(&mut self, event: UserEvent) -> bool {
        match event {
            UserEvent::MouseDrag {
                button,
                modifier_keys,
                drag_amount,
                ..
            } => {
                let is_empty = self.on_mouse_drag_modifier_keys.is_empty();
                let drag_amount = if self.invert_drag {
                    -drag_amount
                } else {
                    drag_amount
                };
                if (is_empty && modifier_keys.is_empty())
                    || (!is_empty && modifier_keys.contains(self.on_mouse_drag_modifier_keys))
                {
                    match button {
                        ROTATE_MOUSE_BUTTON => self.drag_rotate(drag_amount),
                        DISTANCE_MOUSE_BUTTON => self.drag_distance(drag_amount),
                    }
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    #[inline]
    pub fn drag_rotate(&mut self, drag_amount: f32x2) {
        self.update(
            self.distance_from_origin,
            self.rotation_xy + {
                let adjacent = f32x2::splat(self.distance_from_origin);
                let opposite = drag_amount / f32x2::splat(500.);
                let &[x, y] = (opposite / adjacent).as_array();
                f32x2::from_array([
                    y.atan(), // Rotation on x-axis
                    x.atan(), // Rotation on y-axis
                ])
            },
        )
    }

    #[inline]
    pub fn drag_distance(&mut self, drag_amount: f32x2) {
        self.update(
            self.distance_from_origin - drag_amount[1] / 250.,
            self.rotation_xy,
        );
    }

    #[inline(always)]
    fn update(&mut self, new_distance_from_origin: f32, new_rotation_xy: f32x2) {
        self.distance_from_origin = new_distance_from_origin;
        self.rotation_xy = new_rotation_xy;
    }
}
