#[derive(Debug, PartialEq)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32
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