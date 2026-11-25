use crate::{IntoValue, Shape, Value};

pub trait CmpExt<T: Shape>: IntoValue<T> + Sized {
    fn eq(self, rhs: impl IntoValue<T>) -> Eq<T> {
        Eq {
            lhs: self.into_value(),
            rhs: rhs.into_value(),
        }
    }

    fn ne(self, rhs: impl IntoValue<T>) -> Ne<T> {
        Ne {
            lhs: self.into_value(),
            rhs: rhs.into_value(),
        }
    }

    fn gt(self, rhs: impl IntoValue<T>) -> Gt<T> {
        Gt {
            lhs: self.into_value(),
            rhs: rhs.into_value(),
        }
    }

    fn ge(self, rhs: impl IntoValue<T>) -> Ge<T> {
        Ge {
            lhs: self.into_value(),
            rhs: rhs.into_value(),
        }
    }
}
impl<T: Shape, V: IntoValue<T>> CmpExt<T> for V {}

pub struct Eq<T: Shape> {
    lhs: Value<T>,
    rhs: Value<T>,
}

impl<T: Shape> IntoValue<bool> for Eq<T> {
    fn into_value(self) -> Value<bool> {
        todo!();
    }
}

pub struct Ne<T: Shape> {
    lhs: Value<T>,
    rhs: Value<T>,
}

impl<T: Shape> IntoValue<bool> for Ne<T> {
    fn into_value(self) -> Value<bool> {
        todo!();
    }
}

pub struct Gt<T: Shape> {
    lhs: Value<T>,
    rhs: Value<T>,
}

impl<T: Shape> IntoValue<bool> for Gt<T> {
    fn into_value(self) -> Value<bool> {
        todo!();
    }
}

pub struct Ge<T: Shape> {
    lhs: Value<T>,
    rhs: Value<T>,
}

impl<T: Shape> IntoValue<bool> for Ge<T> {
    fn into_value(self) -> Value<bool> {
        todo!();
    }
}
