use typenum::{B1, IsGreaterOrEqual, IsLessOrEqual};

use crate::{
    expr::{Expr, IntoExpr},
    value::{Shape, Signed, Sint, Uint, Unsigned, Width},
};

// zext

pub trait CanZext<To>: Unsigned {}
impl<From: Width, To: Width> CanZext<Uint<To>> for Uint<From> where
    To: IsGreaterOrEqual<From, Output = B1>
{
}

#[track_caller]
pub fn zext<To: Shape>(lhs: impl IntoExpr<Out: CanZext<To>>) -> Expr<To> {
    Expr {
        imp: lhs.into_expr().imp.resize::<To>(true),
    }
}

// sext

pub trait CanSext<To>: Signed {}
impl<From: Width, To: Width> CanSext<Sint<To>> for Sint<From> where
    To: IsGreaterOrEqual<From, Output = B1>
{
}

#[track_caller]
pub fn sext<To: Shape>(lhs: impl IntoExpr<Out: CanSext<To>>) -> Expr<To> {
    Expr {
        imp: lhs.into_expr().imp.resize::<To>(true),
    }
}

// trunc

pub trait CanTrunc<To>: Shape {
    type Truncated;
}

impl<From: Width, To: Width> CanTrunc<Uint<To>> for Uint<From>
where
    To: IsLessOrEqual<From, Output = B1>,
{
    type Truncated = Uint<To>;
}

impl<From: Width, To: Width> CanTrunc<Sint<To>> for Sint<From>
where
    To: IsLessOrEqual<From, Output = B1>,
{
    type Truncated = Sint<To>;
}

#[track_caller]
pub fn trunc<To: Shape>(lhs: impl IntoExpr<Out: CanTrunc<To>>) -> Expr<To> {
    Expr {
        imp: lhs.into_expr().imp.resize::<To>(false),
    }
}
