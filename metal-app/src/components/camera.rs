use metal_types::f32x4x4;

use super::ui_ray::UIRay;
use crate::{ModifierKeys, UserEvent};
use std::{
    ops::Neg,
    simd::{f32x2, f32x4},
};

const INITIAL_CAMERA_DISTANCE: f32 = 2.;
const N: f32 = 0.1;
const F: f32 = 100000.0;
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
    focal_bounds: f32x4,
    ray: UIRay,
    double_inv_screen_size: f32x2,
}

impl Camera {
    #[inline(always)]
    pub const fn new(
        // Assuming the camera is pointed a model, the `focal_bounds` is the width, height, and
        // depth of the model. This is used to make sure the view volume is initially big enough to
        // contain the model.
        // TODO: Figure out better name, potentially means something completely different in
        // computer graphics.
        focal_bounds: f32x4,
        init_rotation: f32x2,
        on_mouse_drag_modifier_keys: ModifierKeys,
        invert_drag: bool,
    ) -> Self {
        Self {
            focal_bounds,
            ray: UIRay {
                distance_from_origin: INITIAL_CAMERA_DISTANCE,
                rotation_xy: init_rotation,
                on_mouse_drag_modifier_keys,
                invert_drag,
            },
            double_inv_screen_size: f32x2::splat(1.),
        }
    }

    #[inline]
    pub fn on_event(&mut self, event: UserEvent, on_update: impl FnMut(CameraUpdate)) {
        let ray_update = self.ray.on_event(event);
        let screen_update = match event {
            UserEvent::WindowFocusedOrResized { size, .. } => {
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

    // TODO: Is this really the best way to calculate FOV calculation
    // - Currently using the `focal_bounds` to determine the overall size of view volume and
    //   the `aspect_ratio` the view volume has the... right aspect ratio.
    // - The caller determines `focal_bounds` based on the loaded model's bounding box.
    // - Haven't worked out the math yet, but I can't imagine this maintains a realistic FOV
    // - Read up https://en.wikipedia.org/wiki/Field_of_view_in_video_games
    #[inline]
    fn calc_matrix_camera_to_projection(&self, aspect_ratio: f32) -> f32x4x4 {
        let [fw, _fh, fd, _] = self.focal_bounds.to_array();
        // Use the Width/Depth ratio to size the View Volume to contain the focal bounds.
        // Put another way, we want to see the whole model (size represented as the focal bounds).
        let ratio = (fw / 2.) / (INITIAL_CAMERA_DISTANCE - (fd / 2.0));
        let w = 2. * (ratio * N);
        let h = aspect_ratio * w;
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
