use std::{
    fmt::{self, Write as _},
    fs,
    io::{self, BufRead, BufReader, Write as _},
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
};

use anyhow::{Context, Result, anyhow, bail, ensure};
use toolchain_api::{Binding, Bindings, BitstreamInput, MakeBitstreamEvent, Toolchain};
use tracing::debug;

pub struct Vivado {
    pub bin: PathBuf,
}

impl Default for Vivado {
    fn default() -> Self {
        Self {
            bin: PathBuf::from("vivado"),
        }
    }
}

impl Toolchain for Vivado {
    fn make_bitstream(
        &self,
        input: BitstreamInput,
        progress: &mut dyn FnMut(MakeBitstreamEvent),
    ) -> Result<()> {
        const SCRIPT_PATH: &str = "build.tcl";
        const SCRIPT_CONTENTS: &str = include_str!("build.tcl");

        let out_dir = input.out_dir;
        let verilog_path = input.verilog_path;
        let board::Toolchain::Vivado(board::ToolchainVivado { part, .. }) = input.board.toolchain
        else {
            bail!("board does not use Vivado toolchain");
        };

        debug!("making bitstream for part {part:?}");

        let xdc_path = out_dir.join("top.xdc");
        fs::write(&xdc_path, write_constraints(&input.bindings)?)
            .with_context(|| anyhow!("writing {xdc_path:?}"))?;

        let script_path = out_dir.join(SCRIPT_PATH);
        fs::write(&script_path, SCRIPT_CONTENTS)
            .with_context(|| anyhow!("writing {script_path:?}"))?;

        let vivado_bin = &self.bin;
        let mut command = Command::new(vivado_bin);
        command
            .current_dir(&out_dir)
            .args(["-mode", "batch"])
            .args(["-source", SCRIPT_PATH])
            .arg("-tclargs")
            .arg(&out_dir)
            .arg(&verilog_path)
            .arg(&xdc_path)
            .arg(input.module)
            .arg(part)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        debug!("spawning vivado: {command:?}");

        let mut child = command
            .spawn()
            .with_context(|| anyhow!("spawning vivado ({vivado_bin:?})"))?;
        forward_output(&mut child, progress)
    }

    fn program(&self) -> Result<()> {
        Ok(())
    }
}

fn write_constraints(bindings: &Bindings) -> Result<String> {
    let mut s = String::new();

    for (idx, binding) in bindings.clock.iter().enumerate() {
        write_pin_constraints(&mut s, binding.resource.pin, &binding.rtl_port)?;
        let port = tcl_word(&binding.rtl_port);
        #[expect(clippy::cast_precision_loss, reason = "acceptable")]
        let period = 1_000_000_000.0 / binding.resource.hz.get() as f64;
        writeln!(
            s,
            "create_clock -name clock_{idx} -period {period} [get_ports {port}]"
        )?;
    }

    write_bindings(&mut s, &bindings.reset, |resource| resource.pin)?;
    write_bindings(&mut s, &bindings.led, |resource| resource.pin)?;
    write_bindings(&mut s, &bindings.button, |resource| resource.pin)?;
    write_bindings(&mut s, &bindings.switch, |resource| resource.pin)?;

    Ok(s)
}

fn write_bindings<T>(
    out: &mut String,
    bindings: &[Binding<T>],
    pin: impl Fn(&T) -> board::Pin,
) -> fmt::Result {
    for binding in bindings {
        write_pin_constraints(out, pin(&binding.resource), &binding.rtl_port)?;
    }
    Ok(())
}

fn write_pin_constraints(out: &mut String, pin: board::Pin, port: &str) -> fmt::Result {
    let port = tcl_word(port);
    let name = tcl_word(pin.name);
    let io_standard = tcl_word(pin.io_standard);
    writeln!(out, "set_property PACKAGE_PIN {name} [get_ports {port}]")?;
    writeln!(
        out,
        "set_property IOSTANDARD {io_standard} [get_ports {port}]"
    )
}

fn tcl_word(value: &str) -> String {
    let mut word = String::from("\"");
    for char in value.chars() {
        match char {
            '\\' | '"' | '$' | '[' | ']' => {
                word.push('\\');
                word.push(char);
            }
            '\n' => word.push_str("\\n"),
            '\r' => word.push_str("\\r"),
            '\t' => word.push_str("\\t"),
            _ => word.push(char),
        }
    }
    word.push('"');
    word
}

fn forward_output(child: &mut Child, progress: &mut dyn FnMut(MakeBitstreamEvent)) -> Result<()> {
    let stdout = child.stdout.take().expect("`stdout` is `Stdio::piped()`");
    let mut stderr = child.stderr.take().expect("`stderr` is `Stdio::piped()`");

    thread::scope(|scope| -> Result<()> {
        let stderr_thread = scope.spawn(move || {
            io::copy(&mut stderr, &mut io::stderr().lock()).context("forwarding stderr")
        });
        let stdout_result = (|| -> Result<()> {
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            loop {
                line.clear();
                if reader
                    .read_until(b'\n', &mut line)
                    .context("reading stdout")?
                    == 0
                {
                    break;
                }
                {
                    let mut out = io::stdout().lock();
                    out.write_all(&line).context("forwarding stdout")?;
                    out.flush().context("flushing stdout")?;
                }
                if let Some(marker) = line.strip_prefix(b"shitty_hdl:") {
                    let marker = str::from_utf8(marker)
                        .context("invalid progress marker")?
                        .trim();
                    let event = match marker {
                        "Synthesis" => MakeBitstreamEvent::Synthesis,
                        "Optimization" => MakeBitstreamEvent::Optimization,
                        "Placement" => MakeBitstreamEvent::Placement,
                        "PhysicalOptimization" => MakeBitstreamEvent::PhysicalOptimization,
                        "Routing" => MakeBitstreamEvent::Routing,
                        "WriteBitstream" => MakeBitstreamEvent::WriteBitstream,
                        _ => panic!("unknown progress marker: {marker}"),
                    };
                    progress(event);
                }
            }
            Ok(())
        })();
        if stdout_result.is_err() {
            let _ = child.kill();
        }
        let status = child.wait().context("waiting for child");
        let stderr_result = stderr_thread
            .join()
            .map_err(|_| anyhow!("stderr forwarding thread panicked"));
        stdout_result?;
        stderr_result??;
        let status = status?;

        ensure!(status.success(), "failed: {status}");
        Ok(())
    })
}
