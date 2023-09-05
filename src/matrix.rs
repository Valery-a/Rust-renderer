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


impl<   T : Add<Output=T> + Value,
    N : Clone + Unsigned,
    M : Clone + Unsigned>

AddAssign<Mat<T,N,M>>

for Mat<T,N,M> where N : Mul<M>, Prod<N,M> : ArrayLength<T>{


    fn add_assign(&mut self, other : Mat<T,N,M>){
        *self = Mat{ar : GenericArray::<T, Prod<N, M>>::
        generate(&|i| self.get(i) + other.get(i))}
    }

}

impl<   T : Mul<Output=T> + Value,
    N : Clone + Unsigned,
    M : Clone + Unsigned>

MulAssign<T>

for Mat<T,N,M> where N : Mul<M>, Prod<N,M> : ArrayLength<T>{


    fn mul_assign(&mut self, k : T){
        *self = Mat{ar : GenericArray::<T, Prod<N, M>>::
        generate(&|i| self.get(i) * k)}
    }

}

impl<   T : Sub<Output=T> + Value,
    N : Clone + Unsigned,
    M : Clone + Unsigned>

SubAssign<Mat<T,N,M>>

for Mat<T,N,M> where N : Mul<M>, Prod<N,M> : ArrayLength<T>{


    fn sub_assign(&mut self, other : Mat<T,N,M>){
        *self = Mat{ar : GenericArray::<T, Prod<N, M>>::
        generate(&|i| self.get(i) - other.get(i))}
    }

}

impl<   T : Neg<Output=T> + Value,
    N : Clone + Unsigned,
    M : Clone + Unsigned>

Neg

for Mat<T,N,M> where N : Mul<M>, Prod<N,M> : ArrayLength<T>{

    type Output = Mat<T,N,M>;

    fn neg(self) -> Mat<T,N,M>{
        Mat{ar : GenericArray::<T, Prod<N, M>>::
        generate(&|i| -self.get(i))}
    }

}

impl<   T : Mul<Output=T> + Value,
    N : Clone + Unsigned,
    M : Clone + Unsigned>

Mul<T>

for Mat<T,N,M> where N : Mul<M>, Prod<N,M> : ArrayLength<T>{

    type Output = Mat<T,N,M>;

    fn mul(self, k : T) -> Mat<T,N,M>{
        Mat{ar : GenericArray::<T, Prod<N, M>>::
        generate(&|i| self.get(i) * k)}
    }

}

impl<
        T : Add<Output=T> + Value + Mul<Output=T> + AdditiveMonoid,
        N : Unsigned + Clone,
        M : Unsigned + Clone,
        L : Unsigned + Clone>

Mul<Mat<T,M,L>>

for Mat<T,N,M> where N : Mul<M>, N : Mul<L>, M : Mul<L>, Prod<N,M> : ArrayLength<T>, Prod<M, L> : ArrayLength<T>, Prod<N, L> : ArrayLength<T>{

    type Output = Mat<T,N,L>;

    fn mul(self, other : Mat<T,M,L>) -> Mat<T,N,L>{

        let mut c = Mat::<T, N, L>::empty();

        for i in 0..N::to_usize(){
            for j in 0..L::to_usize(){
                c[(i,j)] = T::identity();
                for k in 0..M::to_usize(){
                    c[(i,j)] += self[(i,k)] * other[(k,j)];
                }
            }
        }

        c
    }

}

impl<A : Value + Identity<Additive>, N : Unsigned + Clone, M : Unsigned + Clone> Mat<A, N, M> where N : Mul<M>, Prod<N, M> : ArrayLength<A>, M : Mul<N>, Prod<M, N> : ArrayLength<A>{
    pub fn transpose(&self) -> Mat<A, M, N>{
        let mut r = Mat::<A, M, N>::empty();
        for i in 0..N::to_usize(){
            for j in 0..M::to_usize(){
                r[(j, i)] =  self[(i, j)];
            }
        }

        r
    }
}

impl<   T : Sub<Output=T> + Value,
        N : Clone + Unsigned,
        M : Clone + Unsigned>

Sub<Mat<T,N,M>>
for Mat<T,N,M> where N : Mul<M>, Prod<N,M> : ArrayLength<T>{
        type Output = Mat<T,N,M>;

        fn sub(self, other : Mat<T,N,M>) -> Mat<T,N,M>{
                Mat{ar : GenericArray::<T, Prod<N, M>>::
                generate(&|i| self.get(i) - other.get(i))}
        }

}

pub fn dot<T : Value + Identity<Additive> + Mul<Output=T> + AddAssign, N : Unsigned + Clone + Mul<U1>>(that : Vec<T,N>, other : Vec<T,N>) -> T where N : Mul<U1>, Prod<N,U1> : ArrayLength<T>,{
        let mut res = T::identity();
        for i in 0..<N as Unsigned>::to_usize(){
                res += that.ar[i] * other.ar[i];
        }

        res
}


impl<
    T : Mul<Output=T> + AddAssign + AbstractMonoid<Additive> + Value,
    N : Clone + Unsigned>

Vec<T,N> where N : Mul<U1> + Unsigned, Prod<N,U1> : ArrayLength<T>{


    #[inline]
    pub fn dot(self, other : Vec<T,N>) -> T{
        dot(self, other)
    }


}

impl<
    T : Real,
    N : Clone + Unsigned>

Vec<T,N> where N : Mul<U1> + Unsigned, Prod<N,U1> : ArrayLength<T>, GenericArray<T, Prod<N, U1>> : Copy{

    #[inline]
    pub fn norm(self) -> T{
        T::sqrt(dot(self, self))
    }

    #[inline]
    pub fn normalize(self) -> Vec<T, N>{
        self * (T::one() / self.norm())
    }

}

macro_rules! vec3 {
    ( $x:expr , $y:expr, $z:expr ) => {
        {
            Vec3::new($x, $y, $z)
        }
    };
}