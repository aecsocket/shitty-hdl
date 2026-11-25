use std::marker::PhantomData;

use crate::{
    expr::IntoExpr,
    value::{Shape, ShapeSize, Uint},
};

pub struct Assert<const CONDITION: bool>;

pub trait True {}
impl True for Assert<true> {}

// ops

pub trait CanZext<To: Shape>: Shape {}

impl<const FROM: ShapeSize, const TO: ShapeSize> CanZext<Uint<TO>> for Uint<FROM> where
    Assert<{ TO >= FROM }>: True
{
}

pub fn zext<T, Lhs, To>(lhs: Lhs) -> Zext<Lhs, To>
where
    T: Shape + CanZext<To>,
    Lhs: IntoExpr<Out = Lhs>,
    To: Shape,
{
    todo!();
}

pub struct Zext<Lhs, To> {
    operand: Lhs,
    _to: PhantomData<To>,
}

impl<Lhs, To: Shape> IntoExpr for Zext<Lhs, To> {
    type Out = To;
}

pub trait CanTrunc<To: Shape>: Shape {}

impl<const FROM: ShapeSize, const TO: ShapeSize> CanTrunc<Uint<TO>> for Uint<FROM> where
    Assert<{ TO <= FROM }>: True
{
}

pub fn trunc<T, Lhs, To>(lhs: Lhs) -> Trunc<Lhs, To>
where
    T: Shape + CanTrunc<To>,
    Lhs: IntoExpr<Out = Lhs>,
    To: Shape,
{
    todo!();
}

pub struct Trunc<Lhs, To> {
    operand: Lhs,
    _to: PhantomData<To>,
}

impl<Lhs, To: Shape> IntoExpr for Trunc<Lhs, To> {
    type Out = To;
}
