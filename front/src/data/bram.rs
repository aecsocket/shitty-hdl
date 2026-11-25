use derive::Interface;

use crate::{domain::Clock, elab, method::Method, ops::Bits, value::Shape};

#[derive(Interface)]
#[interface(crate = crate)]
pub struct Bram<Addr: Shape, Data: Shape> {
    pub read: Method<Addr, Data>,
    pub write: Method<(Addr, Data), ()>,
}

#[bon::builder(crate = ::bon)]
pub fn bram<Addr: Bits, Data: Bits>(clk: Clock, depth: u64) -> Bram<Addr, Data> {
    elab::bram(clk, depth)
}
