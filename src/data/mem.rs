use crate::{
    module::{In, Out},
    value::Shape,
};
use shitty_hdl_derive::module;

#[module(crate = crate, builder)]
pub fn memory<Addr: Shape, Data: Shape>(
    depth: u64,
    read_enable: In<bool>,
    read_addr: In<Addr>,
    read_data: Out<Data>,
    write_enable: In<bool>,
    write_addr: In<Addr>,
    write_data: In<Data>,
) {
}
