use crate::{
    ir::{self, runtime},
    module::{In, Out},
    signal::Signal,
};

pub fn check_pins(pins: &[(&str, &str)]) -> Result<(), ir::Error> {
    runtime::with_design(|design| {
        let mut seen = std::collections::HashSet::new();
        for &(package_pin, io_standard) in pins {
            ir::require(
                !package_pin.trim().is_empty() && !io_standard.trim().is_empty(),
                "package pin and IO standard must be nonempty",
            )?;
            ir::require(
                seen.insert(package_pin)
                    && !design
                        .bindings
                        .iter()
                        .any(|binding| binding.package_pin == package_pin),
                format!("package pin {package_pin} is already bound"),
            )?;
        }
        Ok(())
    })
}

pub fn bind_pin(
    signal: ir::SignalId,
    package_pin: &str,
    io_standard: &str,
) -> Result<(), ir::Error> {
    check_pins(&[(package_pin, io_standard)])?;
    runtime::with_design(|design| {
        let port = design
            .signals
            .get(signal.0)
            .ok_or_else(|| ir::Error(format!("invalid physical signal {}", signal.0)))?;
        ir::require(
            port.ty.width == 1 && port.port.is_some(),
            "bound pin must be a width-1 physical port",
        )?;
        ir::require(
            !design
                .bindings
                .iter()
                .any(|binding| binding.signal == signal),
            format!("signal {} is already bound to a package pin", signal.0),
        )?;
        design.bindings.push(crate::PinBinding {
            signal,
            bit: 0,
            package_pin: package_pin.into(),
            io_standard: io_standard.into(),
        });
        Ok(())
    })
}

pub fn constrain_clock(signal: ir::SignalId, period: crate::util::Period) -> Result<(), ir::Error> {
    runtime::with_design(|design| {
        design.validate_clock(signal)?;
        ir::require(
            design
                .bindings
                .iter()
                .any(|binding| binding.signal == signal),
            "constrained clock must be bound to a physical pin",
        )?;
        ir::require(
            !design
                .clock_bindings
                .iter()
                .any(|binding| binding.signal == signal),
            format!("clock signal {} is already constrained", signal.0),
        )?;
        design
            .clock_bindings
            .push(crate::ClockBinding { signal, period });
        Ok(())
    })
}

pub fn bind_domain_clock<L: crate::module::DomainLabel>(
    clock: crate::signal::SignalId,
) -> Result<(), ir::Error> {
    runtime::with_design(|design| design.bind_domain_clock::<L>(clock))
}

pub fn bind_domain_reset<L: crate::module::DomainLabel>(
    reset: crate::signal::SignalId,
) -> Result<(), ir::Error> {
    runtime::with_design(|design| design.bind_domain_reset::<L>(reset))
}

pub fn design_id() -> ir::DesignId {
    runtime::owner()
}

#[track_caller]
pub fn input(name: &str, inverted: bool) -> (In<bool>, ir::SignalId) {
    let (signal, physical) = Signal::physical(name, ir::PortDirection::Input, inverted);
    (In::from_signal(signal), physical)
}

#[track_caller]
pub fn output(name: &str, inverted: bool) -> (Out<bool>, ir::SignalId) {
    let (signal, physical) = Signal::physical(name, ir::PortDirection::Output, inverted);
    (Out::from_signal(signal), physical)
}

#[track_caller]
pub fn clock(name: &str) -> (Signal<bool>, ir::SignalId) {
    Signal::physical(name, ir::PortDirection::Input, false)
}
