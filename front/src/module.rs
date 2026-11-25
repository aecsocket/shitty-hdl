use std::marker::PhantomData;

use crate::value::Shape;

pub trait Interface {}

pub struct Method<In: Shape, Out: Shape>(PhantomData<(In, Out)>);

impl<In: Shape, Out: Shape> Method<In, Out> {
    pub fn define_as(f: impl AsyncFnOnce(In::Values) -> Out::Values) -> Self {
        todo!();
    }
}
