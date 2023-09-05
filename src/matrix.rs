use typenum::*;
use std::ops::*;
use generic_array::*;
use alga::general::*;
use std::mem;
use std::fmt::{Debug, Display, Formatter};
use std::fmt;

//Mat, new,

pub trait Value: Copy + PartialEq + Debug  {
}
impl<T: Copy + PartialEq + Debug> Value for T {}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
#[repr(C)]
pub struct Mat<T : Value,N : Clone + Unsigned,M : Clone + Unsigned> where
    N : Mul<M>,
    Prod<N,M> : ArrayLength<T>{

        pub ar : GenericArray<T, typenum::Prod<N,M>>,
}

impl<T : Value, N : Clone + Unsigned, M : Clone + Unsigned> Copy for Mat<T, N, M>where
    N : Mul<M>,
    Prod<N,M> : ArrayLength<T>,
    GenericArray<T, typenum::Prod<N,M>> : Copy{

}


impl<T: Value + Display, N: Unsigned + Clone, M: Unsigned + Clone> Display for Mat<T, N, M>
where
    N: Mul<M>,
    Prod<N, M>: ArrayLength<T>,
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        for i in 0..N::to_usize() {
            write!(f, "[ ")?;
            for j in 0..M::to_usize() {
                write!(f, "{:10.4} ", self[(i, j)])?;
            }
            writeln!(f, "]")?;
        }
        Ok(())
    }
}

macro_rules! coords_impl(
    ($T: ident; $($comps: ident),*) => {
        #[repr(C)]
        #[derive(Eq, PartialEq, Clone, Hash, Debug, Copy)]
        pub struct $T<N : Value> {
            $(pub $comps: N),*
        }
    }
);

coords_impl!(X; x);
coords_impl!(XY; x, y);
coords_impl!(XYZ; x, y, z);
coords_impl!(XYZW; x, y, z, w);


macro_rules! deref_impl(
    ($R: ty, $C: ty; $Target: ident) => {
        impl<N : Value> Deref for Mat<N, $R, $C>{
            type Target = $Target<N>;


            #[inline]
            fn deref(&self) -> &Self::Target {
                unsafe { mem::transmute(&self.ar) }
            }
        }

        impl<N : Value> DerefMut for Mat<N, $R, $C>{
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                unsafe { mem::transmute(&mut self.ar) }
            }
        }
    }
);

deref_impl!(U1, U1; X);
deref_impl!(U2, U1; XY);
deref_impl!(U3, U1; XYZ);
deref_impl!(U4, U1; XYZW);

coords_impl!(M2x2; m11, m21,
                   m12, m22);
coords_impl!(M3x3; m11, m21, m31,
                   m12, m22, m32,
                   m13, m23, m33);

coords_impl!(M4x4; m11, m21, m31, m41,
                   m12, m22, m32, m42,
                   m13, m23, m33, m43,
                   m14, m24, m34, m44);

deref_impl!(U2, U2; M2x2);
deref_impl!(U3, U3; M3x3);
deref_impl!(U4, U4; M4x4);

pub type Vec<T, N> = Mat<T,N,U1>;
pub type Vec2<T> = Vec<T,U2>;
pub type Vec3<T> = Vec<T,U3>;
pub type Mat3<T> = Mat<T, U3, U3>;
pub type Mat4<T> = Mat<T, U4, U4>;
