use std::marker::PhantomData;
use std::ops::{BitAnd, BitOr, Not, ShlAssign};

use crate::{
    domain::{Clock, Domain},
    elab::{self, ActionHandle, ExprHandle},
    value::{IntoValue, IntoValues, Value},
};

#[derive(Debug)]
pub struct Output<D: Domain> {
    value: ExprHandle,
    writer: ActionHandle,
    _brand: PhantomData<D>,
}

impl<D: Domain> Clone for Output<D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Domain> Copy for Output<D> {}

impl<D: Domain> Output<D> {
    #[must_use]
    pub fn new(clock: Clock<D>, name: &str, active_low: bool) -> Self {
        let (value, writer) = elab::io::output(clock, name, active_low);
        Self {
            value,
            writer,
            _brand: PhantomData,
        }
    }

    #[must_use]
    pub fn read(self) -> Value<D, bool> {
        Value::from_handle(self.value)
    }

    pub fn write(&mut self, value: impl IntoValue<D, bool>) {
        elab::call_action::<D, bool, ()>(self.writer, value.into_value());
    }
}

impl<D: Domain> IntoValue<D, bool> for Output<D> {
    fn into_value(self) -> Value<D, bool> {
        self.read()
    }
}

impl<D: Domain> IntoValues<D> for Output<D> {
    type Out = bool;
    type Values = Value<D, bool>;

    fn into_values(self) -> Self::Values {
        self.read()
    }
}

impl<D: Domain, Rhs: IntoValue<D, bool>> ShlAssign<Rhs> for Output<D> {
    fn shl_assign(&mut self, rhs: Rhs) {
        self.write(rhs);
    }
}

impl<D: Domain> Not for Output<D> {
    type Output = Value<D, bool>;

    fn not(self) -> Self::Output {
        !self.read()
    }
}

impl<D: Domain, Rhs: IntoValue<D, bool>> BitAnd<Rhs> for Output<D> {
    type Output = Value<D, bool>;

    fn bitand(self, rhs: Rhs) -> Self::Output {
        self.read() & rhs
    }
}

impl<D: Domain, Rhs: IntoValue<D, bool>> BitOr<Rhs> for Output<D> {
    type Output = Value<D, bool>;

    fn bitor(self, rhs: Rhs) -> Self::Output {
        self.read() | rhs
    }
}

#[must_use]
pub fn input<D: Domain>(clock: Clock<D>, name: &str, active_low: bool) -> Value<D, bool> {
    Value::from_handle(elab::io::input(clock, name, active_low))
}
