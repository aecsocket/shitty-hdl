#![feature(generic_const_exprs)]

pub mod expr;
pub mod module;
pub mod signal;
pub mod value;

pub use shitty_hdl_derive::module;

pub mod prelude {
    pub use crate::expr::{IntoExpr, trunc, zext};
    pub use crate::module::{Clock, Domain, DomainExt, In, Out, comb, if_};
    pub use crate::signal::Signal;
    pub use crate::value::n::*;
    pub use crate::value::{Bits, Shape, Sint, Uint, n};
    pub use shitty_hdl_derive::module;
}

#[doc(hidden)]
pub mod __ {
    pub use crate::module::{Module, SignalFlow, if_};
    pub use bon;
}
