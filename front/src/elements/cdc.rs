//! Clock-domain crossing primitives.
//!
//! These are the only sanctioned ways to move values between clock
//! domains; every other crossing is rejected by the type system.

use crate::{
    domain::{Clock, Domain},
    elab,
    module::Rule,
    reg::Reg,
    value::{Shape, Value},
};

/// Two-flop synchronizer for single-bit signals.
///
/// Samples `input` into two registers clocked by `clk_d2`, eliminating
/// metastability. Not valid for multi-bit buses (the bits may not be
/// coherent); use an asynchronous FIFO or a handshake for those.
#[must_use]
pub fn sync_flop<D1: Domain, D2: Domain>(
    clk_d2: Clock<D2>,
    input: Value<D1, bool>,
) -> Value<D2, bool> {
    let first = Reg::new(clk_d2, false);
    let mut second = Reg::new(clk_d2, false);

    Rule::define_as(async |_cx| {
        elab::write_register::<D2, bool>(first.id, Value::from_handle(input.handle()));
        second.write(first.read());
    });

    second.read()
}

/// Rebrands a value into another domain **without any synchronization**.
///
/// In simulation the value is sampled on the destination clock; in
/// hardware it is a bare wire. Use only when the two domains are
/// synchronous (e.g. phase-locked multiples), or as the building block
/// of a custom crossing primitive.
#[must_use]
pub fn unsafe_synchronizer<D1: Domain, D2: Domain, T: Shape>(value: Value<D1, T>) -> Value<D2, T> {
    Value::from_handle(value.handle())
}
