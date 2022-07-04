use metal_types::f32x4x4;

use super::ui_ray::UIRay;
use crate::{ModifierKeys, UserEvent};
use std::{
    ops::Neg,
    simd::{f32x2, f32x4},
};

const INITIAL_CAMERA_DISTANCE: f32 = 1.0;
const N: f32 = 0.1;
const F: f32 = 100000.0;
const Z_RANGE: f32 = F - N;

pub struct CameraUpdate {
    pub position_world: f32x4,
    pub matrix_screen_to_world: f32x4x4,
    pub matrix_camera_to_world: f32x4x4,
    pub matrix_world_to_projection: f32x4x4,
}

pub struct Camera<const DRAG_SCALE: usize = 250> {
    pub ray: UIRay<DRAG_SCALE>,
    pub screen_size: f32x2,
}

impl<const DRAG_SCALE: usize> Camera<DRAG_SCALE> {
    #[inline(always)]
    pub const fn new(
        init_distance: f32,
        init_rotation: f32x2,
        on_mouse_drag_modifier_keys: ModifierKeys,
        invert_drag: bool,
        min_distance: f32,
    ) -> Self {
        Self {
            ray: UIRay::<DRAG_SCALE> {
                distance_from_origin: init_distance,
                rotation_xy: init_rotation,
                on_mouse_drag_modifier_keys,
                invert_drag,
                min_distance,
            },
            screen_size: f32x2::splat(1.),
        }
    }

    #[inline(always)]
    pub const fn new_with_default_distance(
        init_rotation: f32x2,
        on_mouse_drag_modifier_keys: ModifierKeys,
        invert_drag: bool,
        min_distance: f32,
    ) -> Self {
        Self::new(
            INITIAL_CAMERA_DISTANCE,
            init_rotation,
            on_mouse_drag_modifier_keys,
            invert_drag,
            min_distance,
        )
    }

    #[inline]
    pub fn on_event(&mut self, event: UserEvent) -> Option<CameraUpdate> {
        let ray_update = self.ray.on_event(event);
        let screen_update = match event {
            UserEvent::WindowFocusedOrResized { size, .. } => {
                self.screen_size = size;
                true
            }
            _ => false,
        };
        if ray_update || screen_update {
            Some(self.create_update())
        } else {
            None
        }
    }

    fn create_update(&self) -> CameraUpdate {
        let &[rotx, roty] = self.ray.rotation_xy.neg().as_array();
        let matrix_world_to_camera = f32x4x4::translate(0., 0., self.ray.distance_from_origin)
            * f32x4x4::rotate(rotx, roty, 0.);
        let matrix_camera_to_world = matrix_world_to_camera.inverse();
        let position_world = matrix_camera_to_world * f32x4::from_array([0., 0., 0., 1.]);

        let aspect_ratio = self.screen_size[0] / self.screen_size[1];
        let matrix_world_to_projection =
            calc_matrix_camera_to_projection(aspect_ratio, 60_f32.to_radians())
                * matrix_world_to_camera;

        let matrix_world_to_projection = matrix_world_to_projection;
        let scale_xy = f32x2::splat(2.) / self.screen_size;
        let matrix_screen_to_projection =
            f32x4x4::scale_translate(scale_xy[0], -scale_xy[1], 1., -1., 1., 0.);
        let matrix_screen_to_world =
            matrix_world_to_projection.inverse() * matrix_screen_to_projection;

        CameraUpdate {
            position_world,
            matrix_camera_to_world,
            matrix_screen_to_world,
            matrix_world_to_projection,
        }
    }
}

/// Returns a transformation matrix for converting camera space to projection space that has a
/// **vertical** FOV of **60 degrees**, assumes the nearest visible Z coordinate is `N` and
/// farthest is `F`.
///
/// This matrix is derived from 2 other transformations: Orthographic and Perspective Projection.
/// See this [wonderful video explanation](https://www.youtube.com/watch?v=gQiD2Kd6xoE&t=2059s)
/// that this implementation is based on.
///
/// # Orthographic Transformation Matrix
///
/// Transform camera space coordinates into the Canonical View Volume coordinate space.
/// - Scale X coordinates between [-w/2, w/2] to [-1, 1]
/// - Scale Y coordinates between [-h/2, h/2] to [-1, 1]
/// - Translate and Scale Z coordinates between [ N,   F] to [ 0, 1]
///   - **IMPORTANT** Metal's NDC coordinate space has a Z range of [0, 1], **NOT [-1, 1]** (OpenGL).
///
/// ```ignore
/// let matrix_orthographic = f32x4x4::new(
///   [2. / w, 0.,     0.,           0.],
///   [0.,     2. / h, 0.,           0.],
///   [0.,     0.,     1. / (F - N), -N / (F - N)],
///   [0.,     0.,     0.,           1.],
/// );
/// ```
///
/// # Perspective Projection Transformation Matrix
///
/// - Scale X and Y based on Z (make stuff far away smaller)
///
/// ```ignore
/// let matrix_perspective = = f32x4x4::new(
///   [N,  0., 0.,     0.],
///   [0., N,  0.,     0.],
///   [0., 0., N + F, -N * F],
///   [0., 0., 1.,     0.],
/// );
/// ```
///
/// # Overall Result
///
/// ```ignore
/// matrix_orthographic * matrix_perspective;
/// ```
#[inline]
pub fn calc_matrix_camera_to_projection(aspect_ratio: f32, fov: f32) -> f32x4x4 {
    let sy = 1. / (fov / 2.).tan();
    let sx = sy / aspect_ratio;
    f32x4x4::new(
        [sx, 0., 0., 0.],
        [0., sy, 0., 0.],
        [0., 0., F / Z_RANGE, -N * F / Z_RANGE],
        [0., 0., 1., 0.],
    )
}
