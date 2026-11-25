use std::ops::{BitAnd, BitOr, Not, ShlAssign};

use crate::{
    domain::Clock,
    elab::{self, ActionHandle, ExprHandle},
    value::{IntoValue, IntoValues, Value},
};

#[derive(Debug, Clone, Copy)]
pub struct Output {
    value: ExprHandle,
    writer: ActionHandle,
}

impl Output {
    pub fn new(clock: Clock, name: &str, active_low: bool) -> Self {
        let (value, writer) = elab::io::output(clock, name, active_low);
        Self { value, writer }
    }

    pub fn read(self) -> Value<bool> {
        Value::from_handle(self.value)
    }

    pub fn write(&mut self, value: impl IntoValue<Out = bool>) {
        elab::call_action::<bool, ()>(self.writer, value.into_value());
    }
}

impl IntoValue for Output {
    type Out = bool;

    fn into_value(self) -> Value<bool> {
        self.read()
    }
}

impl IntoValues for Output {
    type Out = bool;
    type Values = Value<bool>;

    fn into_values(self) -> Self::Values {
        self.read()
    }
}

impl<Rhs: IntoValue<Out = bool>> ShlAssign<Rhs> for Output {
    fn shl_assign(&mut self, rhs: Rhs) {
        self.write(rhs);
    }
}

impl Not for Output {
    type Output = Value<bool>;

    fn not(self) -> Self::Output {
        !self.read()
    }
}

impl<Rhs: IntoValue<Out = bool>> BitAnd<Rhs> for Output {
    type Output = Value<bool>;

    fn bitand(self, rhs: Rhs) -> Self::Output {
        self.read() & rhs
    }
}

impl<Rhs: IntoValue<Out = bool>> BitOr<Rhs> for Output {
    type Output = Value<bool>;

    fn bitor(self, rhs: Rhs) -> Self::Output {
        self.read() | rhs
    }
}

pub fn input(clock: Clock, name: &str, active_low: bool) -> Value<bool> {
    Value::from_handle(elab::io::input(clock, name, active_low))
}
