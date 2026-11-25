use std::marker::PhantomData;

use derive_more::Debug;

use crate::signal::SignalView;

pub trait Shape: Sized {
    type View<'d>: Copy;
}

impl Shape for () {
    type View<'d> = SignalView<'d, Self>;
}

impl Shape for bool {
    type View<'d> = SignalView<'d, Self>;
}

#[derive(Debug)]
pub struct Value<T> {
    #[debug(skip)]
    _phantom: PhantomData<T>,
}

pub trait IntoValue {
    type Out: Shape;
}
