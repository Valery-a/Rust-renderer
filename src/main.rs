extern crate sdl2;
use std::any::Any;
use std::io;
use sdl2::pixels::{Color};
use sdl2::event::Event;
use sdl2::rect::Point;
use std::cmp;
use core::f32::{consts::PI, INFINITY};
const RESOLUTION: (u32, u32) = (800, 600);

#[derive(Debug, PartialEq)]
struct Face {
    a: Point,
    b: Point,
    c: Point
}

fn line_intersection(y: i32, p0: &Point, p1: &Point) -> Option<i32> {
    if (p0.y > y && p1.y > y) || (p0.y < y && p1.y < y) { return None }
    let p0x = p0.x as f32;
    let p1x = p1.x as f32;
    let p2x = RESOLUTION.0 as f32;
    let p3x = RESOLUTION.1 as f32;
    let p0y = p0.y as f32;
    let p1y = p1.y as f32;
    let p2y = y as f32;
    let p3y = y as f32;
    let t: f32 = 
        ((p0x-p2x)*(p2y-p3y)-(p0y-p2y)*(p2x-p3x)) /
        ((p0x-p1x)*(p2y-p3y)-(p0y-p1y)*(p2x-p3x));
    let x = (p0x + t*(p1x-p0x)).ceil();

    if x.is_normal() {
        Some(x as i32)
    } else {
        None
    }
}

impl Face {
    fn new(a: Point, b: Point, c: Point) -> Face {
        Face { a, b, c }
    }

    fn orientation(&self) -> bool {
        let e0 = (self.b.x-self.a.x)*(self.b.y+self.a.y);
        let e1 = (self.c.x-self.b.x)*(self.c.y+self.b.y);
        let e2 = (self.a.x-self.c.x)*(self.a.y+self.c.y);
        e0+e1+e2 < 0
    }

    fn row_intersects(&self, y: i32) -> Option<(i32, i32)> {
        let (i0, i1, i2) = (
            line_intersection(y, &self.a, &self.b), 
            line_intersection(y, &self.b, &self.c), 
            line_intersection(y, &self.c, &self.a)
        );
    
        match (i0, i1, i2) {
            (Some(x0), Some(x1), None) => Some((cmp::min(x0, x1), cmp::max(x0, x1))),
            (Some(x0), None, Some(x1)) => Some((cmp::min(x0, x1), cmp::max(x0, x1))),
            (None, Some(x0), Some(x1)) => Some((cmp::min(x0, x1), cmp::max(x0, x1))),
            (Some(x0), Some(x1), Some(x2)) => Some((cmp::min(x0, x1), cmp::max(cmp::max(x1, x2), x0))),
            _ => None
        }
    }
    
    fn height_range(&self) -> (i32, i32) {
        (
            cmp::min(self.a.y, cmp::min(self.b.y, self.c.y)),
            cmp::max(self.a.y, cmp::max(self.b.y, self.c.y))
        )
    }

    fn barycentric(&self, p: &Point) -> (f32, f32, f32) {
        let vx0 = (self.b.x - self.a.x) as f32;
        let vy0 = (self.b.y - self.a.y) as f32;
        let vx1 = (self.c.x - self.a.x) as f32;
        let vy1 = (self.c.y - self.a.y) as f32;
        let vx2 = (     p.x - self.a.x) as f32;
        let vy2 = (     p.y - self.a.y) as f32;
        let den = vx0 * vy1 - vx1 * vy0;
        let v = (vx2 * vy1 - vx1 * vy2) / den;
        let w = (vx0 * vy2 - vx2 * vy0) / den;
        let u = 1. - v - w;
        (u, v, w)
    }
}

#[derive(Debug, PartialEq)]
pub struct Vector {
    x: f32,
    y: f32,
    z: f32
}

impl Vector {
    pub fn new() -> Vector {
        Vector {
            x: 0., y: 0., z: 0.
        }
    }

    pub fn from_xyz(x: f32, y: f32, z: f32) -> Vector {
        Vector {
            x, y, z
        }
    }

    pub fn dot (&self, other: &Vector) -> f32 {
        (self.x * other.x + self.y * other.y + self.z * other.z).sqrt()
    }

    pub fn cross(&self, other: &Vector) -> Vector {
        Vector {
            x: self.y*other.z - self.z*other.y,
            y: self.z*other.x - self.x*other.z,
            z: self.x*other.y - self.y*other.x 
        }
    }

    pub fn neg(&self) -> Vector {
        Vector {
            x: -self.x,
            y: -self.y,
            z: -self.z 
        }
    }

    pub fn add(&self, other: &Vector) -> Vector {
        Vector {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z
        }
    }

    pub fn min(&self, other: &Vector) -> Vector {
        Vector {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z
        }
    }

    pub fn scale(&self, scalar: f32) -> Vector {
        Vector {
            x: self.x*scalar,
            y: self.y*scalar,
            z: self.z*scalar
        }
    }

    pub fn normalize(&self) -> Vector {
        let l = (self.x*self.x + self.y*self.y + self.z*self.z).sqrt();
        let x = self.x / l;
        let y = self.y / l;
        let z = self.z / l;
        Vector {
            x: if (x+y).is_normal() { x + 0.001 } else { x },
            y: if (y+z).is_normal() { y + 0.001 } else { y },
            z: if (z+x).is_normal() { z + 0.001 } else { z }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Matrix {
    indices: [f32; 16]
}

fn det2(m: [f32; 4]) -> f32 {
    (m[0]*m[3]) + (m[1]*m[2])
}

fn det3(m: [f32; 9]) -> f32 {
    let a = m[0];
    let b = m[1];
    let c = m[2];
    a * det2([m[4], m[5], m[7], m[8]]) -
    b * det2([m[3], m[5], m[6], m[8]]) +
    c * det2([m[3], m[4], m[6], m[7]])
}

fn det4(m: [f32; 16]) -> f32 {
    let a = m[0];
    let b = m[1];
    let c = m[2];
    let d = m[3];
    a * det3([m[ 5], m[ 6], m[ 7], m[ 9], m[10], m[11], m[13], m[14], m[15]]) -
    b * det3([m[ 4], m[ 6], m[ 7], m[ 8], m[10], m[11], m[12], m[10], m[11]]) +
    c * det3([m[ 4], m[ 5], m[ 7], m[ 8], m[ 9], m[11], m[12], m[13], m[15]]) -
    d * det3([m[ 4], m[ 5], m[ 6], m[ 8], m[ 9], m[10], m[12], m[13], m[14]])
}

impl Matrix {
    pub fn identity() -> Matrix {
        Matrix {
            indices: [
                1., 0., 0., 0.,
                0., 1., 0., 0.,
                0., 0., 1., 0.,
                0., 0., 0., 1.
                ]
        }
    }

    pub fn zero() -> Matrix {
        Matrix { indices: [0_f32; 16] }
    }

    pub fn look_at(eye: &Vector, target: &Vector, up: &Vector) -> Matrix {
        let z = eye.min(&target).normalize();
        let x = up.cross(&z).normalize();
        let y = z.cross(&x);
        let xw = -x.dot(&eye);
        let yw = -y.dot(&eye);
        let zw = -z.dot(&eye);
        Matrix {
            indices: [
                x.x, y.x, z.x, 0.,
                x.y, y.y, z.y, 0.,
                x.z, y.z, z.z, 0.,
                xw , yw , zw , 1.
                ]
        }
    }

   pub fn frustum(&self, r: f32, l: f32, t: f32, b: f32, n: f32, f: f32) -> Matrix {
        let x = ( 2_f32*n)   / (r-l);
        let y = ( 2_f32*n)   / (t-b);
        let z = (-2_f32*f*n) / (f-n);
        let a =  (r+l) / (r-l);
        let b =  (t+b) / (t-b);
        let c = -(f+n) / (f-n);
        let d = -1_f32;
        Matrix {
            indices: [
                x , 0., a , 0.,
                0., y , b , 0.,
                0., 0., c , z ,
                0., 0., d , 0.
            ]
        }
    }
         
    pub fn perspective(aspect: f32, y_fov: f32, z_near: f32, z_far: f32) -> Matrix {
        let f = 1_f32 / (y_fov / 2_f32).tan();
        let a = f / aspect;
        let range = 1_f32 / (z_near - z_far);
        let finite = z_far.is_finite();
        let b = if finite { (z_near + z_far) * range } else { -1. };
        let c = if finite { 2. * z_near * z_far * range } else { -2. * z_near };
        Matrix {
         indices: [
                a , 0., 0. , 0.,
                0., f , -1., 0.,
                0., 0., b  , -1.,
                0., 0., c , 0.,
            ]
        }
    }

    pub fn dot(&self, other: &Matrix) -> Matrix {
        let (m1, m2) = (&other.indices, &self.indices);

        let a = m1[ 0]*m2[ 0] + m1[ 1]*m2[ 4] + m1[ 2]*m2[ 8] + m1[ 3]*m2[12];
        let e = m1[ 4]*m2[ 0] + m1[ 5]*m2[ 4] + m1[ 6]*m2[ 8] + m1[ 7]*m2[12];
        let i = m1[ 8]*m2[ 0] + m1[ 9]*m2[ 4] + m1[10]*m2[ 8] + m1[11]*m2[12];
        let m = m1[12]*m2[ 0] + m1[13]*m2[ 4] + m1[14]*m2[ 8] + m1[15]*m2[12];

        let b = m1[ 0]*m2[ 1] + m1[ 1]*m2[ 5] + m1[ 2]*m2[ 9] + m1[ 3]*m2[13];
        let f = m1[ 4]*m2[ 1] + m1[ 5]*m2[ 5] + m1[ 6]*m2[ 9] + m1[ 7]*m2[13];
        let j = m1[ 8]*m2[ 1] + m1[ 9]*m2[ 5] + m1[10]*m2[ 9] + m1[11]*m2[13];
        let n = m1[12]*m2[ 1] + m1[13]*m2[ 5] + m1[14]*m2[ 9] + m1[15]*m2[13];

        let c = m1[ 0]*m2[ 2] + m1[ 1]*m2[ 6] + m1[ 2]*m2[10] + m1[ 3]*m2[14];
        let g = m1[ 4]*m2[ 2] + m1[ 5]*m2[ 6] + m1[ 6]*m2[10] + m1[ 7]*m2[14];
        let k = m1[ 8]*m2[ 2] + m1[ 9]*m2[ 6] + m1[10]*m2[10] + m1[11]*m2[14];
        let o = m1[12]*m2[ 2] + m1[13]*m2[ 6] + m1[14]*m2[10] + m1[15]*m2[14];

        let d = m1[ 0]*m2[ 3] + m1[ 1]*m2[ 7] + m1[ 2]*m2[11] + m1[ 3]*m2[15];
        let h = m1[ 4]*m2[ 3] + m1[ 5]*m2[ 7] + m1[ 6]*m2[11] + m1[ 7]*m2[15];
        let l = m1[ 8]*m2[ 3] + m1[ 9]*m2[ 7] + m1[10]*m2[11] + m1[11]*m2[15];
        let p = m1[12]*m2[ 3] + m1[13]*m2[ 7] + m1[14]*m2[11] + m1[15]*m2[15];

        Matrix {
            indices: [
                a, b, c, d,
                e, f, g, h,
                i, j, k, l,
                m, n, o, p
            ]
        }
    }

    pub fn mul(&self, other: &Matrix) -> Matrix {
        let (m1, m2) = (&self.indices, &other.indices);

        let a = m1[ 0]*m2[ 0];
        let e = m1[ 1]*m2[ 4];
        let i = m1[ 2]*m2[ 8];
        let m = m1[ 3]*m2[12];

        let b = m1[ 4]*m2[ 1];
        let f = m1[ 5]*m2[ 5];
        let j = m1[ 6]*m2[ 9];
        let n = m1[ 7]*m2[13];

        let c = m1[ 8]*m2[ 2];
        let g = m1[ 9]*m2[ 6];
        let k = m1[10]*m2[10];
        let o = m1[11]*m2[14];

        let d = m1[12]*m2[ 3];
        let h = m1[13]*m2[ 7];
        let l = m1[14]*m2[11];
        let p = m1[15]*m2[15];

        Matrix {
            indices: [
                a, b, c, d,
                e, f, g, h,
                i, j, k, l,
                m, n, o, p
            ]
        }
    }

    pub fn project(&self, v: &Vector) -> Vector {
        let m = self.indices;

        let x = m[ 0]*v.x + m[ 4]*v.y + m[ 8]*v.z + m[12];
        let y = m[ 1]*v.x + m[ 5]*v.y + m[ 9]*v.z + m[13];
        let z = m[ 2]*v.x + m[ 6]*v.y + m[10]*v.z + m[14];
        let w = m[ 3]*v.x + m[ 7]*v.y + m[11]*v.z + m[15];

        Vector {
            x: x/w, y: y/w, z: z/w
        }
    }

    pub fn scale(&self, scalar: f32) -> Matrix {
        let m = self.indices;
        Matrix {
            indices: m.map(|x| x * scalar)
        }
    }

    pub fn add(&self, other: &Matrix) -> Matrix {
        let m1 = self.indices;
        let m2 = other.indices;
        Matrix {
            indices: [
                m1[ 0]+m2[ 0], m1[ 1]+m2[ 1], m1[ 2]+m2[ 2], m1[ 3]+m2[ 3], 
                m1[ 4]+m2[ 4], m1[ 5]+m2[ 5], m1[ 6]+m2[ 6], m1[ 7]+m2[ 7], 
                m1[ 8]+m2[ 8], m1[ 9]+m2[ 9], m1[10]+m2[10], m1[11]+m2[11], 
                m1[12]+m2[12], m1[13]+m2[13], m1[14]+m2[14], m1[15]+m2[15] 
            ]
        }
    }
    
    pub fn det(&self) -> f32 {
        det4(self.indices)
    }
}

fn screen(v: &Vector) -> Point {
    let x = (v.x+1.) / 2. * (RESOLUTION.0 as f32);
    let y = (v.y+1.) / 2. * (RESOLUTION.1 as f32);
    Point::new(x as i32, y as i32)
}
mod objects;
use crate::objects::cube::Object1;
use crate::objects::pyramid::Object2;

fn main() {
    let mut input = String::new();
    println!("Please enter the object you want to render:");
    io::stdin().read_line(&mut input).expect("Failed to read input.");
    let object = match input.trim() {
        "cube" => Box::new(Object1::cube()) as Box<dyn Any>,
        "pyramid" => Box::new(Object2::pyramid()) as Box<dyn Any>,
        _ => {
            eprintln!("Invalid input. Please enter a valid object");
            return;
        }
    };
    
    let mut input = String::new();
    println!("Please enter the number of objects you want to generate:");
    io::stdin().read_line(&mut input).expect("Failed to read input.");
    let num_objects = match input.trim().parse::<usize>() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("Invalid input. Please enter a valid number");
            return;
        }
    };

    let face = Face::new(Point::new(10, 2), Point::new(200, 200), Point::new(4, 200));
    dbg!(face.barycentric(&Point::new(50, 50)));
    let sdl = sdl2::init().unwrap();
    let video = sdl.video().unwrap();
    let window = video.window("renderer", RESOLUTION.0, RESOLUTION.1)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    let mut event_pump = sdl.event_pump().unwrap();
    let mut i = 0_f32;
    'running: loop {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.set_draw_color(Color::RGB(244, 0, 0));
        i = i+0.1;
        let eye    = Vector::from_xyz(i, -6.,  0.);
        let target = Vector::from_xyz(0_f32, 0.,  0.);
        let up     = Vector::from_xyz(0_f32, 0., -1.);
        let view   = Matrix::look_at(&eye, &target, &up);
        let proj   = Matrix::perspective(1., PI/2., 5., INFINITY);
        let view_proj = &proj.dot(&view);
        for _ in 0..num_objects {
            object
                .downcast_ref::<Object1>()
                .map(|o| o.render(&mut canvas, &view_proj))
                .or_else(|| {
                    object
                        .downcast_ref::<Object2>()
                        .map(|o| o.render(&mut canvas, &view_proj))
                });
        }
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown {..} => { break 'running },
                _ => {}
            }
        }
        canvas.present();
    }
}
