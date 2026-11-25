use derive::Interface;

use crate::{method::Method, value::Shape};

#[derive(Interface)]
#[interface(crate = crate)]
pub struct Bram<Addr: Shape, Data: Shape> {
    pub read: Method<Addr, Data>,
    pub write: Method<(Addr, Data), ()>,
}
