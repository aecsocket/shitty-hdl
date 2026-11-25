use std::ops::{Add, Mul, Sub};

use crate::{
    expr::{Expr, IntoExpr},
    module::{In, Out},
    signal::Signal,
    value::{Shape, Sint, Uint, Width},
};

pub trait LowerAdd {}
impl<N: Width> LowerAdd for Uint<N> {}
impl<N: Width> LowerAdd for Sint<N> {}

pub trait LowerSub {}
impl<N: Width> LowerSub for Uint<N> {}
impl<N: Width> LowerSub for Sint<N> {}

pub trait LowerMul {}
impl<N: Width> LowerMul for Uint<N> {}
impl<N: Width> LowerMul for Sint<N> {}

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

impl<N: Width> CmpExt for Uint<N> {
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

impl<N: Width> CmpExt for Sint<N> {
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
