use std::marker::PhantomData;

use shitty_hdl_derive::module;

use crate::{In, IntoValue, Out, Shape, Value};

#[module(crate = crate, builder)]
pub fn memory<Data: Shape, Addr: Shape>(depth: u64, _: PhantomData<(Data, Addr)>) {}

impl<Data: Shape, Addr: Shape> memory<Data, Addr> {
    pub fn read_lane(&mut self) -> ReadLane<Data, Addr> {
        todo!();
    }

    pub fn write_lane(&mut self) -> WriteLane<Data, Addr> {
        todo!();
    }
}

pub struct ReadLane<Data: Shape, Addr: Shape> {
    pub enable: Out<bool>,
    pub addr: Out<Addr>,
    pub data: In<Data>,
    _phantom: PhantomData<(Data, Addr)>,
}

impl<Data: Shape, Addr: Shape> ReadLane<Data, Addr> {
    pub async fn read(&self, addr: impl IntoValue<Addr>) -> Value<Data> {
        todo!();
    }
}

pub struct WriteLane<Data: Shape, Addr: Shape> {
    pub enable: Out<bool>,
    pub addr: Out<Addr>,
    pub data: Out<Data>,
    _phantom: PhantomData<(Data, Addr)>,
}

impl<Data: Shape, Addr: Shape> WriteLane<Data, Addr> {
    pub async fn write(&self, addr: impl IntoValue<Addr>, data: impl IntoValue<Data>) {
        todo!();
    }
}
