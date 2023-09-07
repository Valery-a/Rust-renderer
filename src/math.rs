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

