use std::ops::Deref;
use gfx_maths::*;
use crate::helpers;
use crate::helpers::gfx_maths_mat4_to_glmatrix_mat4;

// Height of the camera's viewpoint
pub const EYE_HEIGHT: f32 = 2.75;

#[derive(Clone)]
pub struct Camera {
    position: Vec3,
    rotation: Quaternion,
    projection: Mat4,
    view: Mat4,
    window_size: Vec2,
    fov: f32,
    near: f32,
    far: f32,
}

// degrees to radians convert
fn degrees_to_radians(degrees: f32) -> f32 {
    degrees * std::f32::consts::PI / 180.0
}

impl Camera {
    // Constructor function to create a new camera
    pub fn new(window_size: Vec2, fov: f32, near: f32, far: f32) -> Camera {
        let mut camera = Camera {
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(0.0, 0.0, 0.0, 1.0),
            projection: Mat4::identity(),
            view: Mat4::identity(),
            window_size,
            fov,
            near,
            far,
        };

        // Projection and view matrices initialize
        camera.recalculate_projection();
        camera.recalculate_view();
        camera
    }

    // Get the front vector of the camera (the third column in the matrix)
    pub fn get_front(&self) -> Vec3 {
        let mut front = Vec3::new(0.0, 0.0, 1.0);
        front = helpers::rotate_vector_by_quaternion(front, self.rotation);
        *front.normalize().deref()
    }

    // Get the forward vector of the camera with pitch (y-axis rotation) removed
    pub fn get_forward_no_pitch(&self) -> Vec3 {
        let mut front = Vec3::new(0.0, 0.0, 1.0);
        front = helpers::rotate_vector_by_quaternion(front, self.rotation);
        front.y = 0.0;
        *front.normalize().deref()
    }

    // Get the right vector of the camera (first column of the matrix)
    pub fn get_right(&self) -> Vec3 {
        let mut right = Vec3::new(-1.0, 0.0, 0.0);
        right = helpers::rotate_vector_by_quaternion(right, self.rotation);
        *right.normalize().deref()
    }

    // Get the up vector of the camera
    pub fn get_up(&self) -> Vec3 {
        // Cross product of right and forward vectors
        let right = self.get_right();
        let front = self.get_front();
        right.cross(front)
    }

    // Set the rotation to look at the specified target and recalculate the view matrix
    pub fn look_at(&mut self, target: Vec3) {
        // Calculate yaw and pitch angles based on the difference between camera position and target
        let diff = self.position - target;
        let yaw = f32::atan2(diff.x, diff.z);
        let pitch = f32::atan2(diff.y, diff.z);
        self.rotation = Quaternion::from_euler_angles_zyx(&Vec3::new(pitch, yaw, 0.0));
        self.recalculate_view();
    }

    // Recalculate the projection matrix based on the camera's perspective
    fn recalculate_projection(&mut self) {
        let aspect_ratio = self.window_size.x as f32 / self.window_size.y as f32;
        self.projection = Mat4::perspective_opengl(degrees_to_radians(self.fov), self.near, self.far, aspect_ratio);
    }

    // Recalculate the view matrix based on the camera's position and rotation
    fn recalculate_view(&mut self) {
        self.view = Mat4::rotate(self.rotation) * Mat4::translate(-self.position);
    }

    // gettersz and setters for various camera properties
    pub fn get_position(&self) -> Vec3 {
        self.position
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.recalculate_view();
    }

    // Deprecated function to set the position based on a player's position
    pub fn set_position_from_player_position(&mut self, player_position: Vec3) {
        self.position = player_position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
        self.recalculate_view();
    }

    pub fn get_rotation(&self) -> Quaternion {
        self.rotation
    }

    pub fn set_rotation(&mut self, rotation: Quaternion) {
        self.rotation = rotation;
        self.recalculate_view();
    }

    pub fn get_projection(&self) -> Mat4 {
        self.projection
    }

    pub fn get_view(&self) -> Mat4 {
        self.view
    }

    pub fn get_fov(&self) -> f32 {
        self.fov
    }

    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
        self.recalculate_projection();
    }
}
