use std::marker::PhantomData;

use crate::{expr::IntoExpr, value::Shape};

pub fn if_<R: Shape>(cond: impl IntoExpr<Out = bool>, then: impl FnOnce() -> R) -> If<R> {
    If {
        _phantom: PhantomData,
    }
}

pub struct If<R> {
    _phantom: PhantomData<R>,
}

impl<R: Shape> If<R> {
    pub fn else_if(self, cond: impl IntoExpr<Out = bool>, then: impl FnOnce() -> R) -> If<R> {
        self
    }

    pub fn else_(self, then: impl FnOnce() -> R) -> R {
        todo!();
    }
}
