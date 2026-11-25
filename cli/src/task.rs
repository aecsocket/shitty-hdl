use std::{
    fs, mem,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context as _, Result, anyhow, ensure};
use front::elab;
use toolchain_api::{Binding, Bindings, BitstreamInput, Toolchain};
use toolchain_bsc::{EmitBluesim, EmitVerilog};
use tracing::info;

use crate::{Context, START};

const RESET_PREFIX: &str = "RST_N";
const SIM_WRAPPER_MODULE: &str = "mkSimWrapper";

pub fn check(mut cx: Context) -> Result<()> {
    let GenVerilog {
        bsv_path,
        verilog_path,
        ..
    } = gen_verilog(&mut cx)?;
    info!("generated Verilog in {:.2?}", START.elapsed());
    info!("- BSV output to {bsv_path:?}");
    info!("- Verilog output to {verilog_path:?}");
    Ok(())
}

pub fn simulate(mut cx: Context) -> Result<()> {
    let GenBsv {
        module_name,
        bsv_path,
        sim_wrapper_path,
        ..
    } = gen_bsv(&mut cx)?;
    info!("generated BSV in {:.2?}", START.elapsed());
    info!("- BSV output to {bsv_path:?}");
    info!("- simulation wrapper output to {sim_wrapper_path:?}");

    let sim_dir = cx.out_dir.join("bluesim");
    fs::create_dir_all(&sim_dir).with_context(|| anyhow!("creating {sim_dir:?}"))?;

    info!("compiling simulation");
    let exe_path = cx
        .bsc
        .emit_bluesim(EmitBluesim {
            bsv_path: &sim_wrapper_path,
            module: SIM_WRAPPER_MODULE,
            exe_path: &sim_dir.join(&module_name),
            bluespec_dir: &cx.out_dir.join("bluespec"),
            sim_dir: &sim_dir,
        })
        .context("compiling Bluesim simulation")?;
    info!("compiled simulation in {:.2?}", START.elapsed());
    info!("- simulation binary at {exe_path:?}");

    info!("running simulation");
    let status = Command::new(&exe_path)
        .current_dir(&sim_dir)
        .status()
        .with_context(|| anyhow!("running {exe_path:?}"))?;
    ensure!(
        status.success(),
        "simulation {exe_path:?} failed with {status}"
    );
    info!("simulation finished in {:.2?}", START.elapsed());
    Ok(())
}

pub fn build(mut cx: Context, program: bool) -> Result<()> {
    let GenVerilog {
        bindings,
        module_name,
        bsv_path,
        verilog_path,
        verilog_library_dirs,
    } = gen_verilog(&mut cx)?;
    info!("generated Verilog in {:.2?}", START.elapsed());

    let vivado_dir = cx.out_dir.join("vivado");
    fs::create_dir_all(&vivado_dir).with_context(|| anyhow!("creating {vivado_dir:?}"))?;

    let input = BitstreamInput {
        board: cx.board,
        bindings: &bindings,
        verilog_path: &verilog_path,
        verilog_library_dirs: &verilog_library_dirs,
        module: &module_name,
        out_dir: &vivado_dir,
    };

    info!("making bitstream");
    let bitstream_path = toolchain_vivado::Vivado::default()
        .make_bitstream(input, &mut |event| {
            info!("started stage {event:?}");
        })
        .context("making bitstream")?;

    info!("made bitstream in {:.2?}", START.elapsed());
    info!("- BSV output to {bsv_path:?}");
    info!("- Verilog output to {verilog_path:?}");
    info!("- Bitstream output to {bitstream_path:?}");

    if program {
        program_bitstream(&cx, &bitstream_path)?;
    }
    Ok(())
}

pub fn program(mut cx: Context) -> Result<()> {
    let GenVerilog { module_name, .. } = gen_verilog(&mut cx)?;
    let bitstream_path = cx.out_dir.join("vivado").join(format!("{module_name}.bit"));
    ensure!(
        bitstream_path.exists(),
        "bitstream {bitstream_path:?} does not exist; run `build` first"
    );
    program_bitstream(&cx, &bitstream_path)
}

fn program_bitstream(cx: &Context, bitstream_path: &Path) -> Result<()> {
    info!("programming board");
    toolchain_vivado::Vivado::default()
        .program(cx.board, bitstream_path)
        .context("programming board")?;
    info!("programmed board in {:.2?}", START.elapsed());
    Ok(())
}

struct GenBsv {
    bindings: Bindings,
    module_name: String,
    bsv_path: PathBuf,
    sim_wrapper_path: PathBuf,
}

struct GenVerilog {
    bindings: Bindings,
    module_name: String,
    bsv_path: PathBuf,
    verilog_path: PathBuf,
    verilog_library_dirs: Vec<PathBuf>,
}

fn gen_bsv(cx: &mut Context) -> Result<GenBsv> {
    let bluespec_dir = cx.out_dir.join("bluespec");
    fs::create_dir_all(&bluespec_dir).with_context(|| anyhow!("creating {bluespec_dir:?}"))?;

    let platform = platform::from_board(cx.board);
    let storage = elab::Bump::new();
    let make_top = mem::replace(&mut cx.make_top, Box::new(|_| {}));
    let elab_platform = platform.clone();
    let air_module = elab::elaborate(&storage, || make_top(elab_platform));

    let (package, module) = bsv::from_air(&storage, &air_module).context("lowering AIR to BSV")?;
    let mut bindings = Bindings::default();
    let resources = platform.bindings();
    for binding in resources.clock {
        let port = module
            .parameters
            .iter()
            .find_map(|parameter| match parameter {
                bsv::Parameter::Clock { name, port } if *name == binding.name => Some(*port),
                _ => None,
            })
            .with_context(|| {
                anyhow!(
                    "platform clock {:?} is missing from the top module",
                    binding.name
                )
            })?;
        bindings.clock.push(Binding {
            resource: binding.resource,
            rtl_port: port.into(),
        });
    }
    for binding in resources.reset {
        anyhow::ensure!(
            binding.resource.active_on == board::ActiveOn::Low,
            "BSC's global reset requires an active-low board reset; {:?} is active-high",
            binding.resource.pin.name,
        );
        bindings.reset.push(Binding {
            resource: binding.resource,
            rtl_port: RESET_PREFIX.into(),
        });
    }
    for binding in resources.button {
        let port = module
            .parameters
            .iter()
            .find_map(|parameter| match parameter {
                bsv::Parameter::Value(field) if field.name == binding.name => Some(field.name),
                _ => None,
            })
            .with_context(|| {
                anyhow!(
                    "platform button {:?} is missing from the top module",
                    binding.name
                )
            })?;
        bindings.button.push(Binding {
            resource: binding.resource,
            rtl_port: port.into(),
        });
    }
    for binding in resources.led {
        let port = package
            .interfaces
            .iter()
            .find(|interface| interface.name == module.interface)
            .and_then(|interface| {
                interface.members.iter().find_map(|member| match member {
                    bsv::InterfaceMember::Method(method) if method.name == binding.name => {
                        method.result_port
                    }
                    _ => None,
                })
            })
            .with_context(|| {
                anyhow!(
                    "platform LED {:?} is missing from the top module",
                    binding.name
                )
            })?;
        bindings.led.push(Binding {
            resource: binding.resource,
            rtl_port: port.into(),
        });
    }
    let bsv_path = bluespec_dir.join(format!("{}.bsv", package.name));
    fs::write(&bsv_path, package.to_string())
        .with_context(|| anyhow!("writing BSV to {bsv_path:?}"))?;

    let sim_wrapper_path = bluespec_dir.join(format!("{}_sim.bsv", package.name));
    let sim_wrapper =
        bsv::sim_wrapper_source(&package, &module).context("generating simulation wrapper")?;
    fs::write(&sim_wrapper_path, sim_wrapper)
        .with_context(|| anyhow!("writing simulation wrapper to {sim_wrapper_path:?}"))?;

    Ok(GenBsv {
        bindings,
        module_name: module.name.to_string(),
        bsv_path,
        sim_wrapper_path,
    })
}

fn gen_verilog(cx: &mut Context) -> Result<GenVerilog> {
    let GenBsv {
        bindings,
        module_name,
        bsv_path,
        ..
    } = gen_bsv(cx)?;

    let verilog_dir = cx.out_dir.join("verilog");
    fs::create_dir_all(&verilog_dir).with_context(|| anyhow!("creating {verilog_dir:?}"))?;
    let verilog_path = cx
        .bsc
        .emit_verilog(EmitVerilog {
            bsv_path: &bsv_path,
            module: &module_name,
            reset_prefix: RESET_PREFIX,
            bluespec_dir: &cx.out_dir.join("bluespec"),
            verilog_dir: &verilog_dir,
        })
        .context("lowering BSV to Verilog")?;

    Ok(GenVerilog {
        bindings,
        module_name,
        bsv_path,
        verilog_path,
        verilog_library_dirs: vec![verilog_dir, cx.bsc.verilog_library_dir()?],
    })
}
