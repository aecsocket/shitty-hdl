use shitty_hdl::{
    Capture, CmpExt, In, IntoValue, Out, Shape, Signal, capture, comb, data::mem::memory, module,
    sync,
};

#[derive(Clone, Copy, Shape)]
pub struct Word(pub u64);

#[derive(Clone, Copy, Shape)]
pub struct Addr(pub u32);

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
    let mut mem = memory::<Word, Addr>().depth(8).call();
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
        peak_addr <<= read_addr;
    }
}

impl heap {
    pub async fn read(&mut self, addr: impl IntoValue<Addr>) -> Capture<Word> {
        let read_data = capture(self.read_data);
        comb(|| {
            self.do_read <<= true;
            self.read_addr <<= addr;
        });
        read_data
    }

    pub async fn write(&mut self, addr: impl IntoValue<Addr>, data: impl IntoValue<Word>) {
        comb(|| {
            self.do_write <<= true;
            self.write_addr <<= addr;
            self.write_data <<= data;
        });
    }
}

#[module]
pub async fn top() {
    let heap = heap();
    let iters = Signal::<u32>::new(0);

    let x = heap.read(Addr(4)).await;
    let y = heap.read(Addr(5)).await;
    let z = x + y;
    heap.write(Addr(6), z).await;

    sync(|| iters <<= iters + 1);
}

fn main() {}
