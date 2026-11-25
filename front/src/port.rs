use std::ops::{Deref, DerefMut};

use derive_more::Debug;

use crate::{signal::Signal, value::Shape};

#[derive(Debug)]
pub struct In<'d, T: Shape>(Signal<'d, T>);

#[derive(Debug)]
pub struct Out<'d, T: Shape>(Signal<'d, T>);

// flow

mod sealed {
    pub trait Port {}
}

pub trait Port: sealed::Port {
    type Inverse: Port;
}

impl<'d, T: Shape> sealed::Port for In<'d, T> {}
impl<'d, T: Shape> Port for In<'d, T> {
    type Inverse = Out<'d, T>;
}

impl<'d, T: Shape> sealed::Port for Out<'d, T> {}
impl<'d, T: Shape> Port for Out<'d, T> {
    type Inverse = In<'d, T>;
}

// read

impl<'d, T: Shape> Deref for In<'d, T> {
    type Target = Signal<'d, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// write

impl<'d, T: Shape> Deref for Out<'d, T> {
    type Target = Signal<'d, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Shape> DerefMut for Out<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
