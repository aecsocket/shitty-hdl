pub use board;

use board::Board;
use front::domain::Clock;
use std::num::NonZero;

#[derive(Debug)]
pub struct Platform {
    pub board: Board,
    pub default_clock_hz: Option<NonZero<u64>>,
}

impl Platform {
    #[must_use]
    pub fn default_clock(&self) -> Option<Clock> {
        self.board
            .resources
            .default_clock
            .and_then(|index| self.board.resources.clock.get(index))
            .map(|_| Clock::input("clk"))
    }
}

#[must_use]
pub fn from_board(board: Board) -> Platform {
    let default_clock = board
        .resources
        .default_clock
        .and_then(|index| board.resources.clock.get(index));

    Platform {
        board,
        default_clock_hz: default_clock.map(|clock| clock.hz),
    }
}
