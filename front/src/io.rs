use std::marker::PhantomData;
use std::ops::{BitAnd, BitOr, Not, ShlAssign};

use crate::{
    domain::{Clock, Domain},
    elab::{self, ActionHandle, ExprHandle},
    value::{IntoValue, IntoValues, Value},
};

#[derive(Debug, Clone, Copy)]
pub struct Output<D: Domain> {
    value: ExprHandle,
    writer: ActionHandle,
    _brand: PhantomData<D>,
}

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
    pub const fn read(self) -> Value<bool> {
        Value::from_handle(self.value)
    }

    pub fn write(&mut self, value: impl IntoValue<Out = bool>) {
        elab::call_action::<bool, ()>(self.writer, value.into_value());
    }
}

impl<D: Domain> IntoValue for Output<D> {
    type Out = bool;

    fn into_value(self) -> Value<bool> {
        self.read()
    }
}

impl<D: Domain> IntoValues for Output<D> {
    type Out = bool;
    type Values = Value<bool>;

    fn into_values(self) -> Self::Values {
        self.read()
    }
}

impl<D: Domain, Rhs: IntoValue<Out = bool>> ShlAssign<Rhs> for Output<D> {
    fn shl_assign(&mut self, rhs: Rhs) {
        self.write(rhs);
    }
}

impl<D: Domain> Not for Output<D> {
    type Output = Value<bool>;

    fn not(self) -> Self::Output {
        !self.read()
    }
}

impl<D: Domain, Rhs: IntoValue<Out = bool>> BitAnd<Rhs> for Output<D> {
    type Output = Value<bool>;

    fn bitand(self, rhs: Rhs) -> Self::Output {
        self.read() & rhs
    }
}

impl<D: Domain, Rhs: IntoValue<Out = bool>> BitOr<Rhs> for Output<D> {
    type Output = Value<bool>;

    fn bitor(self, rhs: Rhs) -> Self::Output {
        self.read() | rhs
    }
}

#[must_use]
pub fn input<D: Domain>(clock: Clock<D>, name: &str, active_low: bool) -> Value<bool> {
    Value::from_handle(elab::io::input(clock, name, active_low))
}
