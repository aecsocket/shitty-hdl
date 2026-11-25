use std::{marker::PhantomData, ops};

use crate::{Cond, IntoStmts, Shape, Signal, Stmt, Val};

pub trait Module {}

pub fn submodule<M: Module>() -> M {
    todo!();
}

pub fn capture<T: Shape>(value: impl Val<T>) -> Capture<T> {
    todo!();
}
impl<T: Shape> Val<T> for Capture<T> {}

pub struct Capture<T: Shape> {
    _phantom: PhantomData<T>,
}

pub fn if_(cond: impl Cond, then: impl FnMut()) {}

pub fn eq<T: Shape>(a: impl Val<T>, b: impl Val<T>) -> impl Cond {
    todo!();
    false
}

pub fn ne<T: Shape>(a: impl Val<T>, b: impl Val<T>) -> impl Cond {
    todo!();
    false
}

pub fn ge<T: Shape>(a: impl Val<T>, b: impl Val<T>) -> impl Cond {
    todo!();
    false
}

pub fn gt<T: Shape>(a: impl Val<T>, b: impl Val<T>) -> impl Cond {
    todo!();
    false
}

pub fn le<T: Shape>(a: impl Val<T>, b: impl Val<T>) -> impl Cond {
    todo!();
    false
}

pub fn lt<T: Shape>(a: impl Val<T>, b: impl Val<T>) -> impl Cond {
    todo!();
    false
}

pub trait SignalFlow {
    type Inverse: SignalFlow;
}

#[derive(Clone, Copy)]
pub struct In<T: Shape>(Signal<T>);
#[derive(Clone, Copy)]
pub struct Out<T: Shape>(Signal<T>);

impl<T: Shape> SignalFlow for In<T> {
    type Inverse = Out<T>;
}
impl<T: Shape> SignalFlow for Out<T> {
    type Inverse = In<T>;
}

impl<T: Shape> Val<T> for In<T> {}
impl<T: Shape> Val<T> for Out<T> {}

impl<T: Shape> Out<T> {
    pub fn eq(&self, rhs: impl Val<T>) -> impl Cond {
        self.0.eq(rhs)
    }

    pub fn set(&self, rhs: impl Val<T>) -> Stmt {
        self.0.set(rhs)
    }
}

impl<T: Shape, Rhs: Val<T>> ops::ShlAssign<Rhs> for Out<T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        todo!();
    }
}

impl<T: Shape> In<T> {
    pub fn eq(&self, rhs: impl Val<T>) -> impl Cond {
        self.0.eq(rhs)
    }
}
