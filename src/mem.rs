use std::{marker::PhantomData, ops};

use shitty_hdl_derive::module;

use crate::{In, Out, Shape, Val};

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
    pub async fn read(&self, addr: impl Val<Addr>) -> ReadVal<Data> {
        todo!();
    }
}

pub struct ReadVal<Data: Shape> {
    _phantom: PhantomData<Data>,
}

impl<Data: Shape> Val<Data> for ReadVal<Data> {}

impl<Data, Rhs> ops::Add<Rhs> for ReadVal<Data>
where
    Data: Shape,
    Data: ops::Add<Rhs>,
{
    type Output = ();

    fn add(self, rhs: Rhs) -> Self::Output {}
}

//
// impl<Data: Shape, Lhs: Val<Data>> ops::Add<ReadVal<Data>> for Lhs {
//     type Output = ();
//
//     fn add(self, rhs: Data) -> Self::Output {}
// }

pub struct Add<T, Lhs, Rhs> {
    lhs: Lhs,
    rhs: Rhs,
    _phantom: PhantomData<T>,
}

impl<T: Shape, Lhs, Rhs> Val<T> for Add<T, Lhs, Rhs> {}

// crate::val::impl_val!(ReadVal[Data], Data);

pub struct WriteLane<Data: Shape, Addr: Shape> {
    pub enable: Out<bool>,
    pub addr: Out<Addr>,
    pub data: Out<Data>,
    _phantom: PhantomData<(Data, Addr)>,
}

impl<Data: Shape, Addr: Shape> WriteLane<Data, Addr> {
    pub async fn write(&self, addr: impl Val<Addr>, data: impl Val<Data>) {
        todo!();
    }
}
