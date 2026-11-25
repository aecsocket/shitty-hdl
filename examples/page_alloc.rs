use shitty_hdl::prelude::*;

#[derive(Debug, Clone, Copy, Shape)]
pub struct Word(pub Bit<64>);

#[derive(Debug, Clone, Copy, Shape)]
pub struct Addr(pub Bit<45>);

#[derive(Debug, Clone, Copy, Shape)]
pub struct PageAddr(pub Bit<40>);

impl PageAddr {
    pub const ZERO: Self = Self(bit!(0));
    pub const INVALID: Self = Self(bit!(:1));

    pub fn base(self) -> Addr {
        todo!();
    }
}

trait PageAddrValueExt {
    fn base(self) -> Value<Addr>;
}
impl PageAddrValueExt for Value<PageAddr> {
    fn base(self) -> Value<Addr> {
        todo!();
    }
}

pub type Heap = Bram<Addr, Word>;

struct PageAlloc {
    alloc: Action<(), PageAddr>,
    free: Action<PageAddr, ()>,
}

impl Interface for PageAlloc {
    type Define = DefinePageAlloc;
}

struct DefinePageAlloc {
    alloc: Define<(), PageAddr>,
    free: Define<PageAddr, ()>,
}

pub fn page_alloc(heap: Heap) -> <PageAlloc as Interface>::Define {
    let next_clean = Reg::<PageAddr>::new(PageAddr::ZERO);
    let free_head = Reg::<PageAddr>::new(PageAddr::INVALID);
    let free_count = Reg::<PageAddr>::new(PageAddr::ZERO);

    DefinePageAlloc {
        alloc: Action::define_as(async |()| {
            if free_head == PageAddr::INVALID {
                // bump allocate
                sim_assert!(next_clean != PageAddr::INVALID, "OOM");

                next_clean <<= next_clean + 1;
                next_clean
            } else {
                // reallocate from free list
                sim_assert!(free_count > 0);

                let next_free_head = heap.read(free_head.base()).await;
                free_head <<= next_free_head;
                free_count <<= free_count - 1;
                free_head
            }
        }),
        free: Action::define_as(async |page: Value<PageAddr>| {
            sim_assert!(page != PageAddr::INVALID, "freeing invalid page");
            sim_assert!(free_count < PageAddr::INVALID, "freeing too many pages");

            let next_free_head: Value<Word> = Word(zext::<64>(free_head.0));
            heap.write(page.base(), next_free_head);
            free_head <= page;
            free_count <= free_count + 1;
        }),
    }
}

fn main() {}
