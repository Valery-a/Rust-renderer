use crate::vector::Vector;
use crate::matrix::Matrix;


pub struct Camera {
    position: Vector,
    target: Vector,
    up: Vector,
}

impl Camera {
    pub fn new(position: Vector, target: Vector, up: Vector) -> Camera {
        Camera {
            position,
            target,
            up,
        }
    }

    pub fn move_forward(&mut self, distance: f32) {
        let direction = self.target.min(&self.position).normalize();
        let displacement = direction.scale(distance);
        self.position = self.position.add(&displacement);
        self.target = self.target.add(&displacement);
    }

    pub fn move_backward(&mut self, distance: f32) {
        let direction = self.target.min(&self.position).normalize();
        let displacement = direction.scale(-distance);
        self.position = self.position.add(&displacement);
        self.target = self.target.add(&displacement);
    }

    pub fn move_left(&mut self, distance: f32) {
        let right = self.target.cross(&self.up).normalize();
        let displacement = right.scale(-distance);
        self.position = self.position.add(&displacement);
        self.target = self.target.add(&displacement);
    }

    pub fn move_right(&mut self, distance: f32) {
        let right = self.target.cross(&self.up).normalize();
        let displacement = right.scale(distance);
        self.position = self.position.add(&displacement);
        self.target = self.target.add(&displacement);
    }

    pub fn view_matrix(&self) -> Matrix {
        Matrix::look_at(&self.position, &self.target, &self.up)
    }
}