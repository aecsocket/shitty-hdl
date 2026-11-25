use std::marker::PhantomData;

use derive_more::Debug;

use crate::signal::SignalView;

pub trait Shape: Sized {
    type View: Copy;
}

impl Shape for () {
    type View = SignalView<Self>;
}

impl Shape for bool {
    type View = SignalView<Self>;
}

#[derive(Debug)]
pub struct Value<T> {
    #[debug(skip)]
    _phantom: PhantomData<T>,
}

pub trait IntoValue {
    type Out: Shape;
}
