use std::ops::{Deref, DerefMut};

use derive_more::Debug;

use crate::{signal::Signal, value::Shape};

#[derive(Debug)]
pub struct In<T: Shape>(Signal<T>);

#[derive(Debug)]
pub struct Out<T: Shape>(Signal<T>);

// flow

mod sealed {
    pub trait Port {}
}

pub trait Port: sealed::Port {
    type Inverse: Port;
}

impl<T: Shape> sealed::Port for In<T> {}
impl<T: Shape> Port for In<T> {
    type Inverse = Out<T>;
}

impl<T: Shape> sealed::Port for Out<T> {}
impl<T: Shape> Port for Out<T> {
    type Inverse = In<T>;
}

// read

impl<T: Shape> Deref for In<T> {
    type Target = Signal<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// write

impl<T: Shape> Deref for Out<T> {
    type Target = Signal<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Shape> DerefMut for Out<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
