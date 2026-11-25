mod system;

pub use system::SystemBluespec;

use std::path::{Path, PathBuf};

use anyhow::Result;

pub trait BluespecCompiler {
    fn verilog_library_dir(&self) -> Result<PathBuf>;

    fn emit_verilog(&self, input: EmitVerilog<'_>) -> Result<PathBuf>;

    fn emit_bluesim(&self, input: EmitBluesim<'_>) -> Result<PathBuf>;
}

const _ASSERT_DYN_COMPATIBLE: Option<&dyn BluespecCompiler> = None;

#[derive(Debug, Clone, Copy)]
pub struct EmitVerilog<'a> {
    pub bsv_path: &'a Path,
    pub module: &'a str,
    pub reset_prefix: &'a str,
    pub bluespec_dir: &'a Path,
    pub verilog_dir: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct EmitBluesim<'a> {
    pub bsv_path: &'a Path,
    pub module: &'a str,
    pub exe_path: &'a Path,
    pub bluespec_dir: &'a Path,
    pub sim_dir: &'a Path,
}
