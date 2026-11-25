mod demo;
mod slop;

use anyhow::Result;
pub use demo::blinky;
use shitty_hdl_board::{Board, Toolchain};
use shitty_hdl_core::{ir, module::Module};
use shitty_hdl_platform::{ClockBinding, PinBinding, Platform};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub fn cli<Top: Module>(
    manifest_dir: impl AsRef<Path>,
    board: Board,
    make_top: impl FnOnce(Platform) -> Top,
) -> Result<()> {
    slop::cli(manifest_dir, board, make_top)
}

pub fn build<Top: Module>(
    target: impl AsRef<Path>,
    board: Board,
    make_top: impl FnOnce(Platform) -> Top,
) -> Result<PathBuf> {
    slop::build(target, board, make_top)
}

pub fn render_constraints(
    toolchain: Toolchain,
    pins: &[PinBinding],
    clocks: &[ClockBinding],
    port_names: &HashMap<ir::SignalId, String>,
) -> Result<String> {
    slop::render_constraints(toolchain, pins, clocks, port_names)
}

#[macro_export]
macro_rules! cli {
    ($board:expr, $make_top:expr $(,)?) => {
        $crate::cli(env!("CARGO_MANIFEST_DIR"), $board, $make_top).expect("command failed")
    };
}
