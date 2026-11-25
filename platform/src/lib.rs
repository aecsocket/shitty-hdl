pub use board;

use board::Board;
use std::num::NonZero;

#[derive(Debug)]
pub struct Platform {
    pub board: Board,
    pub default_clock_hz: Option<NonZero<u64>>,
}

#[must_use]
pub fn from_board(board: Board) -> Platform {
    let default_clock = board.resources.clock.first();

    Platform {
        board,
        default_clock_hz: default_clock.map(|clock| clock.hz),
    }
}
