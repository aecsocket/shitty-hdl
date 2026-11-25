use super::*;
use shitty_hdl_board::{ActiveOn, arty_a7::A7_35T};
use shitty_hdl_core::signal::Signal;
use std::num::NonZero;

struct Top;
impl Module for Top {
    const NAME: &'static str = "../unsafe/top";
}

#[test]
fn artifacts_use_requested_resources_and_shared_capture() {
    let target = std::env::temp_dir().join(format!("shitty-hdl-cli-test-{}", std::process::id()));
    let output = build(&target, A7_35T, |platform| {
        let mut led = platform.leds.take(0);
        led <<= true;
        Top
    })
    .unwrap();
    assert_eq!(output.parent(), Some(target.as_path()));
    let source = std::fs::read_to_string(output.join("top.sv")).unwrap();
    let xdc = std::fs::read_to_string(output.join("top.xdc")).unwrap();
    assert!(source.contains("clock_0"));
    assert!(xdc.contains("PACKAGE_PIN \"E3\""));
    assert!(xdc.contains("PACKAGE_PIN \"H5\""));
    assert!(!xdc.contains("J5"));
    assert!(xdc.contains("create_clock -period 10 "));
    assert!(xdc.contains("clock_0\\[0\\]"));
    std::fs::remove_dir_all(target).unwrap();
}

#[test]
fn constraints_use_lowered_escaped_names_and_exact_periods() {
    let (design, _) =
        ir::elaborate_named("top", || Signal::<bool>::input("odd$name[port];x", false)).unwrap();
    let lowered = verilog::lower_with_ports(&design).unwrap();
    let signal = *lowered.port_names.keys().next().unwrap();
    let pins = [PinBinding {
        signal,
        bit: 0,
        package_pin: "E3\";[evil]$x".into(),
        io_standard: "LVCMOS33".into(),
    }];
    let clocks = [ClockBinding {
        signal,
        period: Period::from_femtos(NonZero::new(10_123_456).unwrap()),
    }];
    let rendered =
        render_constraints(Toolchain::Vivado, &pins, &clocks, &lowered.port_names).unwrap();
    assert!(rendered.contains("-period 10.123456"));
    assert!(rendered.contains("odd\\$name\\[port\\];x\\[0\\]"));
    assert!(rendered.contains("E3\\\";\\[evil\\]\\$x"));
    assert!(render_constraints(Toolchain::Vivado, &pins, &[], &HashMap::new()).is_err());
    assert_eq!(
        nanos(Period::from_femtos(NonZero::new(1).unwrap())),
        "0.000001"
    );
    assert_eq!(
        nanos(Period::from_femtos(NonZero::new(u64::MAX).unwrap())),
        "18446744073709.551615"
    );
}

#[test]
fn unsupported_toolchain_fails_before_elaboration() {
    let board = Board {
        toolchain: Toolchain::Quartus,
        ..A7_35T
    };
    assert!(build("unused", board, |_| -> Top { panic!("must not elaborate") }).is_err());
    assert!(render_constraints(Toolchain::IceStorm, &[], &[], &HashMap::new()).is_err());
}

#[test]
fn synchronous_demo_uses_platform_clock() {
    let target = std::env::temp_dir().join(format!("shitty-hdl-cli-sync-{}", std::process::id()));
    let output = build(&target, A7_35T, |platform| crate::blinky(platform)).unwrap();
    let source = std::fs::read_to_string(output.join("top.sv")).unwrap();
    assert!(source.contains("posedge clock_0)"), "{source}");
    std::fs::remove_dir_all(target).unwrap();
}

#[test]
fn physical_build_rejects_unbound_ports_and_domains() {
    let error = build("unused", A7_35T, |_| {
        let _input = Signal::<bool>::input("unbound", false);
        Top
    })
    .unwrap_err();
    assert!(error.to_string().contains("no physical pin binding"));

    struct Other;
    impl shitty_hdl_core::module::DomainLabel for Other {}
    let error = build("unused", A7_35T, |_| {
        shitty_hdl_core::module::Domain::<Other>::default().on(|| {});
        Top
    })
    .unwrap_err();
    assert!(error.to_string().contains("no physical clock binding"));
}

#[test]
fn metadata_and_safe_paths() {
    let platform = Platform::from_board(A7_35T).unwrap();
    assert_eq!(platform.default_clock_period.femtos(), 10_000_000);
    assert!(platform.bindings().is_empty());
    assert_eq!(A7_35T.resources.clock[0].pin.name, "E3");
    assert_eq!(A7_35T.resources.reset[0].active_on, ActiveOn::Low);
    for name in ["", ".", "..", "../../x", "/tmp/x", "a\\b"] {
        let safe = safe_name(name);
        assert_eq!(Path::new(&safe).components().count(), 1);
        assert!(!safe.contains(['/', '\\', '.']));
    }
}
