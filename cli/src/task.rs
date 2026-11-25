use std::{fs, mem, path::PathBuf};

use anyhow::{Context as _, Result, anyhow};
use front::elab;
use toolchain_api::{Bindings, BitstreamInput, Toolchain};
use tracing::info;

use crate::{Context, START};

pub fn check(mut cx: Context) -> Result<()> {
    let verilog = gen_verilog(&mut cx)?;
    info!("generated Verilog in {:.2?}", START.elapsed());
    info!("- BSV output to {:?}", verilog.bsv_path);
    info!("- Verilog output to {:?}", verilog.verilog_path);
    Ok(())
}

pub fn build(mut cx: Context) -> Result<()> {
    let verilog = gen_verilog(&mut cx)?;
    info!("generated Verilog in {:.2?}", START.elapsed());

    let vivado_dir = cx.out_dir.join("vivado");
    fs::create_dir_all(&vivado_dir).with_context(|| anyhow!("creating {vivado_dir:?}"))?;

    let input = BitstreamInput {
        board: cx.board,
        bindings: Bindings::default(), // todo
        verilog_path: verilog.verilog_path,
        module: verilog.module_name,
        out_dir: vivado_dir,
    };

    info!("making bitstream");
    toolchain_vivado::Vivado::default()
        .make_bitstream(input, &mut |event| {
            info!("started stage {event:?}");
        })
        .context("making bitstream")?;

    //     toolchain_vivado::Vivado::default().make_bitstream(input, progress)
    //
    //     info!("- BSV output to {:?}", verilog.bsv_path);
    //     info!("- Verilog output to {:?}", verilog.verilog_path);
    Ok(())
}

pub fn upload(cx: Context) -> Result<()> {
    todo!();
}

struct GenVerilog {
    module_name: String,
    bsv_path: PathBuf,
    verilog_path: PathBuf,
}

fn gen_verilog(cx: &mut Context) -> Result<GenVerilog> {
    let bluespec_dir = cx.out_dir.join("bluespec");
    let verilog_dir = cx.out_dir.join("verilog");
    fs::create_dir_all(&bluespec_dir).with_context(|| anyhow!("creating {bluespec_dir:?}"))?;
    fs::create_dir_all(&verilog_dir).with_context(|| anyhow!("creating {verilog_dir:?}"))?;

    let platform = platform::from_board(cx.board);
    let storage = elab::Bump::new();
    let make_top = mem::replace(&mut cx.make_top, Box::new(|_| {}));
    let air_module = elab::elaborate(&storage, || make_top(platform));

    let (package, module) = bsv::from_air(&storage, &air_module).context("lowering AIR to BSV")?;
    let bsv_path = bluespec_dir.join(format!("{}.bsv", package.name));
    fs::write(&bsv_path, package.to_string())
        .with_context(|| anyhow!("writing BSV to {bsv_path:?}"))?;

    let v_path = cx
        .bsc
        .emit_verilog(&bsv_path, module.name, &bluespec_dir, &verilog_dir)
        .context("lowering BSV to Verilog")?;

    Ok(GenVerilog {
        module_name: module.name.to_string(),
        bsv_path,
        verilog_path: v_path,
    })
}
