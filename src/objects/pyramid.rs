use sdl2::render::{Canvas, RenderTarget};
use crate::Face;
use crate::Matrix;
use crate::Vector;
use crate::Color;
use crate::screen;
use sdl2::rect::Point;
pub struct Object2 {
    vectors: Vec<Vector>,
    colors: Vec<Color>,
    faces: Vec<(usize, usize, usize)>
}

impl Object2 {
    pub fn pyramid() -> Object2 {
        Object2 {
            vectors: vec![
                Vector::from_xyz(-1.0, -1.0, 0.0), // Bottom-left point
                Vector::from_xyz(-1.0, 1.0, 0.0),  // Top-left point
                Vector::from_xyz(0.0, 0.0, 1.0),   // Top point
                Vector::from_xyz(1.0, 1.0, 0.0),   // Top-right point
                Vector::from_xyz(1.0, -1.0, 0.0),  // Bottom-right point
            ],
            colors: vec![
                Color::RGB(255, 0, 0),   // Bottom-left point color
                Color::RGB(0, 255, 0),   // Top-left point color
                Color::RGB(0, 0, 255),   // Top point color
                Color::RGB(255, 255, 0), // Top-right point color
                Color::RGB(255, 255, 255), // Bottom-right point color
            ],
            faces: vec![
                // Left face
                (0, 1, 2),
                // Front face
                (1, 3, 2),
                // Right face
                (3, 4, 2),
                // Bottom face
                (4, 0, 2),
            ]
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
                        _ => continue
                    };
                    for x in x_from..x_till {
                        let p = Point::new(x, y);
                        let (u, v, w) = face.barycentric(&p);
                        let r = (c0.r as f32 * u) + (c1.r as f32* v) + (c2.r as f32 * w);
                        let g = (c0.g as f32 * u) + (c1.g as f32* v) + (c2.g as f32 * w);
                        let b = (c0.b as f32 * u) + (c1.b as f32* v) + (c2.b as f32 * w);
                        let c = Color::RGB(r as u8, g as u8, b as u8);
                        canvas.set_draw_color(c);
                        canvas.draw_point(p).unwrap();
                    }
                }
                // Draw a frame around the boject
                canvas.set_draw_color(Color::RGB(244, 0, 0));
                canvas.draw_line(p0, p1).unwrap();
                canvas.draw_line(p1, p2).unwrap();
                canvas.draw_line(p2, p0).unwrap();
            }
        }
    }
}
