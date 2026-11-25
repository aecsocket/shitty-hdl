#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use shitty_hdl::data::Bram;
use shitty_hdl::prelude::*;

#[derive(Debug, Clone, Shape, Bits, Arith)]
pub struct Word(pub Uint<64>);

#[derive(Debug, Clone, Shape, Bits, Arith)]
pub struct Addr(pub Uint<45>);

#[derive(Debug, Clone, Shape, Bits, Arith)]
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
        page.into_value()
            .repack::<Uint<40>>()
            .shl_zext::<5>()
            .repack::<Self>()
    }
}

pub type Heap = Bram<Addr, Word>;

#[derive(Interface)]
struct PageAlloc {
    alloc: Method<(), PageAddr>,
    free: Method<PageAddr, ()>,
}

pub fn page_alloc(heap: Heap) -> PageAlloc {
    let next_clean = Reg::<PageAddr>::new(PageAddr::zero());
    let free_head = Reg::<PageAddr>::new(PageAddr::invalid());
    let free_count = Reg::<PageAddr>::new(PageAddr::zero());

    let alloc = Method::define_as(async |()| {
        if free_head == PageAddr::invalid() {
            // bump allocate
            sim_assert!(next_clean != PageAddr::invalid(), "OOM");

            next_clean <<= next_clean + 1;
            next_clean
        } else {
            // reallocate from free list
            sim_assert!(free_count > 0);

            let next_free_head = heap.read(Addr::from_page(free_head)).await;
            free_head <<= next_free_head.repack_trunc::<PageAddr>();
            free_count <<= (free_count.repack() - 1).repack();
            free_head
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

fn top(heap: Heap) {
    let pages = page_alloc(heap);
}

fn main() {}
