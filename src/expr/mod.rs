mod bool;
mod num;
mod width;

pub use {bool::*, num::CmpExt, width::*};

use crate::{ir, value::Shape};

#[derive(Debug)]
pub struct Expr<T: Shape> {
    pub(crate) imp: ir::Expr<T>,
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

    #[track_caller]
    fn into_expr(self) -> Expr<T> {
        Expr {
            imp: ir::Expr::constant(self),
        }
    }
}

impl<T: Shape> IntoExpr for Expr<T> {
    type Out = T;

    #[track_caller]
    fn into_expr(self) -> Expr<T> {
        Self {
            imp: self.imp.checked(),
        }
    }
}

pub trait EqExt: IntoExpr {
    fn eq(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;
    fn ne(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;
}
