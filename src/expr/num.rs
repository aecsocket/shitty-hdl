use std::ops::{Add, Mul, Sub};

use crate::{
    expr::{Expr, IntoExpr},
    module::{In, Out},
    signal::Signal,
    value::{I, ShapeSize, U},
};

macro_rules! impl_for {
    ($($ty:ident)*) => {
        $(
        // add

        impl<const N: ShapeSize, Rhs: IntoExpr<Out = U<N>>> Add<Rhs> for $ty<U<N>> {
            type Output = Expr<U<N>>;

            fn add(self, rhs: Rhs) -> Self::Output {
                todo!();
            }
        }

        impl<const N: ShapeSize, Rhs: IntoExpr<Out = I<N>>> Add<Rhs> for $ty<I<N>> {
            type Output = Expr<I<N>>;

            fn add(self, rhs: Rhs) -> Self::Output {
                todo!();
            }
        }

        // sub

        impl<const N: ShapeSize, Rhs: IntoExpr<Out = U<N>>> Sub<Rhs> for $ty<U<N>> {
            type Output = Expr<U<N>>;

            fn sub(self, rhs: Rhs) -> Self::Output {
                todo!();
            }
        }

        impl<const N: ShapeSize, Rhs: IntoExpr<Out = I<N>>> Sub<Rhs> for $ty<I<N>> {
            type Output = Expr<I<N>>;

            fn sub(self, rhs: Rhs) -> Self::Output {
                todo!();
            }
        }

        // mul

        impl<const N: ShapeSize, Rhs: IntoExpr<Out = U<N>>> Mul<Rhs> for $ty<U<N>> {
            type Output = Expr<U<N>>;

            fn mul(self, rhs: Rhs) -> Self::Output {
                todo!();
            }
        }

        impl<const N: ShapeSize, Rhs: IntoExpr<Out = I<N>>> Mul<Rhs> for $ty<I<N>> {
            type Output = Expr<I<N>>;

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
