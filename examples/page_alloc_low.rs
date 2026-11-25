#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use board::arty_a7::A7_100T;
use front::elements::{Bram, bram};
use platform::Platform;
use shitty_hdl::prelude::*;

#[derive(Domain)]
pub struct Sync;

#[derive(Debug, Clone, Shape, Arith)]
pub struct Word(pub U<64>);

#[derive(Debug, Clone, Shape, Arith)]
pub struct Addr(pub U<45>);

#[derive(Debug, Clone, Shape, Arith, BitCmp)]
pub struct PageAddr(pub U<40>);

impl PageAddr {
    #[must_use]
    pub fn zero() -> Self {
        Self(u!(0))
    }

    #[must_use]
    pub fn invalid() -> Self {
        Self(u!(:1))
    }
}

impl Addr {
    pub fn from_page<D: Domain>(page: impl IntoValue<D, PageAddr>) -> Value<D, Self> {
        page.repack::<U<40>>().shl_zext::<5>().repack::<Self>()
    }
}

pub type Heap = Bram<Sync, Addr, Word>;

#[derive(Interface)]
pub struct PageAlloc<D: Domain> {
    pub alloc: Method<D, (), PageAddr>,
    pub free: Method<D, PageAddr, ()>,
}

#[module]
pub fn page_alloc(clk: Clock<Sync>, mut heap: Heap) -> PageAlloc<Sync> {
    #[derive(Debug, Shape)]
    enum State {
        Idle,
        AllocWait,
        AllocDone,
        FreeWait,
        FreeDone,
    }

    let mut next_clean = Reg::new(clk, PageAddr::zero());
    let mut free_head = Reg::new(clk, PageAddr::invalid());
    let mut free_count = Reg::new(clk, PageAddr::zero());
    let mut state = Reg::new(clk, State::Idle);

    // alloc

    let mut alloc_out_r = Reg::uninit(clk);
    let alloc_out: Action<Sync, (), PageAddr> =
        Action::when(state == State::AllocDone).define_as(|()| {
            state <<= State::Idle;
            alloc_out_r.read()
        });

    let alloc_req: Action<Sync, (), ()> = Action::when(state == State::Idle).define_as(|()| {
        if free_head == PageAddr::invalid() {
            // bump allocate
            sim_assert!(next_clean != PageAddr::invalid(), "OOM");
            displayln!("allocating page {next_clean:x}");

            alloc_out_r <<= next_clean;
            next_clean <<= next_clean + 1;
            state <<= State::AllocDone;
        } else {
            // reallocate from free list
            sim_assert!(free_count > 0u64);

            heap.read.req.call(Addr::from_page(free_head));
            state <<= State::AllocWait;
        };
    });

    Rule::when(state == State::AllocWait).define_as(|| {
        alloc_out_r <<= free_head.read();
        state <<= State::AllocDone;

        let next_free_head = heap.read.out.call(());
        free_head <<= next_free_head.repack_trunc::<PageAddr>();
        free_count <<= free_count - 1;
    });

    // free

    let free_out: Action<Sync, (), ()> = Action::when(state == State::FreeDone).define_as(|()| {
        state <<= State::Idle;
    });

    let mut free_req_captured = Reg::uninit(clk);
    let free_req: Action<Sync, PageAddr, ()> =
        Action::when(state == State::Idle).define_as(|page: Value<Sync, PageAddr>| {
            sim_assert!(page != PageAddr::invalid(), "freeing invalid page");
            displayln!("freeing page {:x}", page);
            sim_assert!(free_count < PageAddr::invalid(), "freeing too many pages");

            heap.write
                .req
                .call((Addr::from_page(page), free_head.repack_zext::<Word>()));
            free_req_captured <<= page;
            state <<= State::FreeWait;
        });

    Rule::when(state == State::FreeWait).define_as(|| {
        heap.write.out.call(());
        free_head <<= free_req_captured;
        free_count <<= free_count + 1;
        state <<= State::FreeDone;
    });

    PageAlloc {
        alloc: Method::new(alloc_req, alloc_out),
        free: Method::new(free_req, free_out),
    }
}

#[module]
fn top(platform: Platform) {
    #[derive(Debug, Shape)]
    enum State {
        Idle,
        Alloc1Wait,
        Alloc1Done,
        Alloc2Wait,
        Alloc2Done,
        Alloc3Wait,
        Alloc3Done,
        Free2Wait,
        Free2Done,
        Free1Wait,
        Free1Done,
        Free3Wait,
        Free3Done,
    }

    let clk = platform.clock::<Sync>();
    let mut led = platform.led.take(0).output(clk);

    let heap = bram::<Sync, Addr, Word>().clk(clk).depth(32 * 1024).call();
    let mut pages = page_alloc(clk, heap);
    let mut state = Reg::new(clk, State::Idle);
    let mut page1 = Reg::uninit(clk);
    let mut page2 = Reg::uninit(clk);
    let mut page3 = Reg::uninit(clk);

    Rule::when(state == State::Idle).define_as(|| {
        displayln!("idle!");
        pages.alloc.req.call(());
        state <<= State::Alloc1Wait;
    });

    Rule::when(state == State::Alloc1Wait).define_as(|| {
        page1 <<= pages.alloc.out.call(());
        displayln!("page1 = {page1}");
        state <<= State::Alloc1Done;
    });

    Rule::when(state == State::Alloc1Done).define_as(|| {
        pages.alloc.req.call(());
        state <<= State::Alloc2Wait;
    });

    Rule::when(state == State::Alloc2Wait).define_as(|| {
        page2 <<= pages.alloc.out.call(());
        displayln!("page2 = {page2}");
        state <<= State::Alloc2Done;
    });

    Rule::when(state == State::Alloc2Done).define_as(|| {
        pages.alloc.req.call(());
        state <<= State::Alloc3Wait;
    });

    Rule::when(state == State::Alloc3Wait).define_as(|| {
        page3 <<= pages.alloc.out.call(());
        displayln!("page3 = {page3}");
        state <<= State::Alloc3Done;
        led <<= !led;
    });

    Rule::when(state == State::Alloc3Done).define_as(|| {
        pages.free.req.call(page2);
        state <<= State::Free2Wait;
    });

    Rule::when(state == State::Free2Wait).define_as(|| {
        pages.free.out.call(());
        state <<= State::Free2Done;
    });

    Rule::when(state == State::Free2Done).define_as(|| {
        pages.free.req.call(page1);
        state <<= State::Free1Wait;
    });

    Rule::when(state == State::Free1Wait).define_as(|| {
        pages.free.out.call(());
        state <<= State::Free1Done;
    });

    Rule::when(state == State::Free1Done).define_as(|| {
        pages.free.req.call(page3);
        state <<= State::Free3Wait;
    });

    Rule::when(state == State::Free3Wait).define_as(|| {
        pages.free.out.call(());
        state <<= State::Free3Done;
    });

    Rule::when(state == State::Free3Done).define_as(|| {
        state <<= State::Idle;
    });
}

fn main() {
    shitty_hdl::cli!(A7_100T, top);
}
