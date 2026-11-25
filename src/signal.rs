use std::marker::PhantomData;

use crate::{Cond, Shape, Stmt, Val};

#[derive(Clone, Copy)]
pub struct Signal<T: Shape> {
    _phantom: PhantomData<T>,
    do_reset: bool,
}

impl<T: Shape> Signal<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
            do_reset: true,
        }
    }

    #[must_use]
    pub fn without_reset(self) -> Self {
        Self {
            do_reset: false,
            ..self
        }
    }

    pub fn set(self, rhs: impl Val<T>) -> Stmt {
        todo!();
    }

    pub fn eq(self, rhs: impl Val<T>) -> impl Cond {
        todo!();
        false
    }

    pub fn ne(self, rhs: impl Val<T>) -> impl Cond {
        todo!();
        false
    }
}
