use std::path::{Path, PathBuf};

pub use anyhow;

pub use board::Board;

use anyhow::Result;
pub trait Toolchain {
    fn make_bitstream(
        &self,
        input: BitstreamInput,
        progress: &mut (dyn FnMut(MakeBitstreamEvent) + Send),
    ) -> Result<PathBuf>;

    fn program(&self, board: Board, bitstream: &Path) -> Result<()>;
}

const _ASSERT_DYN_COMPATIBLE: Option<&dyn Toolchain> = None;

#[derive(Debug)]
pub struct BitstreamInput<'a> {
    pub board: Board,
    pub bindings: &'a Bindings,
    pub verilog_path: &'a Path,
    pub verilog_library_dirs: &'a [PathBuf],
    pub module: &'a str,
    pub out_dir: &'a Path,
}

#[derive(Debug)]
pub enum MakeBitstreamEvent {
    Synthesis,
    Optimization,
    Placement,
    PhysicalOptimization,
    Routing,
    WriteBitstream,
    Finish,
}

#[derive(Debug, Default)]
pub struct Bindings {
    pub clock: Vec<Binding<board::Clock>>,
    pub reset: Vec<Binding<board::Reset>>,
    pub led: Vec<Binding<board::Led>>,
    pub button: Vec<Binding<board::Button>>,
    pub switch: Vec<Binding<board::Switch>>,
}

#[derive(Debug)]
pub struct Binding<T> {
    pub resource: T,
    pub rtl_port: String,
}
