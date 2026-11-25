use crate::{
    module::{In, Out},
    value::Shape,
};
use shitty_hdl_derive::module;

#[module(crate = crate, builder)]
pub fn memory<Addr: Shape, Data: Shape>(
    _depth: u64,
    _read_enable: In<bool>,
    _read_addr: In<Addr>,
    _read_data: Out<Data>,
    _write_enable: In<bool>,
    _write_addr: In<Addr>,
    _write_data: In<Data>,
) {
    todo!();
}
