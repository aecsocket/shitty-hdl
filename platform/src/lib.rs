mod slop;

pub use shitty_hdl_board as board;
pub use shitty_hdl_board::Board;
pub use shitty_hdl_core as core;

use core::{
    ir,
    module::{In, Out},
    signal::Signal,
    util::Period,
};
use std::{backtrace::Backtrace, cell::RefCell, mem, num::NonZero, rc::Rc};

#[derive(Debug)]
pub struct Platform {
    pub board: Board,
    pub default_clock_hz: Option<NonZero<u64>>,
    pub reset: Resources<In<bool>>,
    pub led: Resources<Out<bool>>,
    pub button: Resources<In<bool>>,
    pub switch: Resources<In<bool>>,
    pub uart: Resources<Uart>,
    pub ddr3: Resources<Ddr3>,
    #[doc(hidden)]
    pub elab: Rc<RefCell<PlatformElab>>,
}

#[derive(Debug, Default)]
pub struct PlatformElab {
    pub bindings: Vec<PinBinding>,
    pub clock_bindings: Vec<ClockBinding>,
    pub default_clock: Option<Signal<bool>>,
    owner: Option<ir::DesignId>,
}

#[derive(Debug)]
pub struct Uart {
    pub tx: Out<bool>,
    pub rx: In<bool>,
}

#[derive(Debug)]
pub struct Ddr3 {
    // todo
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinBinding {
    pub signal: ir::SignalId,
    pub bit: u32,
    pub package_pin: String,
    pub io_standard: String,
}

#[derive(Debug, Clone)]
pub struct ClockBinding {
    pub signal: ir::SignalId,
    pub period: Period,
}

#[derive(Debug)]
pub struct Resources<T> {
    items: Vec<ResourceState<T>>,
}

enum ResourceState<T> {
    Held { make: Box<dyn FnOnce() -> T> },
    Used { at: Backtrace },
}

impl<T> Resources<T> {
    #[track_caller]
    pub fn take(&mut self, index: usize) -> T {
        let state = self.items.get_mut(index).expect("invalid resource index");
        if let ResourceState::Used { at } = state {
            panic!("item already used at {at:?}");
        }
        let old = mem::replace(
            state,
            ResourceState::Used {
                at: Backtrace::force_capture(),
            },
        );
        match old {
            ResourceState::Held { make } => make(),
            ResourceState::Used { .. } => unreachable!(),
        }
    }
}

impl Platform {
    #[must_use]
    pub fn from_board(board: Board) -> Self {
        slop::from_board(board)
    }
}
