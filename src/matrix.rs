use crate::vector::Vector;

#[derive(Debug, PartialEq)]
pub struct Matrix {
    indices: [f32; 16],
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
         
    pub fn rotate_x(&self, angle: f32) -> Matrix {
        let sin_theta = angle.sin();
        let cos_theta = angle.cos();

        let m = self.indices;

        let a = m[0];
        let b = m[1];
        let c = m[2];
        let d = m[3];

        let e = m[4] * cos_theta - m[8] * sin_theta;
        let f = m[5] * cos_theta - m[9] * sin_theta;
        let g = m[6] * cos_theta - m[10] * sin_theta;
        let h = m[7] * cos_theta - m[11] * sin_theta;

        let i = m[8] * cos_theta + m[4] * sin_theta;
        let j = m[9] * cos_theta + m[5] * sin_theta;
        let k = m[10] * cos_theta + m[6] * sin_theta;
        let l = m[11] * cos_theta + m[7] * sin_theta;

        Matrix {
            indices: [
                a, b, c, d, e, f, g, h, i, j, k, l, m[12], m[13], m[14], m[15],
            ],
        }
    }

    pub fn rotate_y(&self, angle: f32) -> Matrix {
        let sin_theta = angle.sin();
        let cos_theta = angle.cos();

        let m = self.indices;

        let a = m[0] * cos_theta + m[8] * sin_theta;
        let b = m[1] * cos_theta + m[9] * sin_theta;
        let c = m[2] * cos_theta + m[10] * sin_theta;
        let d = m[3] * cos_theta + m[11] * sin_theta;

        let e = m[4];
        let f = m[5];
        let g = m[6];
        let h = m[7];

        let i = m[8] * cos_theta - m[0] * sin_theta;
        let j = m[9] * cos_theta - m[1] * sin_theta;
        let k = m[10] * cos_theta - m[2] * sin_theta;
        let l = m[11] * cos_theta - m[3] * sin_theta;

        Matrix {
            indices: [
                a, b, c, d, e, f, g, h, i, j, k, l, m[12], m[13], m[14], m[15],
            ],
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