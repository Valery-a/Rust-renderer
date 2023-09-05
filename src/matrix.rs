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
