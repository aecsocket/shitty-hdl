#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use shitty_hdl::data::Bram;
use shitty_hdl::prelude::*;

#[derive(Debug, Clone, Copy, Shape, Bits)]
pub struct Word(pub Uint<64>);

#[derive(Debug, Clone, Copy, Shape, Bits)]
pub struct Addr(pub Uint<45>);

#[derive(Debug, Clone, Copy, Shape, Bits)]
pub struct PageAddr(pub Uint<40>);

impl PageAddr {
    pub const ZERO: Self = Self(uint!(0));
    pub const INVALID: Self = Self(uint!(:1));
}

impl Addr {
    pub fn from_page(page: impl IntoValue<Out = PageAddr>) -> Value<Self> {
        page.into_value().pack().zext() << 5
    }
}

pub type Heap = Bram<Addr, Word>;

#[derive(Interface)]
struct PageAlloc {
    alloc: Method<(), PageAddr>,
    free: Method<PageAddr, ()>,
}

pub fn page_alloc(heap: Heap) -> PageAlloc {
    let next_clean = Reg::<PageAddr>::new(PageAddr::ZERO);
    let free_head = Reg::<PageAddr>::new(PageAddr::INVALID);
    let free_count = Reg::<PageAddr>::new(PageAddr::ZERO);

    let alloc = Method::define_as(async |()| {
        if free_head == PageAddr::INVALID {
            // bump allocate
            sim_assert!(next_clean != PageAddr::INVALID, "OOM");

            next_clean <<= next_clean + 1;
            next_clean
        } else {
            // reallocate from free list
            sim_assert!(free_count > 0);

            let next_free_head = heap.read(Addr::from_page(free_head)).await;
            let next_free_head = next_free_head.pack().trunc().unpack_to::<PageAddr>();
            free_head <<= next_free_head;
            free_count <<= free_count - 1;
            free_head
        }
    });

    let free = Method::define_as(async |page: Value<PageAddr>| {
        sim_assert!(page != PageAddr::INVALID, "freeing invalid page");
        sim_assert!(free_count < PageAddr::INVALID, "freeing too many pages");

        heap.write(
            Addr::from_page(page),
            free_head.pack().zext().unpack_to::<Word>(),
        )
        .await;
        free_head <= page;
        free_count <= free_count + 1;
    });

    PageAlloc { alloc, free }
}

fn top(heap: Heap) {
    let pages = page_alloc(heap);
}

fn main() {}
