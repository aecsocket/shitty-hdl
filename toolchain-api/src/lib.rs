use std::path::PathBuf;

pub use anyhow;

use anyhow::Result;
use board::Board;

pub trait Toolchain {
    fn make_bitstream(
        &self,
        input: BitstreamInput,
        progress: &mut dyn FnMut(MakeBitstreamEvent),
    ) -> Result<()>;

    fn program(&self) -> Result<()>;
}

const _ASSERT_DYN_COMPATIBLE: Option<&dyn Toolchain> = None;

#[derive(Debug)]
pub struct BitstreamInput {
    pub board: Board,
    pub bindings: Bindings,
    pub verilog_path: PathBuf,
    pub module: String,
    pub out_dir: PathBuf,
}

#[derive(Debug)]
pub enum MakeBitstreamEvent {
    Synthesis,
    Optimization,
    Placement,
    PhysicalOptimization,
    Routing,
    WriteBitstream,
}

#[derive(Debug)]
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
