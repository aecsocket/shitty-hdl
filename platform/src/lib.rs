// mod slop;

pub use board;

use board::Board;
use front::port::{In, Out};
use std::{backtrace::Backtrace, mem, num::NonZero};

#[derive(Debug)]
pub struct Platform {
    pub board: Board,
    pub default_clock_hz: Option<NonZero<u64>>,
    pub reset: Resources<board::Reset>,
    pub led: Resources<board::Led>,
    pub button: Resources<board::Button>,
    pub switch: Resources<board::Switch>,
    pub uart: Resources<board::Uart>,
    pub ddr3: Resources<board::Ddr3>,
}

#[derive(Debug)]
pub struct UartPort<'d> {
    pub tx: Out<'d, bool>,
    pub rx: In<'d, bool>,
}

#[derive(Debug)]
pub struct Ddr3Port {
    // todo
}

#[derive(Debug)]
pub struct Resources<T> {
    items: Vec<ResourceState<T>>,
}

#[derive(Debug)]
enum ResourceState<T> {
    Held { item: T },
    Used { at: Backtrace },
}

impl<T> Resources<T> {
    #[track_caller]
    fn take_(&mut self, index: usize) -> T {
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
            ResourceState::Held { item } => item,
            ResourceState::Used { .. } => unreachable!(),
        }
    }
}

impl Resources<board::Reset> {
    #[track_caller]
    pub fn take(&mut self, index: usize) -> In<bool> {
        todo!();
        // slop::reset(self.take_(index), index)
    }
}

impl Resources<board::Led> {
    #[track_caller]
    pub fn take(&mut self, index: usize) -> Out<bool> {
        todo!();
        // slop::led(self.take_(index), index)
    }
}

impl Resources<board::Button> {
    #[track_caller]
    pub fn take(&mut self, index: usize) -> In<bool> {
        todo!();
        // slop::button(self.take_(index), index)
    }
}

impl Resources<board::Switch> {
    #[track_caller]
    pub fn take(&mut self, index: usize) -> In<bool> {
        todo!();
        // slop::switch(self.take_(index), index)
    }
}

impl Resources<board::Uart> {
    #[track_caller]
    pub fn take(&mut self, index: usize) -> UartPort<'_> {
        todo!();
        // slop::uart(self.take_(index), index)
    }
}

impl Platform {
    #[must_use]
    pub fn from_board(board: Board) -> Self {
        todo!();
        // slop::from_board(board)
    }
}
