use board::arty_a7::A7_100T;
use shitty_hdl::{elements::Bram, elements::bram, prelude::*};

#[derive(Domain)]
pub struct Clk;

fn main() {
    shitty_hdl::cli!(A7_100T, top);
}

#[derive(Interface)]
pub struct WriteBack<D: Domain, Addr: Bits, Data: Bits> {
    pub read: Action<D, (), Data>,
    pub write: Action<D, Data, ()>,
    pub get_addr: Action<D, (), Addr>,
    pub set_addr: Method<D, Addr, ()>,
}

#[module]
pub fn write_back<D: Domain, Addr: Bits, Data: Bits>(
    clk: Clock<D>,
    bram: Bram<D, Addr, Data>,
) -> WriteBack<D, Addr, Data> {
    let mut addr = Reg::<_, Addr>::uninit(clk);
    let mut cached = Reg::<_, Data>::uninit(clk);

    let read = Action::define_as(|()| *cached);
    let write = Action::define_as(|data| cached <<= data);
    let get_addr = Action::define_as(|()| *addr);

    let set_addr = Method::define_as(clk, async |next: Value<D, Addr>| {
        addr <<= next;
        let data = bram.read(next).await;
        cached <<= data;
    });

    WriteBack {
        read,
        write,
        get_addr,
        set_addr,
    }
}

#[module]
pub fn top(platform: Platform) {
    let clk = platform.clock::<Clk>();
    let bram = bram::<Clk, U<10>, U<32>>().clk(clk).depth(1024).call();
    let _cache = write_back(clk, bram);
}
