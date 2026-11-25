use crate::value::{IntoValue, Shape, Value};

#[track_caller]
pub fn if_<R: IntoValue>(cond: impl IntoValue<Out = bool>, then: impl FnOnce() -> R) -> If<R::Out> {
    todo!();
}

pub struct If<Out: Shape> {
    _phantom: std::marker::PhantomData<Out>,
}

impl<Out: Shape> If<Out> {
    #[track_caller]
    pub fn else_if<R: IntoValue<Out = R>>(
        self,
        cond: impl IntoValue<Out = bool>,
        then: impl FnOnce() -> R,
    ) -> Self {
        todo!();
    }

    #[track_caller]
    pub fn else_<R: IntoValue<Out = Out>>(self, then: impl FnOnce() -> R) -> Value<Out> {
        todo!();
    }
}
