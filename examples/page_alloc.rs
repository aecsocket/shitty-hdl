use std::{marker::PhantomData, ops::Deref};

// util

pub trait Shape {
    type Value;
}
impl Shape for () {
    type Value = ();
}

pub struct Value<T: Shape>(PhantomData<T>);
pub trait IntoValue {
    type Out;
}
impl<T: Shape> IntoValue for T {
    type Out = T;
}
impl<T: Shape> IntoValue for Value<T> {
    type Out = T;
}

#[derive(Debug, Clone, Copy)]
pub struct Bit<const N: usize>();
impl<const N: usize> Shape for Bit<N> {
    type Value = Value<Self>;
}

pub fn zext<const TO: usize>(v: impl IntoValue) -> Value<Bit<TO>> {
    todo!();
}

macro_rules! bit {
    ($lit:literal) => {
        Bit()
    };
    // it's like `'1` to say "all ones"
    // but we can't use `'` so we have to use an alternative
    (: $lit:literal) => {
        Bit()
    };
}

macro_rules! sim_assert {
    ($expr:expr) => {};
    ($expr:expr, $msg:literal) => {};
}

pub struct U<const N: usize>();
impl<const N: usize> Shape for U<N> {
    type Value = Value<Self>;
}

macro_rules! u {
    ($lit:literal) => {
        U()
    };
    // it's like `'1` to say "all ones"
    // but we can't use `'` so we have to use an alternative
    (: $lit:literal) => {
        U()
    };
}

pub struct Action<In, Out>(PhantomData<(In, Out)>);
pub struct Define<In, Out>(PhantomData<(In, Out)>);

impl<In: Shape, Out: Shape> Action<In, Out> {
    pub fn define_as(f: impl AsyncFnOnce(In::Value) -> Out::Value) -> Define<In, Out> {
        todo!();
    }
}

pub trait Interface {
    type Define;
}

pub struct Reg<T: Shape>(Value<T>);

impl<T: Shape> Reg<T> {
    pub fn new(init: T) -> Self {
        todo!();
    }
}
impl<T: Shape> Deref for Reg<T> {
    type Target = Value<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

async fn join<A, B>((a, b): (impl Future<Output = A>, impl Future<Output = B>)) -> (A, B) {
    todo!();
}

pub struct Bram<Addr: Shape, Data: Shape> {
    _phantom: PhantomData<(Addr, Data)>,
    // read_req: Action<Addr, ()>,
    // read_out: Action<(), Data>,
    // req_write: Action<(Addr, Data), ()>,
}

impl<Addr: Shape, Data: Shape> Bram<Addr, Data> {
    pub async fn read(&self, addr: impl IntoValue<Out = Addr>) -> Value<Data> {
        todo!();
    }

    pub async fn write(&self, addr: impl IntoValue<Out = Addr>, data: impl IntoValue<Out = Data>) {
        todo!();
    }

    // pub fn read_req(&self, addr: Addr) {}
    // pub fn read_out(&self) -> Value<Data> {
    //   todo!();
    // }
}

// logic

pub struct Word(pub Bit<64>);
impl Shape for Word {
    type Value = Value<Self>;
}

pub struct Addr(pub Bit<45>);
impl Shape for Addr {
    type Value = Value<Self>;
}

#[derive(Debug, Clone, Copy)]
pub struct PageAddr(pub Bit<40>);
impl Shape for PageAddr {
    type Value = Value<Self>;
}
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
