use bytemuck::{Pod, Zeroable};
use cgmath::{
    InnerSpace, Matrix, Matrix3, Matrix4, PerspectiveFov, SquareMatrix, Vector2, Vector3,
};
use wgpu::{util::DeviceExt, BufferUsages};

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
/// Internal type representing the contents of the camera uniform.
struct CameraUniform {
    viewport: [f32; 16],
    transform: [f32; 16],
}
impl CameraUniform {
    pub fn new(viewport: PerspectiveFov<f64>, transform: Matrix4<f64>) -> Self {
        let viewport = {
            #[rustfmt::skip]
            const OPENGL_TO_WGPU_MATRIX: Matrix4<f64> = Matrix4::new(
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 0.5, 0.0,
                0.0, 0.0, 0.5, 1.0,
            );

            let viewport: Matrix4<f64> = viewport.into();
            let viewport = OPENGL_TO_WGPU_MATRIX * viewport;
            let viewport: Matrix4<f32> = viewport.cast().unwrap(); // f64 -> f32 cast, guaranteed not to fail
            *viewport.as_ref()
        };
        let transform = {
            let transform = transform.invert().unwrap();
            let transform: Matrix4<f32> = transform.cast().unwrap(); // f64 -> f32 cast, guaranteed not to fail
            *transform.as_ref()
        };
        Self {
            viewport,
            transform,
        }
    }
}