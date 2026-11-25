use std::ops::{Deref, DerefMut, ShlAssign};

pub use crate::ir::signal::{Signal, SignalId, SignalView};

use crate::{
    expr::{Value, IntoValue},
    ir,
    value::Shape,
};

impl<T: Shape> Signal<T> {
    #[must_use]
    #[track_caller]
    pub fn new(init: impl Into<T>) -> Self {
        Self::allocate(init.into().to_bits(), None, None)
    }

    #[must_use]
    #[track_caller]
    pub fn without_init() -> Self {
        Self::allocate(ir::PackedBits::zero(T::WIDTH), None, None)
    }

    #[must_use]
    #[track_caller]
    pub fn input(name: impl Into<String>, init: impl Into<T>) -> Self {
        Self::allocate(
            init.into().to_bits(),
            Some(name.into()),
            Some(ir::PortDirection::Input),
        )
    }

    #[must_use]
    #[track_caller]
    pub fn output(name: impl Into<String>, init: impl Into<T>) -> Self {
        Self::allocate(
            init.into().to_bits(),
            Some(name.into()),
            Some(ir::PortDirection::Output),
        )
    }

    pub fn id(&self) -> SignalId {
        self.id
    }
}

impl<T: Shape> IntoValue for Signal<T> {
    type Out = T;
    #[track_caller]
    fn into_value(self) -> Value<T> {
        Value {
            imp: ir::Value::read(self.id, 0),
        }
    }
}

impl<T: Shape> IntoValue for SignalView<T> {
    type Out = T;
    #[track_caller]
    fn into_value(self) -> Value<T> {
        Value {
            imp: ir::Value::read(self.id, self.offset),
        }
    }
}

impl<T: Shape> Deref for Signal<T> {
    type Target = T::View;
    fn deref(&self) -> &T::View {
        &self.view
    }
}
impl<T: Shape> DerefMut for Signal<T> {
    fn deref_mut(&mut self) -> &mut T::View {
        &mut self.view
    }
}

impl<T: Shape, Rhs: IntoValue<Out = T>> ShlAssign<Rhs> for Signal<T> {
    #[track_caller]
    fn shl_assign(&mut self, rhs: Rhs) {
        rhs.into_value().imp.assign(self.id, 0);
    }
}

impl<T: Shape, Rhs: IntoValue<Out = T>> ShlAssign<Rhs> for SignalView<T> {
    #[track_caller]
    fn shl_assign(&mut self, rhs: Rhs) {
        rhs.into_value().imp.assign(self.id, self.offset);
    }
}
