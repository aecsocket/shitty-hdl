use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};

use crate::{BluespecCompiler, EmitBluesim, EmitVerilog};
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
    fn verilog_library_dir(&self) -> Result<PathBuf> {
        let output = Command::new(&self.bin)
            .arg("-print-flags")
            .output()
            .with_context(|| anyhow!("querying BSC library directory ({:?})", self.bin))?;
        ensure!(
            output.status.success(),
            "querying BSC library directory failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let flags = str::from_utf8(&output.stdout).context("decoding BSC flags")?;
        let library = flags
            .lines()
            .find_map(|line| line.trim().strip_prefix("-i ").map(str::trim))
            .filter(|path| !path.is_empty())
            .context("BSC did not report its library directory")?;
        let path = Path::new(library).join("Verilog");
        path.canonicalize()
            .with_context(|| anyhow!("locating BSC Verilog library {path:?}"))
    }

    fn emit_verilog(
        &self,
        EmitVerilog {
            bsv_path,
            module,
            reset_prefix,
            bluespec_dir,
            verilog_dir,
        }: EmitVerilog<'_>,
    ) -> Result<PathBuf> {
        let bin = &self.bin;
        let mut command = Command::new(bin);
        command
            .arg("-verilog")
            .args(["-reset-prefix", reset_prefix])
            .args(["-g", module])
            .args([Path::new("-bdir"), bluespec_dir])
            .args([Path::new("-vdir"), verilog_dir])
            .arg(bsv_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        debug!("spawning {command:?}");
        run_bsc(command)?;

        Ok(verilog_dir.join(format!("{module}.v")))
    }

    fn emit_bluesim(
        &self,
        EmitBluesim {
            bsv_path,
            module,
            exe_path,
            bluespec_dir,
            sim_dir,
        }: EmitBluesim<'_>,
    ) -> Result<PathBuf> {
        let bin = &self.bin;

        let mut compile = Command::new(bin);
        compile
            .arg("-sim")
            .arg("-u")
            .args(["-g", module])
            .args([Path::new("-bdir"), bluespec_dir])
            .args([Path::new("-simdir"), sim_dir])
            .arg(bsv_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        debug!("spawning {compile:?}");
        run_bsc(compile)?;

        let mut link = Command::new(bin);
        link.arg("-sim")
            .args(["-e", module])
            .args([Path::new("-bdir"), bluespec_dir])
            .args([Path::new("-simdir"), sim_dir])
            .args(["-o"])
            .arg(exe_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        debug!("spawning {link:?}");
        run_bsc(link)?;

        Ok(exe_path.to_path_buf())
    }
}

fn run_bsc(mut command: Command) -> Result<()> {
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

    Ok(())
}
