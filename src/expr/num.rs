use std::ops::{Add, Mul, Sub};

use crate::{
    expr::{Expr, IntoExpr},
    ir::BinaryOp,
    module::{In, Out},
    signal::{Signal, SignalView},
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
    ($($ty:ident)*) => {$(
        impl<T: Shape + LowerAdd, Rhs: IntoExpr<Out = T>> Add<Rhs> for $ty<T> {
            type Output = Expr<T>;
            #[track_caller]
            fn add(self, rhs: Rhs) -> Expr<T> { Expr { imp: self.into_expr().imp.binary(rhs.into_expr().imp, BinaryOp::Add) } }
        }
        impl<T: Shape + LowerSub, Rhs: IntoExpr<Out = T>> Sub<Rhs> for $ty<T> {
            type Output = Expr<T>;
            #[track_caller]
            fn sub(self, rhs: Rhs) -> Expr<T> { Expr { imp: self.into_expr().imp.binary(rhs.into_expr().imp, BinaryOp::Sub) } }
        }
        impl<T: Shape + LowerMul, Rhs: IntoExpr<Out = T>> Mul<Rhs> for $ty<T> {
            type Output = Expr<T>;
            #[track_caller]
            fn mul(self, rhs: Rhs) -> Expr<T> { Expr { imp: self.into_expr().imp.binary(rhs.into_expr().imp, BinaryOp::Mul) } }
        }
    )*};
}
impl_for!(Expr Signal SignalView In Out);

pub trait CmpExt: IntoExpr {
    fn gt(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;
    fn lt(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;
    fn ge(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;
    fn le(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;
}

impl<E: IntoExpr> CmpExt for E
where
    E::Out: LowerAdd,
{
    #[track_caller]
    fn gt(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        Expr {
            imp: self
                .into_expr()
                .imp
                .compare(rhs.into_expr().imp, BinaryOp::Gt),
        }
    }
    #[track_caller]
    fn lt(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        Expr {
            imp: self
                .into_expr()
                .imp
                .compare(rhs.into_expr().imp, BinaryOp::Lt),
        }
    }
    #[track_caller]
    fn ge(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        Expr {
            imp: self
                .into_expr()
                .imp
                .compare(rhs.into_expr().imp, BinaryOp::Ge),
        }
    }
    #[track_caller]
    fn le(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        Expr {
            imp: self
                .into_expr()
                .imp
                .compare(rhs.into_expr().imp, BinaryOp::Le),
        }
    }
}
