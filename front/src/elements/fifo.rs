use derive::Interface;

use crate::{domain::Domain, method::Method, value::Shape};

#[derive(Interface)]
#[interface(crate = crate)]
pub struct Fifo<D: Domain, T: Shape> {
    pub push: Method<D, T, ()>,
    pub pop: Method<D, (), T>,
}
