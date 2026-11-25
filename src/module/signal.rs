use std::ops::{Deref, DerefMut, ShlAssign};

use crate::{expr::IntoExpr, signal::Signal, value::Shape};

pub struct In<T: Shape>(Signal<T>);
pub struct Out<T: Shape>(Signal<T>);

pub trait SignalFlow {
    type Inverse: SignalFlow;
}
impl<T: Shape> SignalFlow for In<T> {
    type Inverse = Out<T>;
}
impl<T: Shape> SignalFlow for Out<T> {
    type Inverse = In<T>;
}

// read

impl<T: Shape> Deref for In<T> {
    type Target = Signal<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: Shape> Deref for Out<T> {
    type Target = Signal<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Shape> IntoExpr for In<T> {
    type Out = T;
}
impl<T: Shape> IntoExpr for Out<T> {
    type Out = T;
}

// write

impl<T: Shape> DerefMut for Out<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<T: Shape, Rhs: IntoExpr<Out = T>> ShlAssign<Rhs> for Out<T> {
    fn shl_assign(&mut self, rhs: Rhs) {
        self.0 <<= rhs;
    }
}
