use std::{marker::PhantomData, ops};

use crate::{Cond, Shape, Stmt, Val};

#[derive(Clone, Copy)]
pub struct Signal<T: Shape> {
    init: Option<T>,
    _phantom: PhantomData<T>,
}

impl<T: Shape> Signal<T> {
    #[must_use]
    pub fn new(init: T) -> Self {
        Self {
            init: Some(init),
            _phantom: PhantomData,
        }
    }

    pub fn without_reset() -> Self {
        Self {
            init: None,
            _phantom: PhantomData,
        }
    }
}

impl<T: Shape, Rhs: Val<T>> ops::ShlAssign<Rhs> for Signal<T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        todo!();
    }
}
