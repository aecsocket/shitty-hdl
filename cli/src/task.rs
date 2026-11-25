use std::{
    fs, mem,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, anyhow, ensure};
use front::elab;
use toolchain_api::{Binding, Bindings, BitstreamInput, Toolchain};
use toolchain_bsc::EmitVerilog;
use tracing::info;

use crate::{Context, START};

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

struct GenVerilog {
    bindings: Bindings,
    module_name: String,
    bsv_path: PathBuf,
    verilog_path: PathBuf,
    verilog_library_dirs: Vec<PathBuf>,
}

fn gen_verilog(cx: &mut Context) -> Result<GenVerilog> {
    const RESET_PREFIX: &str = "RST_N";

    let bluespec_dir = cx.out_dir.join("bluespec");
    let verilog_dir = cx.out_dir.join("verilog");
    fs::create_dir_all(&bluespec_dir).with_context(|| anyhow!("creating {bluespec_dir:?}"))?;
    fs::create_dir_all(&verilog_dir).with_context(|| anyhow!("creating {verilog_dir:?}"))?;

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

    let verilog_path = cx
        .bsc
        .emit_verilog(EmitVerilog {
            bsv_path: &bsv_path,
            module: module.name,
            reset_prefix: RESET_PREFIX,
            bluespec_dir: &bluespec_dir,
            verilog_dir: &verilog_dir,
        })
        .context("lowering BSV to Verilog")?;

    Ok(GenVerilog {
        bindings,
        module_name: module.name.to_string(),
        bsv_path,
        verilog_path,
        verilog_library_dirs: vec![verilog_dir, cx.bsc.verilog_library_dir()?],
    })
}
