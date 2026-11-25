use std::{marker::PhantomData, ops};

use crate::{IntoValue, Shape, Value, elaborate::with_elaboration, impl_ops};

#[derive(Debug, Clone, Copy)]
pub struct Signal<T: Shape> {
    id: SignalId,
    _phantom: PhantomData<T>,
}

impl<T: Shape> Signal<T> {
    #[must_use]
    pub(crate) fn from_id(id: SignalId) -> Self {
        Self {
            id,
            _phantom: PhantomData,
        }
    }

    #[must_use]
    pub fn new(init: T) -> Self {
        todo!();
    }

    #[must_use]
    pub fn without_reset() -> Self {
        todo!();
    }
}

impl<T: Shape> IntoValue<T> for Signal<T> {
    fn into_value(self) -> Value<T> {
        todo!();
    }
}

impl<T: Shape, Rhs: IntoValue<T>> ops::ShlAssign<Rhs> for Signal<T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        todo!();
    }
}

impl_ops!(Signal<T>);

#[derive(Debug, Clone, Copy)]
pub struct SignalId(usize);

impl SignalId {
    #[must_use]
    pub const fn from_raw(raw: usize) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn into_raw(self) -> usize {
        self.0
    }
}
