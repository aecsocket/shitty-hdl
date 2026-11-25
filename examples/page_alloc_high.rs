#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

use board::arty_a7::A7_100T;
use front::elements::{Bram, bram};
use platform::Platform;
use shitty_hdl::prelude::*;

#[derive(Domain)]
pub struct Sync;

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
    pub fn from_page<D: Domain>(page: impl IntoValue<D, Out = PageAddr>) -> Value<D, Self> {
        page.repack::<Uint<40>>().shl_zext::<5>().repack::<Self>()
    }
}

pub type Heap = Bram<Sync, Addr, Word>;

#[derive(Interface)]
pub struct PageAlloc<D: Domain> {
    pub alloc: Method<D, (), PageAddr>,
    pub free: Method<D, PageAddr, ()>,
}

#[module]
pub fn page_alloc(clk: Clock<Sync>, heap: Heap) -> PageAlloc<Sync> {
    let mut next_clean = Reg::new(clk, PageAddr::zero());
    let mut free_head = Reg::new(clk, PageAddr::invalid());
    let mut free_count = Reg::new(clk, PageAddr::zero());

    let alloc = Method::define_as(clk, async |()| {
        if free_head == PageAddr::invalid() {
            // bump allocate
            sim_assert!(next_clean != PageAddr::invalid(), "OOM");
            displayln!("allocating page {next_clean:x}");

            next_clean <<= next_clean + 1;
            next_clean.read()
        } else {
            // reallocate from free list
            sim_assert!(free_count > 0u64);
            displayln!("reallocating from free list");

            let next_free_head = heap.read(Addr::from_page(free_head)).await;
            free_head <<= next_free_head.repack_trunc::<PageAddr>();
            free_count <<= free_count - 1;
            free_head.read()
        }
    });

    let free = Method::define_as(clk, async |page: Value<Sync, PageAddr>| {
        sim_assert!(page != PageAddr::invalid(), "freeing invalid page");
        sim_assert!(free_count < PageAddr::invalid(), "freeing too many pages");
        displayln!("freeing page {page:x}");

        heap.write(Addr::from_page(page), free_head.repack_zext::<Word>())
            .await;
        free_head <<= page;
        free_count <<= free_count + 1;
    });

    PageAlloc { alloc, free }
}

#[module]
fn top(platform: Platform) {
    let clk = platform.clock::<Sync>().clock;
    let mut led = platform.led.take(0).output(clk);

    let heap = bram::<Sync, Addr, Word>().clk(clk).depth(32 * 1024).call();
    let pages = page_alloc(clk, heap);

    Rule::define_as(async || {
        displayln!("idle!");

        let page1 = pages.alloc().await;
        displayln!("page1 = {page1}");
        led <<= !led;
        pages.free(page1).await;

        let page2 = pages.alloc().await;
        displayln!("page2 = {page2}");
        led <<= !led;
        pages.free(page2).await;

        displayln!("done!");
        sim_finish();
    });
}

fn main() {
    shitty_hdl::cli!(A7_100T, top);
}
