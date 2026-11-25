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

#[derive(Debug, Clone, Shape, Arith)]
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

/// Walks a linked list of pages in the heap, ending when a cell holds the
/// invalid marker. Mirrors the standalone-helper shape: a hardware `while`
/// with awaits and a `break` out of an `if`/`else`.
#[hdl]
async fn chase(heap: Heap, cursor: Reg<Sync, PageAddr>) -> Value<Sync, PageAddr> {
    let mut current = cursor;
    while current.read() != PageAddr::invalid() {
        let next = heap.read(Addr::from_page(current.read())).await;
        let next_page = next.repack_trunc::<PageAddr>();
        if next_page == PageAddr::invalid() {
            break;
        } else {
            current <<= next_page;
        }
    }
    current.read()
}

#[module]
fn top(platform: Platform) {
    let clk = platform.clock::<Sync>();
    let mut cursor = Reg::new(clk, PageAddr::zero());
    let heap = bram::<Sync, Addr, Word>().clk(clk).depth(256).call();

    Rule::define_as(async || {
        // Linked list: page 0 -> 1 -> 2 -> invalid.
        heap.write(
            Addr::from_page(PageAddr::zero()),
            PageAddr(u!(1)).into_value().repack_zext::<Word>(),
        )
        .await;
        heap.write(
            Addr::from_page(PageAddr(u!(1)).into_value()),
            PageAddr(u!(2)).into_value().repack_zext::<Word>(),
        )
        .await;
        heap.write(
            Addr::from_page(PageAddr(u!(2)).into_value()),
            PageAddr::invalid().into_value().repack_zext::<Word>(),
        )
        .await;

        cursor <<= PageAddr::zero();
        let end = chase(heap, cursor).await;
        displayln!("chase ended at {end:x}");
        sim_finish();
    });
}

fn main() {
    shitty_hdl::cli!(A7_100T, top);
}
