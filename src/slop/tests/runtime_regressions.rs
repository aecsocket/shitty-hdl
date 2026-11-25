use shitty_hdl::{prelude::*, sim::Simulator};

struct A;
impl DomainLabel for A {}
struct B;
impl DomainLabel for B {}

#[test]
fn design_ids_are_unique_and_clones_preserve_identity() {
    use shitty_hdl::ir::{Design, DesignId};
    let design = Design::new("first");
    let id: DesignId = design.id;
    assert_eq!(design.clone().id, id);
    let other = Design::new("second");
    assert_ne!(other.id, id);
    let ids: std::collections::HashSet<_> = [id, design.clone().id, other.id].into_iter().collect();
    assert_eq!(ids.len(), 2);
}

#[test]
fn constant_dead_branches_do_not_create_combinational_cycles() {
    let (design, output) = shitty_hdl::ir::elaborate("dead_branch", || {
        let mut output = Signal::<Uint<U8>>::output("output", u!(0));
        let _ = if_(true, || {
            output <<= u!(7);
        })
        .else_(|| {
            output <<= output + u!(1);
        });
        output
    })
    .unwrap();
    let sim = Simulator::from_design(design.clone()).unwrap();
    assert_eq!(sim.get(output).unwrap().words(), &[7]);
    shitty_hdl::verilog::lower(&design).unwrap();
}

#[test]
fn simulator_cannot_override_internal_constants_or_outputs() {
    let (design, (internal, output)) = shitty_hdl::ir::elaborate("constants", || {
        (
            Signal::<Uint<U8>>::new(u!(3)),
            Signal::<Uint<U8>>::output("output", u!(4)),
        )
    })
    .unwrap();
    let mut sim = Simulator::from_design(design).unwrap();
    assert!(sim.set(internal, 9u8).is_err());
    assert!(sim.set(output, 9u8).is_err());
    assert_eq!(sim.get(internal).unwrap().words(), &[3]);
    assert_eq!(sim.get(output).unwrap().words(), &[4]);
}

#[test]
fn caught_branch_panics_restore_the_outer_guard() {
    let (design, output) = shitty_hdl::ir::elaborate("guard_restore", || {
        let condition = Signal::<bool>::input("condition", false);
        let mut output = Signal::<Uint<U8>>::output("output", u!(0));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = if_(condition, || -> () {
                panic!("test branch panic");
            });
        }));
        assert!(result.is_err());
        output <<= u!(9);
        output
    })
    .unwrap();
    let sim = Simulator::from_design(design).unwrap();
    assert_eq!(sim.get(output).unwrap().words(), &[9]);
}

#[test]
fn unreachable_writes_do_not_claim_a_clock_domain() {
    let (design, (_a, _b, output)) = shitty_hdl::ir::elaborate("dead_clock", || {
        let mut a = Domain::<A>::new();
        let mut b = Domain::<B>::new();
        let mut output = Signal::<Uint<U8>>::output("output", u!(0));
        let _ = if_(false, || a.on(|| output <<= u!(3))).else_(|| b.on(|| output <<= u!(7)));
        (a, b, output)
    })
    .unwrap();
    let mut sim = Simulator::from_design(design).unwrap();
    sim.tick_domain::<A>().unwrap();
    assert_eq!(sim.get(output).unwrap().words(), &[0]);
    sim.tick_domain::<B>().unwrap();
    assert_eq!(sim.get(output).unwrap().words(), &[7]);
}

#[test]
fn source_paths_cannot_terminate_generated_comments() {
    use shitty_hdl::ir::{Design, PackedBits, PortDirection, Signal, SourceLocation, ValueType};
    let mut design = Design::new("source_path");
    design.add_signal(Signal {
        name: "output".into(),
        ty: ValueType {
            width: 8,
            signed: false,
        },
        init: PackedBits::from_u64(8, 3),
        port: Some(PortDirection::Output),
        source: Some(SourceLocation {
            file: "src*/counter.rs".into(),
            line: 1,
            column: 1,
        }),
    });
    let text = shitty_hdl::verilog::lower(&design).unwrap();
    assert!(text.contains("\\052/"));
}
