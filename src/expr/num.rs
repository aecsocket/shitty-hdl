use std::ops::{Add, Mul, Sub};

use crate::{
    expr::{Expr, IntoExpr},
    module::{In, Out},
    signal::Signal,
    value::{Shape, ShapeSize, Sint, Uint, cap_of},
};

pub trait LowerAdd {}
impl<const N: ShapeSize> LowerAdd for Uint<N> where [(); cap_of::<N>()]: {}
impl<const N: ShapeSize> LowerAdd for Sint<N> where [(); cap_of::<N>()]: {}

pub trait LowerSub {}
impl<const N: ShapeSize> LowerSub for Uint<N> where [(); cap_of::<N>()]: {}
impl<const N: ShapeSize> LowerSub for Sint<N> where [(); cap_of::<N>()]: {}

pub trait LowerMul {}
impl<const N: ShapeSize> LowerMul for Uint<N> where [(); cap_of::<N>()]: {}
impl<const N: ShapeSize> LowerMul for Sint<N> where [(); cap_of::<N>()]: {}

macro_rules! impl_for {
    ($($ty:ident)*) => {
        $(
        impl<T: Shape + LowerAdd, Rhs: IntoExpr<Out = T>> Add<Rhs> for $ty<T> {
            type Output = Expr<T>;

            fn add(self, rhs: Rhs) -> Self::Output {
                todo!();
            }
        }

        impl<T: Shape + LowerSub, Rhs: IntoExpr<Out = T>> Sub<Rhs> for $ty<T>
        {
            type Output = Expr<T>;

            fn sub(self, rhs: Rhs) -> Self::Output {
                todo!();
            }
        }

        impl<T: Shape + LowerMul, Rhs: IntoExpr<Out = T>> Mul<Rhs> for $ty<T>
        {
            type Output = Expr<T>;

            fn mul(self, rhs: Rhs) -> Self::Output {
                todo!();
            }
        }
        )*
    };
}

impl_for!(Expr Signal In Out);

pub trait CmpExt: IntoExpr {
    fn gt(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;

    fn lt(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;

    fn ge(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;

    fn le(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;
}

impl<const N: ShapeSize> CmpExt for Uint<N>
where
    [(); cap_of::<N>()]:,
{
    fn gt(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        todo!();
    }

    fn lt(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        todo!();
    }

    fn ge(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        todo!();
    }

    fn le(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        todo!();
    }
}

impl<const N: ShapeSize> CmpExt for Sint<N>
where
    [(); cap_of::<N>()]:,
{
    fn gt(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        todo!();
    }

    fn lt(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        todo!();
    }

    fn ge(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        todo!();
    }

    fn le(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        todo!();
    }
}
