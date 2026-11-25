use board::arty_a7::A7_100T;
use shitty_hdl::{elements::Bram, elements::bram, prelude::*};

#[derive(Domain)]
pub struct Clk;

fn main() {
    shitty_hdl::cli!(A7_100T, top);
}

#[derive(Interface)]
pub struct Writeback<D: Domain, Addr: Bits + BitEq, Data: Bits> {
    pub read: Action<D, (), Data>,
    pub write: Action<D, Data, ()>,
    pub get_addr: Action<D, (), Addr>,
    pub is_valid: Action<D, (), bool>,
    pub set_addr: Method<D, Addr, ()>,
    pub evict: Action<D, (), ()>,
}

#[module]
pub fn writeback<D: Domain, Addr: Bits + BitEq, Data: Bits>(
    clk: Clock<D>,
    bram: Bram<D, Addr, Data>,
) -> Writeback<D, Addr, Data> {
    let mut addr = Reg::<_, Addr>::uninit(clk);
    let mut cached = Reg::<_, Data>::uninit(clk);
    let mut valid = Reg::new(clk, false);

    let read = Action::define_as(|()| {
        sim_assert!(valid, "read from invalid write-back cache line");
        *cached
    });
    let write = Action::define_as(|data| {
        sim_assert!(valid, "write to invalid write-back cache line");
        cached <<= data;
    });
    let get_addr = Action::define_as(|()| *addr);
    let is_valid = Action::define_as(|()| *valid);

    let set_addr = Method::define_as(clk, async |next_addr| {
        if valid {
            bram.write(addr, cached).await;
        }
        addr <<= next_addr;
        let next_data = bram.read(next_addr).await;
        cached <<= next_data;
        valid <<= true;
    });

    let evict = Action::define_as(|()| valid <<= false);

    Writeback {
        read,
        write,
        get_addr,
        is_valid,
        set_addr,
        evict,
    }
}

#[module]
pub fn top(platform: Platform) {
    let clk = platform.clock::<Clk>();
    let bram = bram::<Clk, U<4>, U<8>>().clk(clk).depth(16).call();
    let mut cache = writeback(clk, bram);

    Rule::define_as(async || {
        displayln!("-- set_addr(0): load line --");
        cache.set_addr(u!(0)).await;
        displayln!("valid = {}", cache.is_valid.call(()));
        displayln!("initial data = {}", cache.read.call(()));

        displayln!("-- write 42 --");
        cache.write.call(u!(42));
        displayln!("data = {}", cache.read.call(()));

        displayln!("-- set_addr(1): evict, writing 42 back to address 0 --");
        cache.set_addr(u!(1)).await;
        displayln!("valid = {}", cache.is_valid.call(()));

        displayln!("-- set_addr(0): reload --");
        cache.set_addr(u!(0)).await;
        displayln!("data = {} (write-back preserved it)", cache.read.call(()));

        sim_finish();
    });
}
