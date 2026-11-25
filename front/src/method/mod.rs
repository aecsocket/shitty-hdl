use crate::{module::Action, value::Shape};

pub struct Method<In: Shape, Out: Shape> {
    pub req: Action<In, ()>,
    pub out: Action<(), Out>,
}

impl<In: Shape, Out: Shape> Method<In, Out> {
    #[must_use]
    pub const fn new(req: Action<In, ()>, out: Action<(), Out>) -> Self {
        Self { req, out }
    }

    pub fn define_as(f: impl AsyncFnOnce(In::Values) -> Out::Values) -> Self {
        drop(f);
        todo!();
    }
}
