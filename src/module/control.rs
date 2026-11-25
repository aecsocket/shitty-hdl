use std::marker::PhantomData;

use crate::{
    expr::{Expr, IntoExpr},
    value::Shape,
};

pub fn if_<R: IntoExpr>(_cond: impl IntoExpr<Out = bool>, _then: impl FnOnce() -> R) -> If<R::Out> {
    If {
        _phantom: PhantomData,
    }
}

pub struct If<T: Shape> {
    _phantom: PhantomData<T>,
}

impl<T: Shape> If<T> {
    pub fn else_if<R: IntoExpr<Out = T>>(
        self,
        _cond: impl IntoExpr<Out = bool>,
        _then: impl FnOnce() -> R,
    ) -> Self {
        self
    }

    pub fn else_<R: IntoExpr<Out = T>>(self, _then: impl FnOnce() -> R) -> Expr<T> {
        todo!("lower hardware conditional into an expression")
    }
}
