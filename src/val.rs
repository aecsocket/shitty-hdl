use std::marker::PhantomData;

use crate::Signal;

pub trait Shape: Clone + Copy {}
impl Shape for bool {}
impl Shape for u8 {}
impl Shape for u16 {}
impl Shape for u32 {}
impl Shape for u64 {}

pub trait Val<T: Shape> {}
impl<T: Shape> Val<T> for T {}
impl<T: Shape> Val<T> for Signal<T> {}

pub trait Cond {}
impl<T: Val<bool>> Cond for T {}

macro_rules! impl_val {
    ($ty:ident [ $($ty_param:ident),* $(,)? ], $T:ty) => {
        impl<$($ty_param: $crate::Shape),*> $crate::Val<$T>
        for $ty<$($ty_param,)*> {}
//
//         impl<$($ty_param: $crate::Shape),*, Rhs: $crate::Val<$T>> std::ops::Add<Rhs>
//         for $ty<$($ty_param,)*> {
//             type Output =
//         }
    };
}
pub(crate) use impl_val;
