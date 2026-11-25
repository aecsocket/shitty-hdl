use std::collections::{HashMap, HashSet};

use crate::{ir, util::Period, verilog};

#[path = "vivado.rs"]
mod vivado;

#[derive(Debug, Hash)]
pub struct LowerResult {
    pub top_sv: String,
    pub top_xdc: Option<String>,
    pub build_tcl: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinBinding {
    pub signal: ir::SignalId,
    pub bit: u32,
    pub package_pin: String,
    pub io_standard: String,
}

#[derive(Debug, Clone)]
pub struct ClockBinding {
    pub signal: ir::SignalId,
    pub period: Period,
}

#[derive(Debug)]
pub enum Toolchain<'a> {
    Verilog,
    Vivado {
        part: &'a str,

        design_properties: &'a [(&'a str, &'a str)],
    },
}

pub fn lower(design: &ir::Design, toolchain: Toolchain<'_>) -> Result<LowerResult, ir::Error> {
    let lowered = verilog::lower_with_ports(design)?;
    let (top_xdc, build_tcl) = match toolchain {
        Toolchain::Verilog => (None, None),
        Toolchain::Vivado {
            part,
            design_properties,
        } => {
            validate_bindings(design, &design.bindings, &design.clock_bindings)?;
            (
                Some(constraints(
                    &design.bindings,
                    &design.clock_bindings,
                    &lowered.port_names,
                )?),
                Some(vivado::build_script(&design.name, part, design_properties)?),
            )
        }
    };
    Ok(LowerResult {
        top_sv: lowered.source,
        top_xdc,
        build_tcl,
    })
}

fn validate_bindings(
    design: &ir::Design,
    bindings: &[PinBinding],
    clocks: &[ClockBinding],
) -> Result<(), ir::Error> {
    let mut ports = HashSet::new();
    let mut pins = HashSet::new();
    for binding in bindings {
        let signal = design
            .signals
            .get(binding.signal.0)
            .ok_or_else(|| ir::Error(format!("invalid bound signal {}", binding.signal.0)))?;
        if signal.port.is_none() || binding.bit >= signal.ty.width {
            return Err(ir::Error(format!(
                "invalid physical port binding for {} bit {}",
                signal.name, binding.bit
            )));
        }
        if binding.package_pin.is_empty() || binding.io_standard.is_empty() {
            return Err(ir::Error("empty package pin or IO standard".into()));
        }
        if !ports.insert((binding.signal, binding.bit)) || !pins.insert(&binding.package_pin) {
            return Err(ir::Error("duplicate port or package pin binding".into()));
        }
    }
    for (index, signal) in design.signals.iter().enumerate() {
        if signal.port.is_some() {
            for bit in 0..signal.ty.width {
                if !ports.contains(&(ir::SignalId(index), bit)) {
                    return Err(ir::Error(format!(
                        "top-level port {} bit {bit} has no physical pin binding",
                        signal.name
                    )));
                }
            }
        }
    }
    let mut constrained_clocks = HashSet::new();
    for clock in clocks {
        let signal = design
            .signals
            .get(clock.signal.0)
            .ok_or_else(|| ir::Error(format!("invalid clock signal {}", clock.signal.0)))?;
        if signal.port != Some(ir::PortDirection::Input) || signal.ty.width != 1 {
            return Err(ir::Error(
                "physical clock must be a width-1 input port".into(),
            ));
        }
        if !constrained_clocks.insert(clock.signal) {
            return Err(ir::Error("duplicate clock constraint".into()));
        }
    }
    for domain in &design.domains {
        let clock = domain.clock.ok_or_else(|| {
            ir::Error(format!(
                "clock domain {} has no physical clock binding",
                domain.name
            ))
        })?;
        if !constrained_clocks.contains(&clock) {
            return Err(ir::Error(format!(
                "clock domain {} has no timing constraint",
                domain.name
            )));
        }
    }
    Ok(())
}

fn constraints(
    pins: &[PinBinding],
    clocks: &[ClockBinding],
    port_names: &HashMap<ir::SignalId, String>,
) -> Result<String, ir::Error> {
    let port = |signal, bit| -> Result<String, ir::Error> {
        let name = port_names
            .get(&signal)
            .ok_or_else(|| ir::Error("physical binding has no lowered HDL port".into()))?;
        let name = name.strip_prefix('\\').map_or(name.as_str(), str::trim_end);
        let mut pattern = String::from("^");
        for ch in name.chars() {
            if "\\.[]()|^$*+?{}".contains(ch) {
                pattern.push('\\');
            }
            pattern.push(ch);
        }
        pattern.push_str(&format!("\\[{bit}\\]$"));
        vivado::quote(&pattern)
    };
    let mut out = String::new();
    for pin in pins {
        let target = port(pin.signal, pin.bit)?;
        out.push_str(&format!(
            "set_property PACKAGE_PIN {} [get_ports -regexp {target}]\nset_property IOSTANDARD {} [get_ports -regexp {target}]\n",
            vivado::quote(&pin.package_pin)?, vivado::quote(&pin.io_standard)?,
        ));
    }
    for clock in clocks {
        let fs = clock.period.femtos();
        let fraction = fs % 1_000_000;
        let ns = if fraction == 0 {
            (fs / 1_000_000).to_string()
        } else {
            format!("{}.{fraction:06}", fs / 1_000_000)
                .trim_end_matches('0')
                .to_owned()
        };
        out.push_str(&format!(
            "create_clock -period {ns} [get_ports -regexp {}]\n",
            port(clock.signal, 0)?
        ));
    }
    Ok(out)
}
