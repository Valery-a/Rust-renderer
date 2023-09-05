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
