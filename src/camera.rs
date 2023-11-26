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

        // Projection and view matrices initialization
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
        // Combine rotation and translation to form the view matrix
        self.view = Mat4::rotate(self.rotation) * Mat4::translate(-self.position);
    }

    // Move the camera forward along its front vector
    pub fn move_forward(&mut self, distance: f32) {
        let front = self.get_front();
        self.position += front * distance;
        self.recalculate_view();
    }

    // Move the camera backward along its front vector
    pub fn move_backward(&mut self, distance: f32) {
        let front = self.get_front();
        self.position -= front * distance;
        self.recalculate_view();
    }

    // Move the camera left along its right vector
    pub fn move_left(&mut self, distance: f32) {
        let right = self.get_right();
        self.position -= right * distance;
        self.recalculate_view();
    }

    // Move the camera right along its right vector
    pub fn move_right(&mut self, distance: f32) {
        let right = self.get_right();
        self.position += right * distance;
        self.recalculate_view();
    }

    // Ascend (move upwards) the camera
    pub fn ascend(&mut self, distance: f32) {
        self.position.y += distance;
        self.recalculate_view();
    }

    // Descend (move downwards) the camera
    pub fn descend(&mut self, distance: f32) {
        self.position.y -= distance;
        self.recalculate_view();
    }

    // getters and setters for various camera properties

    // Getter for camera position
    pub fn get_position(&self) -> Vec3 {
        self.position
    }

    // Setter for camera position
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.recalculate_view();
    }

    // Deprecated function to set the position based on a player's position
    pub fn set_position_from_player_position(&mut self, player_position: Vec3) {
        // Adjust camera position based on player position and eye height
        self.position = player_position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
        self.recalculate_view();
    }

    // Getter for camera rotation
    pub fn get_rotation(&self) -> Quaternion {
        self.rotation
    }

    // Setter for camera rotation
    pub fn set_rotation(&mut self, rotation: Quaternion) {
        self.rotation = rotation;
        self.recalculate_view();
    }

    // Getter for camera projection matrix
    pub fn get_projection(&self) -> Mat4 {
        self.projection
    }

    // Getter for camera view matrix
    pub fn get_view(&self) -> Mat4 {
        self.view
    }

    // Getter for camera field of view
    pub fn get_fov(&self) -> f32 {
        self.fov
    }

    // Setter for camera field of view
    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
        self.recalculate_projection();
    }
}
