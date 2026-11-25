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
use std::{backtrace::Backtrace, mem, num::NonZero};

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

#[derive(Debug)]
pub struct Resources<T> {
    items: Vec<ResourceState<T>>,
}

#[derive(Debug)]
enum ResourceState<T> {
    /// Still held by the [`Platform`].
    Held { item: T },
    /// Used by a consumer of the [`Platform`].
    Used { at: Backtrace },
}

impl<T> Resources<T> {
    #[track_caller]
    pub fn take(&mut self, index: usize) -> T {
        let state = self.items.get_mut(index).expect("invalid resource index");
        let here = Backtrace::force_capture();
        let old = mem::replace(state, ResourceState::Used { at: here });
        match old {
            ResourceState::Held { item } => item,
            ResourceState::Used { at } => {
                panic!("item already used at {at:?}");
            }
        }
    }
}

#[must_use]
pub fn from_board(board: Board) -> Platform {
    let default_clock = board.resources.default_clock.map(|idx| {
        board
            .resources
            .clock
            .get(idx)
            .expect("invalid default clock index")
    });
    let default_clock_hz = default_clock.map(|clock| clock.hz);

    // let reset = board.resources.reset.iter().map(|reset| );

    Platform {
        board,
        default_clock_hz,
        reset,
    }
}
