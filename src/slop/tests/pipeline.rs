use shitty_hdl::{elaborate, prelude::*, sim::Simulator, verilog};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    process::Command,
};

struct A;
impl DomainLabel for A {}
struct B;
impl DomainLabel for B {}

#[module]
fn counter(
    mut clk: Domain<Sync>,
    enable: In<bool>,
    reset: In<bool>,
    mut count: Out<Uint<U3>>,
    mut doubled: Out<Uint<U4>>,
) {
    clk.on(|| {
        if reset {
            count <<= u!(0);
        } else if enable {
            count <<= count + u!(1);
        }
    });
    comb(|| doubled <<= zext(count) * u!(2));
}

#[module]
fn choices(a: In<bool>, b: In<bool>, mut priority: Out<Uint<U8>>, mut selected: Out<Uint<U8>>) {
    comb(|| {
        priority <<= u!(99);
        if a {
            priority <<= u!(1);
            priority <<= u!(2);
        } else if b {
            priority <<= u!(3);
        } else {
            priority <<= u!(4);
        }
        let value = if a {
            if b { u!(10) } else { u!(11) }
        } else if b {
            u!(12)
        } else {
            u!(13)
        };
        selected <<= value;
    });
}

#[derive(Debug, Clone, PartialEq, Eq, Shape)]
struct Pair {
    lo: Uint<U8>,
    hi: Uint<U8>,
}

#[test]
fn typed_module_counter_enable_reset_and_wrap() {
    let (design, dut) = elaborate(counter).unwrap();
    assert_eq!(design.domains.len(), 1);
    let mut sim = Simulator::from_design(design).unwrap();
    assert_eq!(sim.get(dut.count).unwrap(), u!(0));
    assert_eq!(sim.get(dut.doubled).unwrap(), u!(0));
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(dut.count).unwrap(), u!(0));
    sim.set(dut.enable, true).unwrap();
    for tick in 1..=10 {
        sim.tick_domain::<Sync>().unwrap();
        let expected = tick % 8;
        assert_eq!(sim.get(dut.count).unwrap().words(), &[expected]);
        assert_eq!(sim.get(dut.doubled).unwrap().words(), &[expected * 2]);
    }
    sim.set(dut.enable, false).unwrap();
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(dut.count).unwrap(), u!(2));
    sim.set(dut.enable, true).unwrap();
    sim.set(dut.reset, true).unwrap();
    assert_eq!(sim.get(dut.count).unwrap(), u!(2), "reset is synchronous");
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(dut.count).unwrap(), u!(0));
    assert_eq!(sim.get(dut.doubled).unwrap(), u!(0));
    sim.set(dut.reset, false).unwrap();
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(dut.count).unwrap(), u!(1));
}

#[test]
fn assignment_priority_and_nested_value_conditionals() {
    let (design, dut) = elaborate(|| choices()).unwrap();
    let mut sim = Simulator::from_design(design).unwrap();
    for (a, b, priority, selected) in [
        (false, false, 4, 13),
        (false, true, 3, 12),
        (true, false, 2, 11),
        (true, true, 2, 10),
        (false, false, 4, 13),
    ] {
        sim.set(dut.a, a).unwrap();
        sim.set(dut.b, b).unwrap();
        assert_eq!(sim.get(dut.priority).unwrap().words(), &[priority]);
        assert_eq!(sim.get(dut.selected).unwrap().words(), &[selected]);
    }
}

#[test]
fn composite_packing_views_and_partial_feedforward() {
    let value = Pair {
        lo: u!(0x12),
        hi: u!(0x34),
    };
    assert_eq!(value.to_bits().words(), &[0x3412]);
    assert_eq!(Pair::from_bits(&value.to_bits()), value);
    let (design, (input, output)) = shitty_hdl::ir::elaborate("feedforward", || {
        let input = Signal::<Pair>::input("input_pair", value);
        let mut output = Signal::<Pair>::without_init();
        comb(|| {
            output.lo <<= input.lo;
            let next_hi = output.lo + u!(1);
            output.hi <<= next_hi;
        });
        (input, output)
    })
    .unwrap();
    let mut sim = Simulator::from_design(design).unwrap();
    assert_eq!(
        sim.get(output).unwrap(),
        Pair {
            lo: u!(0x12),
            hi: u!(0x13)
        }
    );
    sim.set_view(input.hi, Uint::<U8>::from(0xffu8)).unwrap();
    assert_eq!(sim.get_view(input.lo).unwrap(), u!(0x12));
    assert_eq!(sim.get_view(input.hi).unwrap(), u!(0xff));
    sim.set_view(input.lo, 0xffu8).unwrap();
    assert_eq!(
        sim.get(output).unwrap(),
        Pair {
            lo: u!(0xff),
            hi: u!(0)
        }
    );
}

#[test]
fn partial_clocked_fields_swap_from_old_state() {
    let (design, (_clk, pair)) = shitty_hdl::ir::elaborate("field_swap", || {
        let mut clk = Domain::<Sync>::new();
        let mut pair = Signal::<Pair>::new(Pair {
            lo: u!(1),
            hi: u!(2),
        });
        clk.on(|| {
            let old_lo = pair.lo;
            let old_hi = pair.hi;
            pair.lo <<= old_hi;
            pair.hi <<= old_lo;
        });
        (clk, pair)
    })
    .unwrap();
    let mut sim = Simulator::from_design(design).unwrap();
    for (lo, hi) in [(2, 1), (1, 2), (2, 1)] {
        sim.tick_domain::<Sync>().unwrap();
        assert_eq!(sim.get_view(pair.lo).unwrap().words(), &[lo]);
        assert_eq!(sim.get_view(pair.hi).unwrap().words(), &[hi]);
    }
}

#[test]
fn independent_and_simultaneous_clock_domains() {
    let (design, (a_clk, b_clk, a, b)) = shitty_hdl::ir::elaborate("domain_swap", || {
        let mut a_clk = Domain::<A>::new();
        let mut b_clk = Domain::<B>::new();
        let mut a = Signal::<Uint<U8>>::new(u!(1));
        let mut b = Signal::<Uint<U8>>::new(u!(2));
        a_clk.on(|| a <<= b);
        b_clk.on(|| b <<= a);
        (a_clk, b_clk, a, b)
    })
    .unwrap();
    assert_ne!(a_clk.id(), b_clk.id());
    let mut simultaneous = Simulator::from_design(design.clone()).unwrap();
    simultaneous.step(&[a_clk.id(), b_clk.id()]).unwrap();
    assert_eq!(simultaneous.get(a).unwrap(), u!(2));
    assert_eq!(simultaneous.get(b).unwrap(), u!(1));
    simultaneous.step(&[a_clk.id(), b_clk.id()]).unwrap();
    assert_eq!(simultaneous.get(a).unwrap(), u!(1));
    assert_eq!(simultaneous.get(b).unwrap(), u!(2));
    let mut sequential = Simulator::from_design(design).unwrap();
    sequential.tick_domain::<A>().unwrap();
    assert_eq!(sequential.get(a).unwrap(), u!(2));
    assert_eq!(sequential.get(b).unwrap(), u!(2));
    sequential.tick_domain::<B>().unwrap();
    assert_eq!(sequential.get(b).unwrap(), u!(2));
}

#[test]
fn invalid_cross_domain_drivers_and_comb_feedback_are_rejected() {
    let conflict = shitty_hdl::ir::elaborate("conflict", || {
        let mut a = Domain::<A>::new();
        let mut b = Domain::<B>::new();
        let mut signal = Signal::<Uint<U8>>::new(u!(0));
        a.on(|| signal <<= u!(1));
        b.on(|| signal <<= u!(2));
    });
    assert!(
        conflict.is_err(),
        "two domains must not drive the same bits"
    );
    let feedback = shitty_hdl::ir::elaborate("feedback", || {
        let mut signal = Signal::<Uint<U8>>::new(u!(0));
        comb(|| signal <<= signal + u!(1));
    });
    assert!(
        feedback.is_err(),
        "combinational self-feedback must be rejected"
    );
}

#[test]
fn unwind_restores_elaboration_domain_and_condition_context() {
    assert!(catch_unwind(|| shitty_hdl::ir::elaborate("panic", || panic!("intentional"))).is_err());
    assert!(catch_unwind(|| Signal::<bool>::new(false)).is_err());
    let (design, (_clk, output)) = shitty_hdl::ir::elaborate("restored", || {
        let mut clk = Domain::<Sync>::new();
        let mut output = Signal::<Uint<U8>>::new(u!(0));
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                clk.on(|| {
                    if_(false, || -> () {
                        panic!("inside condition and clock scope")
                    });
                });
            }))
            .is_err()
        );
        assert!(
            catch_unwind(|| {
                let _ = shitty_hdl::ir::elaborate("nested_panic", || panic!("nested"));
            })
            .is_err()
        );
        comb(|| output <<= u!(7));
        (clk, output)
    })
    .unwrap();
    let mut sim = Simulator::from_design(design).unwrap();
    assert_eq!(sim.get(output).unwrap(), u!(7));
    sim.tick_domain::<Sync>().unwrap();
    assert_eq!(sim.get(output).unwrap(), u!(7));
}

#[test]
fn cross_design_signals_are_rejected_but_domain_labels_are_shared() {
    let mut foreign_clk = Domain::<Sync>::new();
    let local_clk = Domain::<Sync>::default();
    assert_eq!(foreign_clk.id(), local_clk.id());
    assert_eq!(foreign_clk.id(), shitty_hdl::ir::DomainId::of::<Sync>());
    let (first, foreign) = shitty_hdl::ir::elaborate("first", || {
        foreign_clk.on(|| {});
        Signal::<Uint<U8>>::input("input_a", u!(1))
    })
    .unwrap();
    let (second, local) = shitty_hdl::ir::elaborate("second", || {
        let mut clk = local_clk;
        clk.on(|| {});
        Signal::<Uint<U8>>::input("input_b", u!(2))
    })
    .unwrap();
    let mut sim = Simulator::from_design(second).unwrap();
    assert!(sim.get(foreign).is_err());
    assert!(sim.set(foreign, 3u8).is_err());
    assert!(sim.get_view(*foreign).is_err());
    assert!(sim.set_view(*foreign, 3u8).is_err());
    sim.tick(foreign_clk.id()).unwrap();
    sim.tick_domain::<Sync>().unwrap();
    sim.step(&[local_clk.id(), foreign_clk.id()]).unwrap();
    assert!(sim.tick_domain::<A>().is_err());
    assert!(
        sim.step(&[local_clk.id(), Domain::<A>::new().id()])
            .is_err()
    );
    assert_eq!(sim.get(local).unwrap(), u!(2));
    assert!(
        catch_unwind(|| {
            let _ = shitty_hdl::ir::elaborate("illegal_read", || foreign + u!(1));
        })
        .is_err()
    );
    let (third, ()) = shitty_hdl::ir::elaborate("shared_domain", || foreign_clk.on(|| {})).unwrap();
    assert_eq!(third.domains.len(), 1);
    assert_eq!(third.domains[0].id, local_clk.id());
    assert_eq!(
        Simulator::from_design(first).unwrap().get(foreign).unwrap(),
        u!(1)
    );
}

#[test]
fn same_label_handles_are_copyable_and_discovered_once_on_use() {
    fn assert_copy_clone<T: Copy + Clone>() {}
    assert_copy_clone::<Domain<A>>();
    let mut first = Domain::<A>::new();
    let mut second = Domain::<A>::default();
    let unused = Domain::<B>::new();
    assert_eq!(first.id(), second.id());
    assert_ne!(first.id(), unused.id());
    let (design, output) = shitty_hdl::ir::elaborate("deduplicated", || {
        let mut output = Signal::<Uint<U8>>::new(u!(0));
        first.on(|| output <<= u!(1));
        second.on(|| output <<= u!(2));
        output
    })
    .unwrap();
    assert_eq!(design.domains.len(), 1);
    assert_eq!(design.domains[0].id, first.id());
    let mut sim = Simulator::from_design(design).unwrap();
    sim.tick_domain::<A>().unwrap();
    assert_eq!(sim.get(output).unwrap(), u!(2));
    assert!(sim.tick(unused.id()).is_err());
}

#[test]
fn wide_unsigned_arithmetic_and_signed_resize_comparison() {
    let (design, (a, b, sum, product, difference, negative, extended, narrowed, less)) =
        shitty_hdl::ir::elaborate("wide", || {
            let a = Signal::<Uint<U128>>::input("a", Uint::from(u64::MAX as u128));
            let b = Signal::<Uint<U128>>::input("b", Uint::from(2u128));
            let mut sum = Signal::<Uint<U128>>::without_init();
            let mut product = Signal::<Uint<U128>>::without_init();
            let mut difference = Signal::<Uint<U128>>::without_init();
            let negative = Signal::<Sint<U8>>::input("negative", -2i8);
            let mut extended = Signal::<Sint<U128>>::without_init();
            let mut narrowed = Signal::<Sint<U8>>::without_init();
            let mut less = Signal::<bool>::without_init();
            comb(|| {
                sum <<= a + b;
                product <<= a * b;
                difference <<= b - a;
                extended <<= sext(negative);
                narrowed <<= trunc(extended);
                less <<= extended.lt(Sint::<U128>::from(1i8));
            });
            (
                a, b, sum, product, difference, negative, extended, narrowed, less,
            )
        })
        .unwrap();
    assert!(!verilog::lower(&design).unwrap().is_empty());
    let mut sim = Simulator::from_design(design).unwrap();
    for (av, bv) in [
        (u64::MAX as u128, 2u128),
        (u128::MAX, 1),
        (1u128 << 100, 17),
    ] {
        sim.set(a, av).unwrap();
        sim.set(b, bv).unwrap();
        assert_eq!(sim.get(sum).unwrap(), Uint::from(av.wrapping_add(bv)));
        assert_eq!(sim.get(product).unwrap(), Uint::from(av.wrapping_mul(bv)));
        assert_eq!(
            sim.get(difference).unwrap(),
            Uint::from(bv.wrapping_sub(av))
        );
    }
    for value in [-128i8, -2, -1, 0, 1, 127] {
        sim.set(negative, value).unwrap();
        assert_eq!(sim.get(extended).unwrap(), Sint::<U128>::from(value));
        assert_eq!(sim.get(narrowed).unwrap(), Sint::<U8>::from(value));
        assert_eq!(sim.get(less).unwrap(), value < 1);
    }
}

#[test]
fn zero_width_frontend_simulation_and_backend() {
    let (design, (_clk, input, output, state)) = shitty_hdl::ir::elaborate("zero_width", || {
        let mut clk = Domain::<Sync>::new();
        let input = Signal::<Uint<U0>>::input("empty_input", u!(0));
        let mut output = Signal::<Uint<U0>>::output("empty_output", u!(0));
        let mut state = Signal::<Uint<U0>>::new(u!(0));
        comb(|| output <<= input + u!(0));
        clk.on(|| state <<= input);
        (clk, input, output, state)
    })
    .unwrap();
    let sv = verilog::lower(&design).unwrap();
    assert!(!sv.contains("[-1:"));
    let mut sim = Simulator::from_design(design).unwrap();
    sim.set(input, Uint::<U0>::constant::<0>()).unwrap();
    sim.tick_domain::<Sync>().unwrap();
    for signal in [input, output, state] {
        assert!(sim.get(signal).unwrap().words().is_empty());
    }
}

#[test]
fn generated_verilog_parses_and_optionally_lints() {
    let (design, _) = elaborate(counter).unwrap();
    // lower() parses and formats its generated SystemVerilog internally.
    let sv = verilog::lower(&design).unwrap();
    assert!(sv.contains(&format!("module \\{} ", design.name)));
    assert!(sv.contains("posedge"));
    let dir = std::env::temp_dir().join(format!("shitty_hdl_pipeline_{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(dir.clone());
    std::fs::write(dir.join("pipeline.sv"), &sv).unwrap();
    let yosys_script = format!(
        "read_verilog -sv pipeline.sv; hierarchy -check -top \\{} ; proc; check -assert",
        design.name
    );
    for (tool, args) in [
        ("yosys", vec!["-Q", "-T", "-p", yosys_script.as_str()]),
        (
            "verilator",
            vec![
                "--lint-only",
                "--Wno-fatal",
                "--top-module",
                design.name.as_str(),
                "pipeline.sv",
            ],
        ),
    ] {
        match Command::new(tool).args(args).current_dir(&dir).output() {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("skipping {tool}: not installed")
            }
            Err(error) => panic!("could not run {tool}: {error}"),
            Ok(output) => assert!(
                output.status.success(),
                "{tool} failed:\n{}\n{}\nGenerated SV:\n{sv}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        }
    }
}
