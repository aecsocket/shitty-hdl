mod system;

pub use system::SystemBluespec;

use std::path::{Path, PathBuf};

use anyhow::Result;

pub trait BluespecCompiler {
    fn verilog_library_dir(&self) -> Result<PathBuf>;

    fn emit_verilog(
        &self,
        bsv_path: &Path,
        module: &str,
        bdir: &Path,
        vdir: &Path,
    ) -> Result<PathBuf>;
}

const _ASSERT_DYN_COMPATIBLE: Option<&dyn BluespecCompiler> = None;
