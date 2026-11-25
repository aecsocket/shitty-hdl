mod bool;
mod num;
mod width;

pub use num::CmpExt;
pub use width::*;

use crate::{ir, value::Shape};

#[derive(Debug)]
pub struct Value<T: Shape> {
    pub(crate) imp: ir::Value<T>,
}

impl<T: Shape> Clone for Value<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Shape> Copy for Value<T> {}

pub trait IntoValue {
    type Out: Shape;

    fn into_value(self) -> Value<Self::Out>;
}

impl<T: Shape> IntoValue for T {
    type Out = T;

    #[track_caller]
    fn into_value(self) -> Value<T> {
        Value {
            imp: ir::Value::constant(self),
        }
    }
}

impl<T: Shape> IntoValue for Value<T> {
    type Out = T;

    #[track_caller]
    fn into_value(self) -> Value<T> {
        Self {
            imp: self.imp.checked(),
        }
    }
}

pub trait EqExt: IntoValue {
    fn eq(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool>;
    fn ne(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool>;
}
