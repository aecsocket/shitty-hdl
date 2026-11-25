use crate::{
    ir::{self, runtime},
    module::{In, Out},
    signal::Signal,
};

pub fn design_id() -> ir::DesignId {
    runtime::owner()
}

#[track_caller]
pub fn input(name: &str, inverted: bool) -> (In<bool>, ir::SignalId) {
    let (signal, physical) = Signal::physical(name, ir::PortDirection::Input, inverted);
    (In::from_signal(signal), physical)
}

#[track_caller]
pub fn output(name: &str, inverted: bool) -> (Out<bool>, ir::SignalId) {
    let (signal, physical) = Signal::physical(name, ir::PortDirection::Output, inverted);
    (Out::from_signal(signal), physical)
}

#[track_caller]
pub fn clock(name: &str) -> (Signal<bool>, ir::SignalId) {
    Signal::physical(name, ir::PortDirection::Input, false)
}
