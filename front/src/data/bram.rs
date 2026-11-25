use derive::{Interface, module};

use crate::{method::Method, value::Shape};

#[derive(Interface)]
#[interface(crate = crate)]
pub struct Bram<Addr: Shape, Data: Shape> {
    pub read: Method<Addr, Data>,
    pub write: Method<(Addr, Data), ()>,
}

#[module(crate = crate, builder)]
pub fn bram<Addr: Shape, Data: Shape>(depth: u64) -> Bram<Addr, Data> {
    todo!();
}
