use std::marker::PhantomData;

pub trait Shape: Sized {
    type Value;
}

pub struct Value<T: Shape>(PhantomData<T>);

impl<T: Shape> Clone for Value<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Shape> Copy for Value<T> {}

pub trait IntoValue {
    type Out: Shape;
}

impl<T: Shape> IntoValue for T {
    type Out = T;
}

impl<T: Shape> IntoValue for Value<T> {
    type Out = T;
}
