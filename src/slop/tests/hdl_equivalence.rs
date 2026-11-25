use shitty_hdl::{
    elaborate,
    ir::{self, BinaryOp as B, PackedBits, PortDirection as P, ValueKind as V, ValueType},
    prelude::*,
    sim::Simulator,
    verilog,
};
use std::{
    fmt::Write,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

#[module]
fn equivalence_counter(
    mut clk: Domain<Sync>,
    enable: In<bool>,
    reset: In<bool>,
    mut count: Out<Uint<U3>>,
    mut doubled: Out<Uint<U4>>,
) {
    clk.on(|| {
        if enable {
            count <<= count + u!(1);
        }
        // A later reset assignment must override the enabled increment.
        if reset {
            count <<= u!(0);
        }
    });
    comb(|| doubled <<= zext(count) * u!(2));
}

fn literal(bits: &PackedBits) -> String {
    let digits: String = (0..bits.width())
        .rev()
        .map(|i| if bits.bit(i) { '1' } else { '0' })
        .collect();
    format!("{}'b{digits}", bits.width())
}

fn bench(design: &ir::Design) -> String {
    let mut tb = String::from("module tb;\n");
    let mut ports = Vec::new();
    for signal in &design.signals {
        if signal.port.is_some() {
            writeln!(
                tb,
                "logic [{}:0] {} = {};",
                signal.ty.width - 1,
                signal.name,
                literal(&signal.init)
            )
            .unwrap();
            ports.push(format!(".{}({})", signal.name, signal.name));
        }
    }
    for clock in &design.domains {
        writeln!(tb, "logic {} = 0;", clock.name).unwrap();
        ports.push(format!(".{}({})", clock.name, clock.name));
    }
    writeln!(
        tb,
        "\\{} dut({});\ninitial begin\n#1;",
        design.name,
        ports.join(", ")
    )
    .unwrap();
    tb
}

fn checks(tb: &mut String, sim: &Simulator, case: &str) {
    for (i, signal) in sim.design().signals.iter().enumerate() {
        if signal.port == Some(P::Output) {
            let expected = literal(sim.get_signal(ir::SignalId(i)).unwrap());
            writeln!(
        tb,
        "if ({} !== {expected}) $fatal(1, \"{case}: {} expected %b got %b\", {expected}, {});",
        signal.name, signal.name, signal.name
      )
      .unwrap();
        }
    }
}

fn execute(design: &ir::Design, mut tb: String) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let scratch = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!(
            "hdl_equivalence_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&scratch).unwrap();
    tb.push_str("$display(\"behavioral equivalence passed\");\n$finish;\nend\nendmodule\n");
    std::fs::write(scratch.join("design.sv"), verilog::lower(design).unwrap()).unwrap();
    std::fs::write(scratch.join("tb.sv"), tb).unwrap();
    // Keep failed artifacts for reproducing exact simulator/backend
    // discrepancies.
    let build = Command::new("verilator")
        .args([
            "--binary",
            "--timing",
            "--assert",
            "--Wno-fatal",
            "--top-module",
            "tb",
            "--Mdir",
            "obj",
            "design.sv",
            "tb.sv",
        ])
        .current_dir(&scratch)
        .output()
        .expect("requires Verilator and a C++ toolchain");
    assert!(
        build.status.success(),
        "Verilator build failed in {}:\n{}\n{}",
        scratch.display(),
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(scratch.join("obj/Vtb"))
        .current_dir(&scratch)
        .output()
        .unwrap();
    println!("{}: {}", design.name, String::from_utf8_lossy(&run.stdout));
    assert!(
        run.status.success(),
        "behavioral mismatch in {}:\n{}\n{}",
        scratch.display(),
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::remove_dir_all(scratch).unwrap();
}

#[test]
#[ignore = "requires Verilator and a C++ toolchain"]
fn frontend_clocked_counter() {
    let (design, dut) = elaborate(equivalence_counter).unwrap();
    let mut tb = bench(&design);
    let mut sim = Simulator::from_design(design.clone()).unwrap();
    checks(&mut tb, &sim, "initial");
    let inputs: Vec<_> = [(false, false), (true, true), (false, true)]
        .into_iter()
        .chain(std::iter::repeat_n((true, false), 10))
        .chain([(false, false), (true, true), (true, false)])
        .collect();
    let inputs_names: Vec<_> = design
        .signals
        .iter()
        .filter(|s| s.port == Some(P::Input))
        .map(|s| s.name.clone())
        .collect();
    assert_eq!(inputs_names.len(), 2);
    for (cycle, (enable, reset)) in inputs.into_iter().enumerate() {
        sim.set(dut.enable, enable).unwrap();
        sim.set(dut.reset, reset).unwrap();
        writeln!(
            tb,
            "{} = 1'b{}; {} = 1'b{}; #1;",
            inputs_names[0],
            u8::from(enable),
            inputs_names[1],
            u8::from(reset)
        )
        .unwrap();
        checks(&mut tb, &sim, &format!("cycle {cycle} before edge"));
        sim.tick_domain::<Sync>().unwrap();
        writeln!(tb, "{} = 1; #1;", design.domains[0].name).unwrap();
        checks(&mut tb, &sim, &format!("cycle {cycle} after edge"));
        writeln!(tb, "{} = 0; #1;", design.domains[0].name).unwrap();
    }
    execute(&design, tb);
}

fn ty(width: u32, signed: bool) -> ValueType {
    ValueType { width, signed }
}
fn input(d: &mut ir::Design, name: &str, t: ValueType) -> (ir::SignalId, ir::ValueId) {
    let s = d.add_signal(ir::Signal {
        name: name.into(),
        ty: t,
        init: PackedBits::zero(t.width),
        port: Some(P::Input),
        source: None,
    });
    let v = d.add_value(t, V::Read(s), None);
    (s, v)
}
fn output(d: &mut ir::Design, name: String, t: ValueType, kind: V) {
    let value = d.add_value(t, kind, None);
    let signal = d.add_signal(ir::Signal {
        name,
        ty: t,
        init: PackedBits::zero(t.width),
        port: Some(P::Output),
        source: None,
    });
    d.writes.push(ir::Write {
        target: ir::Target {
            signal,
            offset: 0,
            width: t.width,
        },
        value,
        condition: None,
        domain: ir::WriteDomain::Comb,
        source: None,
    });
}

#[test]
#[ignore = "requires Verilator and a C++ toolchain"]
fn raw_ir_operations() {
    let mut d = ir::Design::new("operations_equivalence");
    let (select, cond) = input(&mut d, "select_input", ty(1, false));
    let (shift, shift_value) = input(&mut d, "shift_input", ty(130, false));
    let mut operands = Vec::new();
    for width in [1, 65, 130] {
        for signed in [false, true] {
            let t = ty(width, signed);
            let prefix = format!("w{width}_{}", if signed { "s" } else { "u" });
            let (a, av) = input(&mut d, &format!("{prefix}_a"), t);
            let (b, bv) = input(&mut d, &format!("{prefix}_b"), t);
            operands.push((a, b, width));
            for op in [
                B::Add,
                B::Sub,
                B::Mul,
                B::Eq,
                B::Ne,
                B::Lt,
                B::Le,
                B::Gt,
                B::Ge,
                B::And,
                B::Or,
                B::Xor,
                B::Shl,
                B::Shr,
                B::ArithmeticShr,
            ] {
                let comparison = matches!(op, B::Eq | B::Ne | B::Lt | B::Le | B::Gt | B::Ge);
                let rhs = if matches!(op, B::Shl | B::Shr | B::ArithmeticShr) {
                    shift_value
                } else {
                    bv
                };
                output(
                    &mut d,
                    format!("{prefix}_{op:?}"),
                    if comparison { ty(1, false) } else { t },
                    V::Binary { op, lhs: av, rhs },
                );
            }
            for op in [ir::UnaryOp::Not, ir::UnaryOp::Neg] {
                output(
                    &mut d,
                    format!("{prefix}_{op:?}"),
                    t,
                    V::Unary { op, value: av },
                );
            }
            output(
                &mut d,
                format!("{prefix}_mux"),
                t,
                V::Mux {
                    condition: cond,
                    then_value: av,
                    else_value: bv,
                },
            );
            let extended_ty = ty(width * 2, signed);
            let ax = d.add_value(
                extended_ty,
                V::Extend {
                    value: av,
                    sign: signed,
                },
                None,
            );
            let bx = d.add_value(
                extended_ty,
                V::Extend {
                    value: bv,
                    sign: signed,
                },
                None,
            );
            output(
                &mut d,
                format!("{prefix}_extended"),
                extended_ty,
                V::Extend {
                    value: av,
                    sign: signed,
                },
            );
            output(
                &mut d,
                format!("{prefix}_full_product"),
                extended_ty,
                V::Binary {
                    op: B::Mul,
                    lhs: ax,
                    rhs: bx,
                },
            );
            output(
                &mut d,
                format!("{prefix}_trunc"),
                t,
                V::Slice {
                    value: ax,
                    offset: 0,
                },
            );
            let bit = d.add_value(ty(1, false), V::Constant(PackedBits::from_u64(1, 1)), None);
            output(
                &mut d,
                format!("{prefix}_insert"),
                t,
                V::Insert {
                    base: av,
                    value: bit,
                    offset: width / 2,
                },
            );
        }
    }
    let mut tb = bench(&d);
    let mut sim = Simulator::from_design(d.clone()).unwrap();
    checks(&mut tb, &sim, "initial");
    for case in 0..12 {
        let mut values = vec![
            (select, PackedBits::from_u64(1, case % 2)),
            (
                shift,
                match case {
                    0..=6 => PackedBits::from_u64(130, [0, 1, 63, 64, 65, 129, 130][case as usize]),
                    7 => PackedBits::from_u64(130, 131),
                    8 => PackedBits::from_u64(130, u64::MAX),
                    _ => PackedBits::from_words(130, vec![0, 1, 1]),
                },
            ),
        ];
        for &(a, b, width) in &operands {
            let patterns = [
                vec![0, 0, 0],
                vec![u64::MAX; 3],
                vec![1, 0, 0],
                vec![0, 0, 2],
                vec![0xaaaa_aaaa_aaaa_aaaa; 3],
                vec![0x5555_5555_5555_5555; 3],
            ];
            values.push((
                a,
                PackedBits::from_words(width, patterns[case as usize % patterns.len()].clone()),
            ));
            values.push((
                b,
                PackedBits::from_words(
                    width,
                    patterns[(case as usize + 1) % patterns.len()].clone(),
                ),
            ));
        }
        for (id, bits) in values {
            writeln!(tb, "{} = {};", d.signals[id.0].name, literal(&bits)).unwrap();
            sim.set_signal(id, bits).unwrap();
        }
        tb.push_str("#1;\n");
        checks(&mut tb, &sim, &format!("vector {case}"));
    }
    execute(&d, tb);
}
