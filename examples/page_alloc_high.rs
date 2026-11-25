#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use front::data::Bram;
use shitty_hdl::prelude::*;

#[derive(Debug, Clone, Shape, Bits, Arith)]
pub struct Word(pub Uint<64>);

#[derive(Debug, Clone, Shape, Bits, Arith)]
pub struct Addr(pub Uint<45>);

#[derive(Debug, Clone, Shape, Bits, Arith, BitCmp, ArithCmp)]
pub struct PageAddr(pub Uint<40>);

impl PageAddr {
    #[must_use]
    pub fn zero() -> Self {
        Self(uint!(0))
    }

    #[must_use]
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
pub fn page_alloc(clk: Clock, heap: Heap) -> PageAlloc {
    let mut next_clean = Reg::<PageAddr>::new(clk, PageAddr::zero());
    let mut free_head = Reg::<PageAddr>::new(clk, PageAddr::invalid());
    let mut free_count = Reg::<PageAddr>::new(clk, PageAddr::zero());

    let alloc = Method::define_as(async |()| {
        if free_head == PageAddr::invalid() {
            // bump allocate
            sim_assert!(next_clean != PageAddr::invalid(), "OOM");

            next_clean <<= next_clean + 1;
            next_clean.read()
        } else {
            // reallocate from free list
            sim_assert!(free_count > 0);

            let next_free_head = heap.read(Addr::from_page(free_head)).await;
            free_head <<= next_free_head.repack_trunc::<PageAddr>();
            free_count <<= free_count - 1;
            free_head.read()
        }
    });

    let free = Method::define_as(async |page: Value<PageAddr>| {
        sim_assert!(page != PageAddr::invalid(), "freeing invalid page");
        sim_assert!(free_count < PageAddr::invalid(), "freeing too many pages");

        heap.write(Addr::from_page(page), free_head.repack_zext::<Word>())
            .await;
        free_head <<= page;
        free_count <<= free_count + 1;
    });

    PageAlloc { alloc, free }
}

fn main() {}
