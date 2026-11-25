mod arith;
mod width;

pub use width::{CanTrunc, CanZext, Trunc, Zext, trunc, zext};

use crate::value::Shape;

pub trait IntoExpr {
    type Out: Shape;
}

impl<T: Shape> IntoExpr for T {
    type Out = Self;
}
