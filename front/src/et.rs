//! Elaboration runtime.
//!
//! Like a `runtime` (`rt`) but for elaboration, which is active when we're
//! lowering items to [`pir`].

use std::marker::PhantomData;

// pub fn lower() -> pir::Module {
//     todo!();
// }

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Brand<'d>(PhantomData<fn(&'d ()) -> &'d ()>);

fn with_brand<R>(f: impl for<'d> FnOnce(Brand<'d>) -> R) -> R {
    f(Brand(PhantomData))
}
