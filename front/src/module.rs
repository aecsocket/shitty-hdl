use std::marker::PhantomData;

use crate::value::{IntoValue, Shape, Value};

pub trait Interface {}

pub struct Rule(());

impl Rule {
    pub fn when(cond: impl IntoValue<Out = bool>) -> DefineRule {
        todo!();
    }

    pub fn define_as(f: impl FnOnce()) {
        todo!();
    }
}

pub struct DefineRule(());

impl DefineRule {
    pub fn define_as(f: impl FnOnce()) {
        todo!();
    }
}

pub struct Action<In: Shape, Out: Shape>(PhantomData<(In, Out)>);

impl<In: Shape, Out: Shape> Action<In, Out> {
    pub fn when(cond: impl IntoValue<Out = bool>) -> DefineAction<In, Out> {
        todo!();
    }

    pub fn define_as(f: impl FnOnce(In::Values) -> Out::Values) -> Self {
        drop(f);
        todo!();
    }

    pub fn call(&mut self, args: In::Values) -> Out::Values {
        todo!();
    }
}

pub struct DefineAction<In: Shape, Out: Shape>(PhantomData<(In, Out)>);

impl<In: Shape, Out: Shape> DefineAction<In, Out> {
    pub fn define_as(self, f: impl FnOnce(In::Values) -> Out::Values) -> Action<In, Out> {
        drop(f);
        todo!();
    }
}

pub fn if_<R: IntoValue>(cond: impl IntoValue<Out = bool>, then: impl AsyncFnOnce() -> R) -> If<R> {
    drop((cond, then));
    todo!();
}

pub struct If<T> {
    _phantom: PhantomData<T>,
}

impl<T: IntoValue> If<T> {
    #[must_use]
    pub fn else_if(self, cond: impl IntoValue<Out = bool>, then: impl AsyncFnOnce() -> T) -> Self {
        drop((cond, then));
        todo!();
    }

    pub fn else_(self, then: impl AsyncFnOnce() -> T) -> <T as IntoValue>::Out {
        drop(then);
        todo!();
    }
}
