#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

pub mod expr;
pub mod module;
pub mod signal;
mod util;
pub mod value;

pub use shitty_hdl_derive::{Shape, module};

pub mod prelude {
    pub use crate::expr::{EqExt, IntoExpr, trunc, zext};
    pub use crate::module::{Clock, Domain, DomainExt, In, Out, comb, if_};
    pub use crate::signal::Signal;
    pub use crate::u;
    pub use crate::value::{Bits, I, Shape, U};
    pub use shitty_hdl_derive::{Shape, module};
}

#[doc(hidden)]
pub mod __ {
    pub use crate::module::{Module, SignalFlow, if_};
    pub use bon;
}
