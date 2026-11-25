use super::*;
use shitty_hdl_board::{ActiveOn, arty_a7::A7_35T};
use shitty_hdl_core::signal::Signal;
use std::num::NonZero;

struct Top;
impl Module for Top {
    const NAME: &'static str = "../unsafe/top";
}

#[test]
fn artifacts_include_only_taken_resources() {
    let target = std::env::temp_dir().join(format!("shitty-hdl-cli-test-{}", std::process::id()));
    let output = build(&target, A7_35T, |mut platform| {
        let mut led = platform.led.take(0);
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
    assert!(!xdc.contains("PACKAGE_PIN \"J5\""));
    assert!(source.contains("led_0"));
    assert!(!source.contains("uart_"));
    let expected_pins = 2;
    assert_eq!(
        xdc.matches("set_property PACKAGE_PIN ").count(),
        expected_pins
    );
    assert_eq!(xdc.matches("create_clock ").count(), 1);
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
    let xdc = std::fs::read_to_string(output.join("top.xdc")).unwrap();
    for name in ["clock_0", "led_0", "button_0"] {
        assert!(source.contains(name), "{source}");
        assert!(xdc.contains(name), "{xdc}");
    }
    for name in ["reset_", "switch_", "led_1", "uart_"] {
        assert!(!source.contains(name), "{source}");
        assert!(!xdc.contains(name), "{xdc}");
    }
    assert_eq!(xdc.matches("set_property PACKAGE_PIN ").count(), 3);
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
fn combinational_build_without_default_oscillator() {
    let mut board = A7_35T;
    board.resources.default_clock = None;
    let target = std::env::temp_dir().join(format!("shitty-hdl-cli-comb-{}", std::process::id()));
    let output = build(&target, board, |mut platform| {
        assert!(platform.default_clock_hz.is_none());
        assert!(platform.elab.borrow().default_clock.is_none());
        assert!(platform.elab.borrow().clock_bindings.is_empty());
        assert!(platform.elab.borrow().bindings.is_empty());
        let mut led = platform.led.take(0);
        led <<= platform.button.take(0);
        Top
    })
    .unwrap();
    let source = std::fs::read_to_string(output.join("top.sv")).unwrap();
    let xdc = std::fs::read_to_string(output.join("top.xdc")).unwrap();
    assert!(!source.contains("clock_0"));
    assert!(!xdc.contains("create_clock"));
    assert!(!xdc.contains("PACKAGE_PIN \"E3\""));
    assert!(source.contains("led_0"));
    assert!(source.contains("button_0"));
    assert!(!source.contains("uart_"));
    assert!(!xdc.contains("uart_"));
    assert_eq!(xdc.matches("set_property PACKAGE_PIN ").count(), 2);
    std::fs::remove_dir_all(target).unwrap();

    let error = build("unused", board, |_| {
        shitty_hdl_core::module::Domain::<Sync>::default().on(|| {});
        Top
    })
    .unwrap_err();
    assert!(error.to_string().contains("no physical clock binding"));
}

#[test]
fn metadata_and_safe_paths() {
    ir::elaborate_named("metadata", || {
        let platform = shitty_hdl_platform::from_board(A7_35T);
        assert_eq!(platform.default_clock_hz.unwrap().get(), 100_000_000);
        let elab = platform.elab.borrow();
        assert_eq!(elab.clock_bindings.len(), 1);
        assert_eq!(elab.clock_bindings[0].period.femtos(), 10_000_000);
        assert_eq!(elab.bindings.len(), 1);
        assert_eq!(elab.bindings[0].package_pin, "E3");
        assert_eq!(elab.bindings[0].signal, elab.default_clock.unwrap().id());
    })
    .unwrap();
    assert_eq!(A7_35T.resources.clock[0].pin.name, "E3");
    assert_eq!(A7_35T.resources.reset[0].active_on, ActiveOn::Low);
    for name in ["", ".", "..", "../../x", "/tmp/x", "a\\b"] {
        let safe = safe_name(name);
        assert_eq!(Path::new(&safe).components().count(), 1);
        assert!(!safe.contains(['/', '\\', '.']));
    }
}
