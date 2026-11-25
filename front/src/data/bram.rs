use derive::Interface;

use crate::{
    domain::{Clock, Domain},
    elab,
    method::Method,
    ops::Bits,
    value::Shape,
};

#[derive(Interface)]
#[interface(crate = crate)]
pub struct Bram<Addr: Shape, Data: Shape> {
    pub read: Method<Addr, Data>,
    pub write: Method<(Addr, Data), ()>,
}

#[bon::builder]
pub fn bram<D: Domain, Addr: Bits, Data: Bits>(clk: Clock<D>, depth: u64) -> Bram<Addr, Data> {
    elab::bram(clk, depth)
}
