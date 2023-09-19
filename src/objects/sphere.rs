use sdl2::rect::Point;
use sdl2::render::{Canvas, RenderTarget};
use crate::Face;
use crate::Matrix;
use crate::Vector;
use crate::Color;
use crate::screen;

pub struct Object3 {
    vectors: Vec<Vector>,
    colors: Vec<Color>,
    faces: Vec<(usize, usize, usize)>
}

impl Object3 {

    pub fn sphere() -> Object3 {
        let latitude_segments = 20;
        let longitude_segments = 20;
        let _radius = 1.0;

        let mut vectors = Vec::new();
        let mut colors = Vec::new();
        let mut faces = Vec::new();

        // Generate vertices
        for lat in 0..=latitude_segments {
            let theta = lat as f32 * std::f32::consts::PI / latitude_segments as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            for lon in 0..=longitude_segments {
                let phi = lon as f32 * 2.0 * std::f32::consts::PI / longitude_segments as f32;
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();

                let x = cos_phi * sin_theta;
                let y = cos_theta;
                let z = sin_phi * sin_theta;

                vectors.push(Vector::from_xyz(x, y, z));
                colors.push(Color::RGB(128, 0, 128));
            }
        }

        // Generate faces
        for lat in 0..latitude_segments {
            for lon in 0..longitude_segments {
                let first = lat * (longitude_segments + 1) + lon;
                let second = first + longitude_segments + 1;

                faces.push((first, first + 1, second));
                faces.push((second, first + 1, second + 1));
            }
        }

        Object3 {
            vectors,
            colors,
            faces,
        }
    }

    pub fn render<T: RenderTarget>(&self, canvas: &mut Canvas<T>, m: &Matrix) {
        let points: Vec<_> = self.vectors.iter().map(|v| m.project(&v)).collect();
        for &(i0, i1, i2) in &self.faces {
            let p0 = screen(&points[i0]);
            let p1 = screen(&points[i1]);
            let p2 = screen(&points[i2]);
            let c0 = self.colors[i0];
            let c1 = self.colors[i1];
            let c2 = self.colors[i2];
            let face = Face::new(p0, p1, p2);
            if face.orientation() {
                let (y_from, y_till) = face.height_range();
                for y in y_from..y_till {
                    let (x_from, x_till) = match face.row_intersects(y) {
                        Some(p) => p,
                        _ => continue,
                    };
                    for x in x_from..x_till {
                        let p = Point::new(x, y);
                        let (u, v, w) = face.barycentric(&p);
                        let r = (c0.r as f32 * u) + (c1.r as f32 * v) + (c2.r as f32 * w);
                        let g = (c0.g as f32 * u) + (c1.g as f32 * v) + (c2.g as f32 * w);
                        let b = (c0.b as f32 * u) + (c1.b as f32 * v) + (c2.b as f32 * w);
                        let c = Color::RGB(r as u8, g as u8, b as u8);
                        canvas.set_draw_color(c);
                        canvas.draw_point(p).unwrap();
                    }
                }
            }
        }
    }
}