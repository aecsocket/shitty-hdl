use std::ops;

use crate::{IntoValue, Shape, Value};

pub trait ValAdd<Rhs = Self>: Shape {}
pub trait ValSub<Rhs = Self>: Shape {}
pub trait ValMul<Rhs = Self>: Shape {}
pub trait ValDiv<Rhs = Self>: Shape {}

macro_rules! impl_ops_for_ty {
    ($($ty:ty)*) => {
        $(
        impl Shape for $ty {}
        impl ValAdd<$ty> for $ty {}
        impl ValSub<$ty> for $ty {}
        impl ValMul<$ty> for $ty {}
        impl ValDiv<$ty> for $ty {}
        )*
    };
}

impl_ops_for_ty!(u8 u16 u32 u64 u128);
impl_ops_for_ty!(i8 i16 i32 i64 i128);

impl<T: ValAdd<T>, Rhs: IntoValue<T>> ops::Add<Rhs> for Value<T> {
    type Output = ();

    fn add(self, rhs: Rhs) -> Self::Output {
        todo!();
    }
}
