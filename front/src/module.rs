use std::marker::PhantomData;

use crate::value::Shape;

pub trait Interface {
    type Define;
}

pub struct Action<In: Shape, Out: Shape>(PhantomData<(In, Out)>);
pub struct Define<In: Shape, Out: Shape>(PhantomData<(In, Out)>);
