use std::ops::{Deref, DerefMut, ShlAssign};

use crate::{
    expr::{Expr, IntoExpr},
    ir::PortDirection,
    signal::Signal,
    value::Shape,
};

pub struct In<T: Shape>(Signal<T>);
pub struct Out<T: Shape>(Signal<T>);

impl<T: Shape> Clone for In<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Shape> Copy for In<T> {}
impl<T: Shape> Clone for Out<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Shape> Copy for Out<T> {}

pub trait SignalFlow {
    type Inverse: SignalFlow;
    fn new_port(name: &str) -> (Self, Self::Inverse)
    where
        Self: Sized;
}
impl<T: Shape> SignalFlow for In<T> {
    type Inverse = Out<T>;
    #[track_caller]
    fn new_port(name: &str) -> (Self, Out<T>) {
        let signal = Signal::port(name, PortDirection::Input);
        (Self(signal), Out(signal))
    }
}
impl<T: Shape> SignalFlow for Out<T> {
    type Inverse = In<T>;
    #[track_caller]
    fn new_port(name: &str) -> (Self, In<T>) {
        let signal = Signal::port(name, PortDirection::Output);
        (Self(signal), In(signal))
    }
}

impl<T: Shape> Deref for In<T> {
    type Target = Signal<T>;
    fn deref(&self) -> &Signal<T> {
        &self.0
    }
}
impl<T: Shape> Deref for Out<T> {
    type Target = Signal<T>;
    fn deref(&self) -> &Signal<T> {
        &self.0
    }
}
impl<T: Shape> DerefMut for Out<T> {
    fn deref_mut(&mut self) -> &mut Signal<T> {
        &mut self.0
    }
}

impl<T: Shape, Rhs: IntoExpr<Out = T>> ShlAssign<Rhs> for Out<T> {
    #[track_caller]
    fn shl_assign(&mut self, rhs: Rhs) {
        self.0 <<= rhs;
    }
}

impl<T: Shape> From<In<T>> for Signal<T> {
    fn from(port: In<T>) -> Self {
        port.0
    }
}
impl<T: Shape> From<Out<T>> for Signal<T> {
    fn from(port: Out<T>) -> Self {
        port.0
    }
}

impl<T: Shape> IntoExpr for In<T> {
    type Out = T;
    #[track_caller]
    fn into_expr(self) -> Expr<T> {
        self.0.into_expr()
    }
}
impl<T: Shape> IntoExpr for Out<T> {
    type Out = T;
    #[track_caller]
    fn into_expr(self) -> Expr<T> {
        self.0.into_expr()
    }
}
