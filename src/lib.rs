mod core;
pub mod data;
pub mod elaborate;
pub mod n;
pub mod sim;

pub use core::*;
pub use shitty_hdl_derive::{Shape, module};

pub mod prelude {
    pub use crate::{
        Clock, CmpExt, In, IntoValue, Out, Shape, Signal, Value, capture, comb, module, n::*,
    };
}

// re-exports for macros
#[doc(hidden)]
pub mod __ {
    pub use crate::{Module, Shape, SignalFlow, ValAdd, ValDiv, ValMul, ValSub, if_};
    pub use bon;
}
