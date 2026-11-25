#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use shitty_hdl::prelude::*;

#[derive(Clone, Shape)]
// #[shape(add, sub, mul, div)]
pub struct Word(pub Bits<64>);

#[derive(Clone, Shape)]
// #[shape(add, sub)]
pub struct Addr(pub Uint<45>);

#[derive(Clone, Copy, Shape)]
pub enum Tag {
    Wire = 1,
    Graft = 2,
    Erase = 3,
    ExtVal = 4,
    Comb = 5,
    ExtFn = 6,
}

#[derive(Clone, Copy, Shape)]
pub struct Label(pub Bits<16>);

#[derive(Clone, Copy, Shape)]
pub struct Port {
    pub tag: Tag,
    pub payload: Bits<45>,
    pub label: Label,
}

#[module(builder)]
pub fn heap(
    capacity: u64,
    do_read: In<bool>,
    read_addr: In<Addr>,
    mut read_data: Out<Word>,
    do_write: In<bool>,
    write_addr: In<Addr>,
    write_data: In<Word>,
    mut peak_addr: Out<Addr>,
) {
    let mut mem = memory::<Word, Addr>().depth(capacity).call();
    let mut read = mem.read_lane();
    let mut write = mem.write_lane();

    comb(|| {
        read.enable <<= do_read;
        read.addr <<= read_addr;
        read_data <<= read.data;
        write.enable <<= do_write;
        write.addr <<= write_addr;
        write.data <<= write_data;
    });

    if read_addr.ge(peak_addr) {
        sync(|| peak_addr <<= read_addr);
    }
}

impl heap {
    pub async fn read(&mut self, addr: impl IntoExpr<Out = Addr>) -> Value<Word> {
        let read_data = capture(self.read_data);
        comb(|| {
            self.do_read <<= true;
            self.read_addr <<= addr;
        });
        read_data
    }

    pub async fn write(
        &mut self,
        addr: impl IntoExpr<Out = Addr>,
        data: impl IntoExpr<Out = Word>,
    ) {
        comb(|| {
            self.do_write <<= true;
            self.write_addr <<= addr;
            self.write_data <<= data;
        });
    }
}

#[module]
pub async fn top() {
    let mut heap = heap().capacity(8).call();
    let mut iters = Signal::<Uint<32>>::new(0);

    let x = heap.read(Addr(4)).await;
    let y = heap.read(Addr(5)).await;
    let z = x + y;
    heap.write(Addr(6), z).await;

    sync(|| iters <<= iters + 1);
}

fn main() {
    let simulator = Simulator::new(top);
}
