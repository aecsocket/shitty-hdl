use crate::{
    expr::{Expr, IntoExpr},
    util::ty::{Assert, True},
    value::{I, Shape, ShapeSize, Signed, U, Unsigned, cap_of},
};

// zext

pub trait CanZext<To>: Unsigned {}
impl<const FROM: ShapeSize, const TO: ShapeSize> CanZext<U<TO>> for U<FROM>
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
impl<const FROM: ShapeSize, const TO: ShapeSize> CanSext<I<TO>> for I<FROM>
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

impl<const FROM: ShapeSize, const TO: ShapeSize> CanTrunc<U<TO>> for U<FROM>
where
    [(); cap_of::<FROM>()]:,
    [(); cap_of::<TO>()]:,
    Assert<{ TO <= FROM }>: True,
{
    type Truncated = U<TO>;
}

impl<const FROM: ShapeSize, const TO: ShapeSize> CanTrunc<I<TO>> for I<FROM>
where
    [(); cap_of::<FROM>()]:,
    [(); cap_of::<TO>()]:,
    Assert<{ TO <= FROM }>: True,
{
    type Truncated = I<TO>;
}

pub fn trunc<To: Shape>(lhs: impl IntoExpr<Out: CanTrunc<To>>) -> Expr<To> {
    todo!();
}
