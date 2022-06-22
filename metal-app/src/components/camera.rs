use metal_types::f32x4x4;

use super::ui_ray::UIRay;
use crate::{ModifierKeys, UserEvent};
use std::{
    ops::Neg,
    simd::{f32x2, f32x4},
};

const INITIAL_CAMERA_DISTANCE: f32 = 1.;
const N: f32 = 0.1;
const F: f32 = 100000.0;
const NEAR_FIELD_MAJOR_AXIS: f32 = N / INITIAL_CAMERA_DISTANCE;
const PERSPECTIVE_MATRIX: f32x4x4 = f32x4x4::new(
    [N, 0., 0., 0.],
    [0., N, 0., 0.],
    [0., 0., N + F, -N * F],
    [0., 0., 1., 0.],
);

pub struct CameraUpdate {
    pub camera_position: f32x4,
    pub matrix_screen_to_world: f32x4x4,
    pub matrix_world_to_projection: f32x4x4,
}

pub struct Camera {
    ray: UIRay,
    double_inv_screen_size: f32x2,
}

impl Camera {
    #[inline(always)]
    pub const fn new(init_rotation: f32x2, on_mouse_drag_modifier_keys: ModifierKeys) -> Self {
        Self {
            ray: UIRay {
                distance_from_origin: INITIAL_CAMERA_DISTANCE,
                rotation_xy: init_rotation,
                on_mouse_drag_modifier_keys,
                invert_drag: false,
            },
            double_inv_screen_size: f32x2::splat(1.),
        }
    }

    #[inline]
    pub fn on_event(&mut self, event: UserEvent, on_update: impl FnMut(CameraUpdate)) {
        let ray_update = self.ray.on_event(event);
        let screen_update = match event {
            UserEvent::WindowResize { size, .. } => {
                self.double_inv_screen_size = f32x2::splat(2.) / size;
                true
            }
            _ => false,
        };
        if ray_update || screen_update {
            self.handle_update(on_update);
        }
    }

    fn handle_update(&self, mut on_update: impl FnMut(CameraUpdate)) {
        let &[rotx, roty] = self.ray.rotation_xy.neg().as_array();
        let matrix_world_to_camera = f32x4x4::translate(0., 0., self.ray.distance_from_origin)
            * f32x4x4::rotate(rotx, roty, 0.);
        let camera_position =
            matrix_world_to_camera.inverse() * f32x4::from_array([0., 0., 0., 1.]);

        let aspect_ratio = self.double_inv_screen_size[0] / self.double_inv_screen_size[1];
        let matrix_world_to_projection =
            self.calc_matrix_camera_to_projection(aspect_ratio) * matrix_world_to_camera;

        let matrix_world_to_projection = matrix_world_to_projection;
        let matrix_screen_to_projection = f32x4x4::scale_translate(
            self.double_inv_screen_size[0],
            -self.double_inv_screen_size[1],
            1.,
            -1.,
            1.,
            0.,
        );
        let matrix_screen_to_world =
            matrix_world_to_projection.inverse() * matrix_screen_to_projection;

        on_update(CameraUpdate {
            camera_position,
            matrix_screen_to_world,
            matrix_world_to_projection,
        });
    }

    #[inline]
    fn calc_matrix_camera_to_projection(&self, aspect_ratio: f32) -> f32x4x4 {
        let w = NEAR_FIELD_MAJOR_AXIS;
        let h = aspect_ratio * NEAR_FIELD_MAJOR_AXIS;
        let orthographic_matrix = {
            f32x4x4::new(
                [2. / w, 0., 0., 0.],
                [0., 2. / h, 0., 0.],
                // IMPORTANT: Metal's NDC coordinate space has a z range of [0.,1], **NOT [-1,1]** (OpenGL).
                [0., 0., 1. / (F - N), -N / (F - N)],
                [0., 0., 0., 1.],
            )
        };
        orthographic_matrix * PERSPECTIVE_MATRIX
    }
}
