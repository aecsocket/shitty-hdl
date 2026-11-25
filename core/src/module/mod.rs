mod control;
mod domain;
mod signal;

pub use control::*;
pub use domain::*;
pub use signal::*;

pub trait Module {
    const NAME: &'static str;
}
