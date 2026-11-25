use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};

use crate::BluespecCompiler;
use anyhow::{Context, Result, anyhow, ensure};
use tracing::{debug, warn};

#[derive(Debug)]
pub struct SystemBluespec {
    pub bin: PathBuf,
}

impl Default for SystemBluespec {
    fn default() -> Self {
        Self {
            bin: PathBuf::from("bsc"),
        }
    }
}

impl BluespecCompiler for SystemBluespec {
    fn emit_verilog(
        &self,
        bsv_path: &Path,
        module: &str,
        bdir: &Path,
        vdir: &Path,
    ) -> Result<PathBuf> {
        let bin = &self.bin;
        let mut command = Command::new(bin);
        command
            .arg("-verilog")
            .args(["-g", module])
            .args([Path::new("-bdir"), bdir])
            .args([Path::new("-vdir"), vdir])
            .arg(bsv_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .with_context(|| anyhow!("spawning {command:?}"))?;
        let stdout = child.stdout.take().expect("`stdout` is `Stdio::piped()`");
        let stderr = child.stderr.take().expect("`stderr` is `Stdio::piped()`");

        thread::scope(|scope| {
            scope.spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    debug!("bsc: {line}");
                }
            });

            scope.spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    warn!("bsc: {line}");
                }
            });
        });

        let status = child.wait().context("waiting for bsc to finish")?;
        ensure!(status.success(), "bsc failed");

        Ok(vdir.join(format!("{module}.v")))
    }
}
