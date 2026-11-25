use derive::Interface;

use crate::{
    domain::{Clock, Domain},
    elab,
    method::Method,
    ops::Bits,
};

#[derive(Interface)]
#[interface(crate = crate)]
pub struct Bram<D: Domain, Addr: Bits, Data: Bits> {
    pub read: Method<D, Addr, Data>,
    pub write: Method<D, (Addr, Data), ()>,
}

#[bon::builder]
pub fn bram<D: Domain, Addr: Bits, Data: Bits>(clk: Clock<D>, depth: u64) -> Bram<D, Addr, Data> {
    elab::bram(clk, depth)
}
