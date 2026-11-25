mod core;
pub mod data;

pub use core::*;
pub use shitty_hdl_derive::{Shape, module};

pub mod prelude {
    pub use crate::{CmpExt, In, Out, Shape, Signal, Value, comb, module, sync};
}

// re-exports for macros
#[doc(hidden)]
pub mod __ {
    pub use crate::{Module, Shape, SignalFlow, if_};
    pub use bon;
}

//
// impl Module {
//     #[expect(non_snake_case)]
//     pub fn If(&mut self, cond: impl Cond, then: impl Fn(&mut Self)) {}
// }
//
//
// impl<Rhs: Stmts> ops::AddAssign<Rhs> for Domain {
//     fn add_assign(&mut self, rhs: Rhs) {}
// }
//
// pub trait Set<Rhs> {
//     fn set(&self, rhs: Rhs) -> Stmt;
// }
//
// pub trait Eq {
//     type Cond: Cond;
//
//     fn eq(&self, rhs: impl Value) -> Self::Cond;
// }
//
// pub struct In<T> {
//     _phantom: PhantomData<T>,
// }
//
// impl<T> Set<T> for In<T> {
//     fn set(&self, rhs: T) -> Stmt {
//         todo!();
//     }
// }
//
// pub struct Out<T> {
//     _phantom: PhantomData<T>,
// }
//
// impl<T> Set<T> for Out<T> {
//     fn set(&self, rhs: T) -> Stmt {
//         todo!();
//     }
// }
//
// #[derive(Clone, Copy)]
// pub struct Signal<T> {
//     _phantom: PhantomData<T>,
// }
//
// impl<T> Signal<T> {
//     pub fn new() -> Self {
//         Self {
//             _phantom: PhantomData,
//         }
//     }
// }
//
// impl<T> Set<T> for Signal<T> {
//     fn set(&self, rhs: T) -> Stmt {
//         todo!();
//     }
// }
//
// pub struct Stmt {}
//
// pub trait Stmts {
//     fn stmts(self) -> impl Iterator<Item = Stmt>;
// }
//
// impl Stmts for Stmt {
//     fn stmts(self) -> impl Iterator<Item = Stmt> {
//         std::iter::once(self)
//     }
// }
//
// impl<const N: usize> Stmts for [Stmt; N] {
//     fn stmts(self) -> impl Iterator<Item = Stmt> {
//         self.into_iter()
//     }
// }
//
// impl Cond for bool {}
//
// impl ops::Add<u32> for Signal<u32> {
//     type Output = AddConst<u32>;
//
//     fn add(self, rhs: u32) -> Self::Output {
//         AddConst { signal: self, rhs }
//     }
// }
//
// pub struct AddConst<T> {
//     signal: Signal<T>,
//     rhs: T,
// }
//
// impl<T> Value for AddConst<T> {}
//
// impl ops::Mul<u32> for Signal<u32> {
//     type Output = MulConst<u32>;
//
//     fn mul(self, rhs: u32) -> Self::Output {
//         MulConst { signal: self, rhs }
//     }
// }
//
// pub struct MulConst<T> {
//     signal: Signal<T>,
//     rhs: T,
// }
//
// impl<T> Value for MulConst<T> {}
//
// pub trait Cmp<Rhs> {
//     type Cond: Cond;
//
//     fn eq(&self, rhs: Rhs) -> Self::Cond;
// }
//
// impl Cmp<u32> for Signal<u32> {
//     type Cond = CmpConst<u32>;
//
//     fn eq(&self, rhs: u32) -> Self::Cond {
//         CmpConst { signal: *self, rhs }
//     }
// }
//
// pub struct CmpConst<T> {
//     signal: Signal<T>,
//     rhs: T,
// }
//
// impl<T> Cond for CmpConst<T> {}
