use std::{fs, mem, path::PathBuf};

use anyhow::{Context as _, Result, anyhow};
use front::elab;
use toolchain_api::{BitstreamInput, Toolchain};
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

    //     let input = BitstreamInput {
    //         board: cx.board,
    //         bindings: todo!(),
    //         module:
    //     };
    //
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
    bsv_path: PathBuf,
    verilog_path: PathBuf,
}

fn gen_verilog(cx: &mut Context) -> Result<GenVerilog> {
    let bdir = cx.out_dir.join("bluespec");
    let vdir = cx.out_dir.join("verilog");
    fs::create_dir_all(&bdir).with_context(|| anyhow!("creating {bdir:?}"))?;
    fs::create_dir_all(&vdir).with_context(|| anyhow!("creating {vdir:?}"))?;

    let platform = platform::from_board(cx.board);
    let storage = elab::Bump::new();
    let make_top = mem::replace(&mut cx.make_top, Box::new(|_| {}));
    let air_module = elab::elaborate(&storage, || make_top(platform));

    let (package, top) = bsv::from_air(&storage, &air_module).context("lowering AIR to BSV")?;
    let bsv_path = bdir.join(format!("{}.bsv", package.name));
    fs::write(&bsv_path, package.to_string())
        .with_context(|| anyhow!("writing BSV to {bsv_path:?}"))?;

    let v_path = cx
        .bsc
        .emit_verilog(&bsv_path, top.name, &bdir, &vdir)
        .context("lowering BSV to Verilog")?;

    Ok(GenVerilog {
        bsv_path,
        verilog_path: v_path,
    })
}
