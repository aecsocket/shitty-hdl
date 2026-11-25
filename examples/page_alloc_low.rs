#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use shitty_hdl::data::Bram;
use shitty_hdl::prelude::*;

#[derive(Debug, Clone, Shape, Bits, Arith)]
pub struct Word(pub Uint<64>);

#[derive(Debug, Clone, Shape, Bits, Arith)]
pub struct Addr(pub Uint<45>);

#[derive(Debug, Clone, Shape, Bits, Arith, BitCmp, ArithCmp)]
pub struct PageAddr(pub Uint<40>);

impl PageAddr {
    pub fn zero() -> Self {
        Self(uint!(0))
    }

    pub fn invalid() -> Self {
        Self(uint!(:1))
    }
}

impl Addr {
    pub fn from_page(page: impl IntoValue<Out = PageAddr>) -> Value<Self> {
        page.repack::<Uint<40>>().shl_zext::<5>().repack::<Self>()
    }
}

pub type Heap = Bram<Addr, Word>;

#[derive(Interface)]
pub struct PageAlloc {
    pub alloc: Method<(), PageAddr>,
    pub free: Method<PageAddr, ()>,
}

#[module]
pub fn page_alloc(mut heap: Heap) -> PageAlloc {
    #[derive(Debug, Shape, BitCmp)]
    enum State {
        Idle,
        AllocWait,
        AllocDone,
        FreeWait,
        FreeDone,
    }

    let mut next_clean = Reg::<PageAddr>::new(PageAddr::zero());
    let mut free_head = Reg::<PageAddr>::new(PageAddr::invalid());
    let mut free_count = Reg::<PageAddr>::new(PageAddr::zero());
    let mut state = Reg::<State>::new(State::Idle);

    // alloc

    let mut alloc_out_r = Reg::<PageAddr>::uninit();
    let alloc_out = Action::when(state == State::AllocDone).define_as(|()| {
        state <<= State::Idle;
        alloc_out_r.read()
    });

    let alloc_req = Action::when(state == State::Idle).define_as(|()| {
        if free_head == PageAddr::invalid() {
            // bump allocate
            sim_assert!(next_clean != PageAddr::invalid(), "OOM");

            alloc_out_r <<= next_clean;
            next_clean <<= next_clean + 1;
            state <<= State::AllocDone;
        } else {
            // reallocate from free list
            sim_assert!(free_count > 0);

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

    let free_out = Action::when(state == State::FreeDone).define_as(|()| {
        state <<= State::Idle;
    });

    let mut free_req_captured = Reg::<PageAddr>::uninit();
    let free_req = Action::when(state == State::Idle).define_as(|page: Value<PageAddr>| {
        sim_assert!(page != PageAddr::invalid(), "freeing invalid page");
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

fn top(heap: Heap) {
    let pages = page_alloc(heap);
}

fn main() {}
