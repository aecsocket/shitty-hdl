use std::ops::{Add, Mul};

use crate::{
    expr::IntoExpr,
    signal::Signal,
    value::{
        Shape,
        n::{u17, u18},
    },
};

pub trait LowerAdd<Rhs: Shape>: Shape {}

impl LowerAdd<u17> for u17 {}
impl LowerAdd<u18> for u18 {}

pub trait LowerMul<Rhs: Shape>: Shape {}

impl LowerMul<u17> for u17 {}
impl LowerMul<u18> for u18 {}

impl<L, Rhs> Add<Rhs> for Signal<L>
where
    Rhs: IntoExpr,
    L: Shape + LowerAdd<Rhs::Out>,
{
    type Output = ();

    fn add(self, rhs: Rhs) -> Self::Output {}
}

impl<L, Rhs> Mul<Rhs> for Signal<L>
where
    Rhs: IntoExpr,
    L: Shape + LowerMul<Rhs::Out>,
{
    type Output = ();

    fn mul(self, rhs: Rhs) -> Self::Output {}
}
