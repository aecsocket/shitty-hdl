#[cfg(test)]
mod tests;

use anyhow::{Context, Result, bail, ensure};
use shitty_hdl_board::{Board, Toolchain};
use shitty_hdl_core::{
    ir,
    module::{Module, Sync},
    util::Period,
    verilog,
};
use shitty_hdl_platform::{ClockBinding, PinBinding, Platform};
use std::{
    collections::HashMap,
    fmt::Write,
    path::{Path, PathBuf},
};

#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Build,
    Upload,
}

pub(super) fn cli<Top: Module>(
    manifest_dir: impl AsRef<Path>,
    board: Board,
    make_top: impl FnOnce(Platform) -> Top,
) -> Result<()> {
    match <Cli as clap::Parser>::parse().command {
        Command::Upload => bail!("upload is unsupported; builds currently generate artifacts only"),
        Command::Build => {
            let dir = manifest_dir.as_ref();
            let metadata = cargo_metadata::MetadataCommand::new()
                .current_dir(dir)
                .manifest_path(dir.join("Cargo.toml"))
                .no_deps()
                .exec()
                .context("running cargo metadata")?;
            let output = build(
                metadata.target_directory.join("hdl").as_std_path(),
                board,
                make_top,
            )?;
            println!(
                "Artifacts only: {} (synthesis and upload were not executed)",
                output.display()
            );
            Ok(())
        }
    }
}

pub(super) fn build<Top: Module>(
    target: impl AsRef<Path>,
    board: Board,
    make_top: impl FnOnce(Platform) -> Top,
) -> Result<PathBuf> {
    ensure!(
        board.toolchain == Toolchain::Vivado,
        "unsupported toolchain {:?}",
        board.toolchain
    );
    let platform = Platform::from_board(board)?;
    let capture = platform.clone();
    let mut clock = None;
    let (mut design, top) = ir::elaborate_named(Top::NAME, || -> Result<Top> {
        clock = Some(platform.clocks.try_take(0)?.id());
        Ok(make_top(platform))
    })?;
    let _top = top?;
    design.bind_domain_clock::<Sync>(clock.context("default clock was not allocated")?)?;
    for domain in &design.domains {
        ensure!(
            domain.clock.is_some(),
            "clock domain {} has no physical clock binding",
            domain.name
        );
    }
    let bindings = capture.bindings();
    for (index, signal) in design.signals.iter().enumerate() {
        if signal.port.is_some() {
            for bit in 0..signal.ty.width {
                ensure!(
                    bindings
                        .iter()
                        .any(|binding| binding.signal == ir::SignalId(index) && binding.bit == bit),
                    "top-level port {} bit {bit} has no physical pin binding; request a board resource instead",
                    signal.name,
                );
            }
        }
    }
    let lowered = verilog::lower_with_ports(&design)?;
    let xdc = render_constraints(
        board.toolchain,
        &bindings,
        &capture.clock_bindings(),
        &lowered.port_names,
    )?;
    let name = safe_name(Top::NAME);
    let output = target.as_ref().join(name);
    tracing::debug!(?output, "writing HDL artifacts only");
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("top.sv"), lowered.source)?;
    std::fs::write(output.join("top.xdc"), xdc)?;
    Ok(output)
}

fn safe_name(name: &str) -> String {
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() { "top".into() } else { name }
}

// Double-quoted Tcl words: prevent substitutions, including bus-index commands.
fn tcl(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '\\' | '"' | '$' | '[' | ']' => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn semantic_name(name: &str) -> &str {
    name.strip_prefix('\\').map_or(name, str::trim_end)
}

fn nanos(period: Period) -> String {
    let fs = period.femtos();
    let fraction = fs % 1_000_000;
    if fraction == 0 {
        (fs / 1_000_000).to_string()
    } else {
        format!("{}.{fraction:06}", fs / 1_000_000)
            .trim_end_matches('0')
            .to_owned()
    }
}

pub(super) fn render_constraints(
    toolchain: Toolchain,
    pins: &[PinBinding],
    clocks: &[ClockBinding],
    port_names: &HashMap<ir::SignalId, String>,
) -> Result<String> {
    ensure!(
        toolchain == Toolchain::Vivado,
        "unsupported toolchain {toolchain:?}"
    );
    let bus = |signal, bit| -> Result<String> {
        let name = port_names
            .get(&signal)
            .context("physical binding has no lowered HDL port")?;
        Ok(tcl(&format!("{}[{bit}]", semantic_name(name))))
    };
    let mut out = String::new();
    for pin in pins {
        let port = bus(pin.signal, pin.bit)?;
        writeln!(
            out,
            "set_property PACKAGE_PIN {} [get_ports -quiet -- {port}]",
            tcl(&pin.package_pin)
        )?;
        writeln!(
            out,
            "set_property IOSTANDARD {} [get_ports -quiet -- {port}]",
            tcl(&pin.io_standard)
        )?;
    }
    for clock in clocks {
        writeln!(
            out,
            "create_clock -period {} [get_ports -quiet -- {}]",
            nanos(clock.period),
            bus(clock.signal, 0)?
        )?;
    }
    Ok(out)
}
