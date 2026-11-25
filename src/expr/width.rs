use crate::{
    expr::{Expr, IntoExpr},
    util::ty::{Assert, True},
    value::{Shape, ShapeSize, Signed, Sint, Uint, Unsigned, cap_of},
};

// zext

pub trait CanZext<To>: Unsigned {}
impl<const FROM: ShapeSize, const TO: ShapeSize> CanZext<Uint<TO>> for Uint<FROM>
where
    [(); cap_of::<FROM>()]:,
    [(); cap_of::<TO>()]:,
    Assert<{ TO >= FROM }>: True,
{
}

pub fn zext<To: Shape>(lhs: impl IntoExpr<Out: CanZext<To>>) -> Expr<To> {
    todo!();
}

// sext

pub trait CanSext<To>: Signed {}
impl<const FROM: ShapeSize, const TO: ShapeSize> CanSext<Sint<TO>> for Sint<FROM>
where
    [(); cap_of::<FROM>()]:,
    [(); cap_of::<TO>()]:,
    Assert<{ TO >= FROM }>: True,
{
}

pub fn sext<To: Shape>(lhs: impl IntoExpr<Out: CanSext<To>>) -> Expr<To> {
    todo!();
}

// trunc

pub trait CanTrunc<To>: Shape {
    type Truncated;
}

impl<const FROM: ShapeSize, const TO: ShapeSize> CanTrunc<Uint<TO>> for Uint<FROM>
where
    [(); cap_of::<FROM>()]:,
    [(); cap_of::<TO>()]:,
    Assert<{ TO <= FROM }>: True,
{
    type Truncated = Uint<TO>;
}

impl<const FROM: ShapeSize, const TO: ShapeSize> CanTrunc<Sint<TO>> for Sint<FROM>
where
    [(); cap_of::<FROM>()]:,
    [(); cap_of::<TO>()]:,
    Assert<{ TO <= FROM }>: True,
{
    type Truncated = Sint<TO>;
}

pub fn trunc<To: Shape>(lhs: impl IntoExpr<Out: CanTrunc<To>>) -> Expr<To> {
    todo!();
}
