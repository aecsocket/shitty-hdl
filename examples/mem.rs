use shitty_hdl::{In, Out, Shape, Val, capture, memory, module, submodule};

#[derive(Clone, Copy)]
pub struct Word(pub u64);
impl Shape for Word {}

#[derive(Clone, Copy)]
pub struct Addr(pub u32);
impl Shape for Addr {}

#[module]
pub fn heap(
    do_read: In<bool>,
    read_addr: In<Addr>,
    mut read_data: Out<Word>,
    do_write: In<bool>,
    write_addr: In<Addr>,
    write_data: In<Word>,
    mut peak_addr: Out<Addr>,
) {
    let mut mem = submodule::<memory<Word, Addr>>();
    let mut read = mem.read_port();
    let mut write = mem.write_port();

    read.enable <<= do_read;
    read.addr <<= read_addr;
    read_data <<= read.data;
    write.enable <<= do_write;
    write.addr <<= write_addr;
    write.data <<= write_data;

    if read_addr > peak_addr {
        peak_addr <<= read_addr;
    }
}

impl heap {
    pub async fn read(&mut self, addr: impl Val<Addr>) -> impl Val<Word> {
        let read_data = capture(self.read_data);
        self.do_read <<= true;
        self.read_addr <<= addr;
        read_data
    }

    pub async fn write(&mut self, addr: impl Val<Addr>, data: impl Val<Word>) {
        self.do_write <<= true;
        self.write_addr <<= addr;
        self.write_data <<= data;
    }
}

#[module]
pub async fn foo() {
    let heap = submodule::<heap>();

    let x = heap.read(Addr(4)).await;
    let y = heap.read(Addr(5)).await;
    let z = x + y;
    heap.write(Addr(6), z).await;
}

fn main() {}
