mod bool;
mod num;
mod width;

use std::marker::PhantomData;

pub use {bool::*, width::*};

use crate::value::Shape;

#[derive(Debug)]
pub struct Expr<T: Shape> {
    _phantom: PhantomData<T>,
}

impl<T: Shape> Clone for Expr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for Expr<T> {}

pub trait IntoExpr {
    type Out: Shape;

    fn into_expr(self) -> Expr<Self::Out>;
}

impl<T: Shape> IntoExpr for T {
    type Out = T;

    fn into_expr(self) -> Expr<Self::Out> {
        todo!();
    }
}

impl<T: Shape> IntoExpr for Expr<T> {
    type Out = T;

    fn into_expr(self) -> Expr<T> {
        self
    }
}
