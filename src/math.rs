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