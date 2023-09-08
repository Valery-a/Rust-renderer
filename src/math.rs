use std;
use noise::{NoiseFn, Perlin};
use alga::general::{Real, Identity, Additive};
use matrix::*;
use std::fmt::Debug;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Triangle2<T : Value>{
    pub p1: Vec2<T>,
    pub p2: Vec2<T>,
    pub p3: Vec2<T>,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Triangle3<T : Value>{
    pub p1: Vec3<T>,
    pub p2: Vec3<T>,
    pub p3: Vec3<T>,
}

impl<T : Value + Identity<Additive>> Triangle3<T>{

}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Line2<T : Value> {
    pub start : Vec2<T>,
    pub end : Vec2<T>,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Line3<T : Value> {
    pub start : Vec3<T>,
    pub end : Vec3<T>,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Plane<T : Value> {
    pub point : Vec3<T>,
    pub normal : Vec3<T>,
}

//axis aligned
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Square2<T : Value>{
    pub center : Vec2<T>,
    pub extent : T,
}

//axis aligned
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Cube<T : Value>{
    pub center : Vec3<T>,
    pub extent : T,
}

impl<T : Real> Cube<T>{
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Sphere<T : Value>{
    pub center : Vec3<T>,
    pub rad : T,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Ray<T : Value>{
    pub start : Vec3<T>,
    pub dir : Vec3<T>
}

pub trait DenFn2<T : Value> : Fn(Vec2<T>) -> T + Copy{}
pub trait DenFn3<T : Value> : Fn(Vec3<T>) -> T + Copy{}


impl<T : Value, F : Fn(Vec2<T>) -> T + Copy> DenFn2<T> for F{}
impl<T : Value, F : Fn(Vec3<T>) -> T + Copy> DenFn3<T> for F{}

pub fn intersection3<T : Real>(a : impl DenFn3<T>, b : impl DenFn3<T>) -> impl DenFn3<T>{
    move |x|{Real::max(a(x), b(x))}
}


pub fn union3<T : Real>(a : impl DenFn3<T>, b : impl DenFn3<T>) -> impl DenFn3<T>{
    move |x|{Real::min(a(x), b(x))}
}


pub fn difference3<T : Real>(a : impl DenFn3<T>, b : impl DenFn3<T>) -> impl DenFn3<T>{
    move |x|{Real::max(a(x), -b(x))}
}

//0 to 1.0
pub fn octave_perlin2(perlin : &Perlin, x : f32, z : f32, octaves : usize, persistence : f32) -> f32{
    let mut total = 0.0;
    let mut frequency = 1.0;
    let mut amplitude = 1.0;
    let mut max_value = 0.0;

    let k = 2.0.powi((octaves - 1) as i32);

    for _i in 0..octaves{
        total += (perlin.get([(x * frequency / k) as f64, (z * frequency / k) as f64]) + 1.0)/2.0 * amplitude as f64;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= 2.0;
    }

    total as f32 / max_value
}

pub fn noise_f32(perlin : Perlin, cube : Cube<f32>) -> impl DenFn3<f32>{
     move |x| {
        if point3_inside_cube_inclusive(x, cube){
            let den = -octave_perlin2(&perlin, x.x - (cube.center.x - cube.extent), x.z - (cube.center.z - cube.extent), 4, 0.56) * 2.0 * cube.extent;
            let dy = x.y - (cube.center.y - cube.extent); //cube.extent / 2.0 ; // 0 - 1
            //println!("{} {} {}", den, dy, x.y);
            den + dy
        }else{
            0.01
        }
        
    }
}

pub fn mk_half_space_x_neg<T : Real>(x : T) -> impl DenFn3<T>{
    move |p : Vec3<T>|{p.x - x}
}

pub fn mk_half_space_x_pos<T : Real>(x : T) -> impl DenFn3<T>{
    move |p : Vec3<T>|{x - p.x}
}

pub fn mk_half_space_y_neg<T : Real>(y : T) -> impl DenFn3<T>{
    move |p : Vec3<T>|{p.y - y}
}

pub fn mk_half_space_y_pos<T : Real>(y : T) -> impl DenFn3<T>{
    move |p : Vec3<T>|{y - p.y}
}

pub fn mk_half_space_z_neg<T : Real>(z : T) -> impl DenFn3<T>{
    move |p : Vec3<T>|{p.z - z}
}

pub fn mk_half_space_z_pos<T : Real>(z : T) -> impl DenFn3<T>{
    move |p : Vec3<T>|{z - p.z}
}

pub fn mk_aabb<T : Real + Copy>(center : Vec3<T>, extent : Vec3<T>) -> impl DenFn3<T> {
    let x_neg = mk_half_space_x_neg(center.x + extent.x);
    let x_pos = mk_half_space_x_pos(center.x - extent.x);

    let y_neg = mk_half_space_y_neg(center.y + extent.y);
    let y_pos = mk_half_space_y_pos(center.y - extent.y);

    let z_neg = mk_half_space_z_neg(center.z + extent.z);
    let z_pos = mk_half_space_z_pos(center.z - extent.z);

    let ix = intersection3(x_neg, x_pos);
    let iy = intersection3(y_neg, y_pos);
    let iz = intersection3(z_neg, z_pos);

    let ixy = intersection3(ix, iy);

    intersection3(ixy, iz)
}

pub fn mk_half_space_pos<T : Real>(plane : Plane<T>) -> impl DenFn3<T>{
     move |p|{
        let d = p - plane.point;
        let dist = dot(d,plane.normal);
        -dist 
     }
}

pub fn mk_half_space_neg<T : Real>(plane : Plane<T>) -> impl DenFn3<T>{
     move |p|{
        let d = p - plane.point;
        let dist = dot(d, plane.normal);
        dist 
     }
}

pub fn mk_obb<T : Real>(center : Vec3<T>, right : Vec3<T>, up : Vec3<T>, extent : Vec3<T>) -> impl DenFn3<T> {
    let r_neg = mk_half_space_neg(Plane{point : center + right * extent.x, normal : right});
    let r_pos = mk_half_space_pos(Plane{point : center - right * extent.x, normal : right});

    let u_neg = mk_half_space_neg(Plane{point : center + up * extent.y, normal : up});
    let u_pos = mk_half_space_pos(Plane{point : center - up * extent.y, normal : up});

    let look = cross(right,up);

    let l_neg = mk_half_space_neg(Plane{point : center + look * extent.z, normal : look});
    let l_pos = mk_half_space_pos(Plane{point : center - look * extent.z, normal : look});

    let ix = intersection3(r_neg, r_pos);
    let iy = intersection3(u_neg, u_pos);
    let iz = intersection3(l_neg, l_pos);

    let ixy = intersection3(ix, iy);

    intersection3(ixy, iz)
}

pub fn mk_sphere<T : Real>(sphere : Sphere<T>) -> impl DenFn3<T>{
    move |x : Vec3<T>|{
        let dist = x - sphere.center;
        dot(dist,dist) - sphere.rad * sphere.rad
    }
}

pub fn mk_torus_z<T : Real>(r_big : T, r : T, offset : Vec3<T>) -> impl DenFn3<T>{
    move |p : Vec3<T>|{
        let x = p - offset;
        let a = (x.x * x.x + x.y * x.y).sqrt() - r_big;
        a * a + x.z * x.z - r * r
    }
}

pub fn mk_torus_y<T : Real>(r_big : T, r : T, offset : Vec3<T>) -> impl DenFn3<T>{
    move |p : Vec3<T>|{
        let x = p - offset;
        let a = (x.x * x.x + x.z * x.z).sqrt() - r_big;
        a * a + x.y * x.y - r * r
    }
}

pub fn point3_inside_cube_inclusive<T : Real>(point3 : Vec3<T>, square3 : Cube<T>) -> bool{
    point3.x <= square3.center.x + square3.extent &&
    point3.x >= square3.center.x - square3.extent &&

    point3.y <= square3.center.y + square3.extent &&
    point3.y >= square3.center.y - square3.extent &&

    point3.z <= square3.center.z + square3.extent &&
    point3.z >= square3.center.z - square3.extent
}

pub fn rot_mat3<T : Real>(u : Vec3<T>, rad : T) -> Mat3<T>{
    let c = T::cos(rad);
    let s = T::sin(rad);
    let one = T::one();
    Mat3::new(
        c + u.x*u.x*(one - c), u.x*u.y*(one - c) - u.z*s, u.x*u.z*(one - c) + u.y*s,
        u.y*u.x*(one - c) + u.z*s, c + u.y*u.y*(one - c), u.y*u.z*(one - c) - u.x*s,
        u.z*u.x*(one - c) - u.y*s - u.y*s, u.z*u.y*(one - c) + u.x*s, c + u.z*u.z*(one - c)
    )
}

//column-major
pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4<f32>{
    Mat4::new(2.0 / (right - left), 0.0, 0.0, -(right + left) / (right - left),
     0.0, 2.0 / (top - bottom), 0.0, -(top + bottom) / (top - bottom),
     0.0, 0.0, -2.0 / (far - near), -(far + near) / (far - near),
     0.0, 0.0, 0.0, 1.0
    )
}

//row-major
pub fn perspective(fovy : f32, aspect : f32, near : f32, far : f32) -> Mat4<f32>{
    let top = near * (std::f32::consts::PI / 180.0 * fovy / 2.0).tan();
    let bottom = -top;
    let right = top * aspect;
    let left = -right;
    Mat4::new(2.0 * near / (right - left), 0.0, (right + left) / (right - left), 0.0,
                 0.0, 2.0 * near / (top - bottom), (top + bottom) / (top - bottom), 0.0,
                 0.0, 0.0, -(far + near) / (far - near), -2.0 * far * near / (far - near),
                 0.0, 0.0, -1.0, 0.0)
}

//column-major
pub fn view_dir(pos : Vec3<f32>, look : Vec3<f32>, up : Vec3<f32>) -> Mat4<f32>{
    let za = -look;
    let xa = cross(up, za);
    let ya = cross(za, xa);

    Mat4::new(xa.x, ya.x, za.x, 0.0,
                 xa.y, ya.y, za.y, 0.0,
                 xa.z, ya.z, za.z, 0.0,
                 -dot(xa,pos), -dot(ya,pos), -dot(za,pos), 1.0).transpose()
}