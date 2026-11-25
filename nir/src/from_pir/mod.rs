use std::{collections::HashMap, str::FromStr};

use irt::bumpalo::Bump;

use crate as nir;

mod cycles;
mod errors;
mod expr;

pub use errors::{DriverDomain, LowerError, NetLocation, PatternPart, SignalAccess};

#[must_use]
pub fn from_pir<'d>(
    pir: pir::Module<'d>,
    bump: &'d Bump,
) -> (Option<nir::Netlist<'d>>, Vec<LowerError>) {
    let mut lowerer = Lowerer {
        bump,
        modules: Vec::new(),
        signals: HashMap::new(),
        signal_order: Vec::new(),
        drivers: Vec::new(),
        clocks: HashMap::new(),
        cells: Vec::new(),
        errors: Vec::new(),
    };
    lowerer.collect_module(pir, None);
    lowerer.inputs();
    for module in 0..lowerer.modules.len() {
        for domain in lowerer.modules[module].source.uses_domains {
            let clock = match domain.domain {
                pir::Domain::Comb => None,
                pir::Domain::Clocked(clock) => Some(clock),
            };
            lowerer.statements(module, clock, domain.stmts, nir::Net::ONE);
        }
    }
    lowerer.finish()
}

struct ModuleBuilder<'d> {
    source: pir::Module<'d>,
    parent: Option<usize>,
    children: Vec<usize>,
    cells: Vec<nir::CellRef<'d>>,
    clock_ports: Vec<nir::Port<'d>>,
}

struct SignalInfo<'d> {
    module: usize,
    input: bool,
    value: nir::Value<'d>,
    driven: Vec<Option<usize>>,
    resolved: Vec<Option<nir::Net<'d>>>,
}

struct Driver<'d> {
    signal: pir::Signal<'d>,
    module: usize,
    domain: Option<pir::ClockedDomain<'d>>,
    assigns: Vec<nir::Assign<'d>>,
}

struct Lowerer<'d> {
    bump: &'d Bump,
    modules: Vec<ModuleBuilder<'d>>,
    signals: HashMap<pir::Signal<'d>, SignalInfo<'d>>,
    signal_order: Vec<pir::Signal<'d>>,
    drivers: Vec<Driver<'d>>,
    clocks: HashMap<pir::ClockedDomain<'d>, nir::Value<'d>>,
    cells: Vec<(nir::CellRef<'d>, u32)>,
    errors: Vec<LowerError>,
}

const fn width(shape: pir::Shape) -> u32 {
    match shape {
        pir::Shape::Bits(width) | pir::Shape::Unsigned(width) | pir::Shape::Signed(width) => width,
    }
}

const fn driver_domain(domain: Option<pir::ClockedDomain<'_>>) -> DriverDomain {
    match domain {
        None => DriverDomain::Comb,
        Some(domain) => DriverDomain::Clocked {
            name: domain.0.name,
        },
    }
}

impl<'d> Lowerer<'d> {
    fn collect_module(&mut self, source: pir::Module<'d>, parent: Option<usize>) -> usize {
        let module = self.modules.len();
        self.modules.push(ModuleBuilder {
            source,
            parent,
            children: Vec::new(),
            cells: Vec::new(),
            clock_ports: Vec::new(),
        });
        for (index, port) in source.ports.iter().enumerate() {
            if source.ports[..index]
                .iter()
                .any(|other| other.name == port.name)
            {
                self.errors.push(LowerError::DuplicatePortName {
                    module: source.name,
                    port: port.name,
                });
            }
            if let Some(other) = source.ports[..index]
                .iter()
                .find(|other| other.signal == port.signal)
            {
                self.errors.push(LowerError::DuplicatePortSignal {
                    module: source.name,
                    signal: port.signal.0.name,
                    first_port: other.name,
                    second_port: port.name,
                });
            }
            if let Err(error) = self.declare(module, port.signal, port.flow == pir::Flow::In) {
                self.errors.push(error);
            }
        }
        for signal in source.signals {
            if let Err(error) = self.declare(module, *signal, false) {
                self.errors.push(error);
            }
        }
        for child in source.children {
            if self.modules[module]
                .children
                .iter()
                .any(|&index| self.modules[index].source.name == child.name)
            {
                self.errors.push(LowerError::DuplicateChildModule {
                    module: source.name,
                    child: child.name,
                });
            }
            let child = self.collect_module(*child, Some(module));
            self.modules[module].children.push(child);
        }
        module
    }

    fn declare(
        &mut self,
        module: usize,
        signal: pir::Signal<'d>,
        input: bool,
    ) -> Result<(), LowerError> {
        if let Some(info) = self.signals.get(&signal) {
            if info.module != module {
                return Err(LowerError::DuplicateSignalDeclaration {
                    signal: signal.0.name,
                    first_module: self.modules[info.module].source.name,
                    second_module: self.modules[module].source.name,
                });
            }
            return Ok(());
        }
        let width = width(signal.0.ty.0.shape);
        if signal.0.init.width() != width {
            self.errors.push(LowerError::InitialWidthMismatch {
                signal: signal.0.name,
                expected: width,
                actual: signal.0.init.width(),
            });
        }
        let nets = self.bump.alloc_slice_fill_iter((0..width).map(|_| {
            nir::Net::Late(nir::LateNet(self.bump.alloc(nir::LateNetDef {
                name: signal.0.name,
            })))
        }));
        self.signals.insert(
            signal,
            SignalInfo {
                module,
                input,
                value: nir::Value(nets),
                driven: vec![None; width as usize],
                resolved: vec![None; width as usize],
            },
        );
        self.signal_order.push(signal);
        Ok(())
    }

    fn inputs(&mut self) {
        for port in self.modules[0].source.ports {
            if port.flow == pir::Flow::In {
                let value = self.emit(
                    0,
                    nir::Input {
                        width: width(port.signal.0.ty.0.shape),
                    }
                    .into(),
                    width(port.signal.0.ty.0.shape),
                );
                let info = self.signals.get_mut(&port.signal).unwrap();
                for (target, net) in info.resolved.iter_mut().zip(value.0) {
                    *target = Some(*net);
                }
            }
        }
    }

    fn emit(&mut self, module: usize, cell: nir::Cell<'d>, width: u32) -> nir::Value<'d> {
        let cell = nir::CellRef(self.bump.alloc(cell));
        self.modules[module].cells.push(cell);
        self.cells.push((cell, width));
        nir::Value(
            self.bump
                .alloc_slice_fill_iter((0..width).map(|bit| nir::Net::Cell { cell, bit })),
        )
    }

    fn resize(&self, value: nir::Value<'d>, width: u32, signed: bool) -> nir::Value<'d> {
        if value.0.len() == width as usize {
            return value;
        }
        let fill = if signed {
            value.0.last().copied().unwrap_or(nir::Net::ZERO)
        } else {
            nir::Net::ZERO
        };
        nir::Value(self.bump.alloc_slice_fill_iter(
            (0..width as usize).map(|bit| value.0.get(bit).copied().unwrap_or(fill)),
        ))
    }

    fn constant(&self, bits: &nir::Bits) -> nir::Value<'d> {
        nir::Value(
            self.bump
                .alloc_slice_fill_iter((0..bits.width()).map(|bit| {
                    let word = bits.words().get(bit as usize / 64).copied().unwrap_or(0);
                    nir::Net::Const(word & (1_u64 << (bit % 64)) != 0)
                })),
        )
    }

    fn signal_value(
        &self,
        module: usize,
        signal: pir::Signal<'d>,
    ) -> Result<nir::Value<'d>, LowerError> {
        let info = self
            .signals
            .get(&signal)
            .ok_or(LowerError::UndeclaredSignal {
                module: self.modules[module].source.name,
                signal: signal.0.name,
                access: SignalAccess::Read,
            })?;
        let child_port = self.modules[info.module].parent == Some(module)
            && self.modules[info.module]
                .source
                .ports
                .iter()
                .any(|port| port.signal == signal);
        if info.module != module && !child_port {
            return Err(LowerError::SignalAccessViolation {
                module: self.modules[module].source.name,
                signal: signal.0.name,
                declared_in: self.modules[info.module].source.name,
                access: SignalAccess::Read,
            });
        }
        Ok(info.value)
    }

    fn bit_value(&self, bit: nir::Net<'d>) -> nir::Value<'d> {
        nir::Value(self.bump.alloc_slice_copy(&[bit]))
    }

    fn and(&mut self, module: usize, lhs: nir::Net<'d>, rhs: nir::Net<'d>) -> nir::Net<'d> {
        match (lhs, rhs) {
            (nir::Net::Const(false), _) | (_, nir::Net::Const(false)) => nir::Net::ZERO,
            (nir::Net::Const(true), rhs) => rhs,
            (lhs, nir::Net::Const(true)) => lhs,
            _ if lhs == rhs => lhs,
            _ => {
                let lhs = self.bit_value(lhs);
                let rhs = self.bit_value(rhs);
                self.emit(
                    module,
                    nir::Binary {
                        op: nir::BinaryOp::And,
                        lhs,
                        rhs,
                    }
                    .into(),
                    1,
                )
                .0[0]
            }
        }
    }

    fn not(&mut self, module: usize, value: nir::Net<'d>) -> nir::Net<'d> {
        if let nir::Net::Const(value) = value {
            return nir::Net::Const(!value);
        }
        let value = self.bit_value(value);
        self.emit(
            module,
            nir::Unary {
                op: nir::UnaryOp::Not,
                value,
            }
            .into(),
            1,
        )
        .0[0]
    }

    fn statements(
        &mut self,
        module: usize,
        domain: Option<pir::ClockedDomain<'d>>,
        statements: &'d [pir::Stmt<'d>],
        condition: nir::Net<'d>,
    ) {
        for statement in statements {
            match statement {
                pir::Stmt::Assign(assign) => {
                    if let Err(error) = self.assign(module, domain, *assign, condition) {
                        self.errors.push(error);
                    }
                }
                pir::Stmt::Switch(switch) => self.switch(module, domain, *switch, condition),
            }
        }
    }

    fn switch(
        &mut self,
        module: usize,
        domain: Option<pir::ClockedDomain<'d>>,
        switch: pir::Switch<'d>,
        condition: nir::Net<'d>,
    ) {
        let expected = width(switch.on.ty.0.shape);
        let value = match self.value(module, switch.on) {
            Ok(value) => Some(value),
            Err(error) => {
                self.errors.push(error);
                None
            }
        };
        let mut remaining = condition;
        for arm in switch.arms {
            let mut valid = value.is_some();
            for pattern in arm.patterns {
                let (bits, mask) = match pattern {
                    nir::Pattern::Any => continue,
                    nir::Pattern::Exact(bits) => (*bits, None),
                    nir::Pattern::Masked { value, mask } => (*value, Some(*mask)),
                };
                if bits.width() != expected {
                    self.errors.push(LowerError::PatternWidthMismatch {
                        part: PatternPart::Value,
                        expected,
                        actual: bits.width(),
                    });
                    valid = false;
                }
                if let Some(mask) = mask
                    && mask.width() != expected
                {
                    self.errors.push(LowerError::PatternWidthMismatch {
                        part: PatternPart::Mask,
                        expected,
                        actual: mask.width(),
                    });
                    valid = false;
                }
            }
            let guard = if let Some(guard) = arm.guard {
                if width(guard.ty.0.shape) != 1 {
                    self.errors.push(LowerError::GuardWidthMismatch {
                        actual: width(guard.ty.0.shape),
                    });
                    valid = false;
                }
                match self.value(module, guard) {
                    Ok(guard) => guard.0.first().copied().unwrap_or(nir::Net::ZERO),
                    Err(error) => {
                        self.errors.push(error);
                        valid = false;
                        nir::Net::ZERO
                    }
                }
            } else {
                nir::Net::ONE
            };
            let unconditional = arm.patterns.iter().any(|pattern| match pattern {
                nir::Pattern::Any => true,
                nir::Pattern::Masked { mask, .. } => mask.words().iter().all(|word| *word == 0),
                nir::Pattern::Exact(_) => false,
            });
            let matched = if !valid || arm.patterns.is_empty() || remaining == nir::Net::ZERO {
                nir::Net::ZERO
            } else if unconditional {
                remaining
            } else if let Some(value) = value {
                let arms = self.bump.alloc_slice_copy(&[nir::MatchArm {
                    patterns: arm.patterns,
                }]);
                self.emit(
                    module,
                    nir::Match {
                        enable: remaining,
                        value,
                        arms,
                    }
                    .into(),
                    1,
                )
                .0[0]
            } else {
                nir::Net::ZERO
            };
            let active = self.and(module, matched, guard);
            self.statements(module, domain, arm.then, active);
            let inactive = self.not(module, active);
            remaining = self.and(module, remaining, inactive);
        }
    }

    fn assign(
        &mut self,
        module: usize,
        domain: Option<pir::ClockedDomain<'d>>,
        assign: pir::Assign<'d>,
        condition: nir::Net<'d>,
    ) -> Result<(), LowerError> {
        let value = match self.value(module, assign.from) {
            Ok(value) => Some(value),
            Err(error) => {
                self.errors.push(error);
                None
            }
        };
        let (signal, offset, size, whole) = match assign.into {
            pir::Target::Signal(signal) => (signal, 0, width(signal.0.ty.0.shape), true),
            pir::Target::Slice(slice) => (slice.into, slice.offset, slice.width, false),
        };
        let info = self
            .signals
            .get(&signal)
            .ok_or(LowerError::UndeclaredSignal {
                module: self.modules[module].source.name,
                signal: signal.0.name,
                access: SignalAccess::Drive,
            })?;
        let allowed = if info.module == module {
            !info.input
        } else {
            self.modules[info.module].parent == Some(module) && info.input
        };
        if !allowed {
            return Err(LowerError::SignalAccessViolation {
                module: self.modules[module].source.name,
                signal: signal.0.name,
                declared_in: self.modules[info.module].source.name,
                access: SignalAccess::Drive,
            });
        }
        if offset > width(signal.0.ty.0.shape) || size > width(signal.0.ty.0.shape) - offset {
            return Err(LowerError::AssignmentOutOfBounds {
                signal: signal.0.name,
                offset,
                width: size,
                signal_width: width(signal.0.ty.0.shape),
            });
        }
        if width(assign.from.ty.0.shape) != size {
            return Err(LowerError::AssignmentWidthMismatch {
                signal: signal.0.name,
                expected: size,
                actual: width(assign.from.ty.0.shape),
            });
        }
        if whole && assign.from.ty != signal.0.ty {
            return Err(LowerError::AssignmentTypeMismatch {
                signal: signal.0.name,
                expected: signal.0.ty.0.name,
                actual: assign.from.ty.0.name,
            });
        }
        let Some(value) = value else {
            return Ok(());
        };
        if condition == nir::Net::ZERO || size == 0 {
            return Ok(());
        }
        let driver = if let Some(index) = self.drivers.iter().position(|driver| {
            driver.signal == signal && driver.module == module && driver.domain == domain
        }) {
            index
        } else {
            let index = self.drivers.len();

            self.drivers.push(Driver {
                signal,
                module,
                domain,
                assigns: Vec::new(),
            });
            index
        };
        let info = self.signals.get_mut(&signal).unwrap();
        for bit in offset..offset + size {
            if let Some(other) = info.driven[bit as usize].filter(|other| *other != driver) {
                let first = &self.drivers[other];
                return Err(LowerError::ConflictingDrivers {
                    signal: signal.0.name,
                    bit,
                    first_module: self.modules[first.module].source.name,
                    first_domain: driver_domain(first.domain),
                    second_module: self.modules[module].source.name,
                    second_domain: driver_domain(domain),
                });
            }
        }
        for bit in offset..offset + size {
            info.driven[bit as usize] = Some(driver);
        }
        self.drivers[driver].assigns.push(nir::Assign {
            condition,
            offset,
            value,
        });
        Ok(())
    }

    fn clock(&mut self, domain: pir::ClockedDomain<'d>, module: usize) -> nir::Net<'d> {
        let value = if let Some(value) = self.clocks.get(&domain) {
            *value
        } else {
            let value = self.emit(0, nir::Input { width: 1 }.into(), 1);
            self.clocks.insert(domain, value);
            value
        };
        let mut current = Some(module);
        while let Some(module) = current {
            if !self.modules[module]
                .clock_ports
                .iter()
                .any(|port| port.value == value)
            {
                let base = format!("{}_clk", domain.0.name);
                let mut name = nir::Ident::from_str(&base).unwrap();
                let mut suffix = 0;
                while self.modules[module]
                    .source
                    .ports
                    .iter()
                    .any(|port| port.name == name)
                    || self.modules[module]
                        .clock_ports
                        .iter()
                        .any(|port| port.name == name)
                {
                    suffix += 1;
                    name = nir::Ident::from_str(&format!("{base}_{suffix}")).unwrap();
                }
                self.modules[module].clock_ports.push(nir::Port {
                    name,
                    flow: nir::Flow::In,
                    value,
                });
            }
            current = self.modules[module].parent;
        }
        value.0[0]
    }

    fn finish(mut self) -> (Option<nir::Netlist<'d>>, Vec<LowerError>) {
        if !self.errors.is_empty() {
            return (None, self.errors);
        }
        for index in 0..self.drivers.len() {
            let driver = &self.drivers[index];
            let signal = driver.signal;
            let module = driver.module;
            let domain = driver.domain;
            let default = if domain.is_some() {
                self.signals[&signal].value
            } else {
                self.constant(signal.0.init)
            };
            let assigns = self.bump.alloc_slice_copy(&driver.assigns);
            let next = self.emit(
                module,
                nir::AssignList { default, assigns }.into(),
                width(signal.0.ty.0.shape),
            );
            let value = if let Some(domain) = domain {
                let clock = self.clock(domain, module);
                self.emit(
                    module,
                    nir::FlipFlop {
                        data: next,
                        init: signal.0.init,
                        clock,
                        edge: nir::Edge::Pos,
                        async_reset: nir::Net::ZERO,
                    }
                    .into(),
                    width(signal.0.ty.0.shape),
                )
            } else {
                next
            };
            let info = self.signals.get_mut(&signal).unwrap();
            for (bit, net) in value.0.iter().enumerate() {
                if info.driven[bit] == Some(index) {
                    info.resolved[bit] = Some(*net);
                }
            }
        }
        let mut connections = Vec::new();
        let mut resolved = HashMap::new();
        for signal in &self.signal_order {
            let info = &self.signals[signal];
            let initial = self.constant(signal.0.init);
            let nets: Vec<_> = info
                .resolved
                .iter()
                .enumerate()
                .map(|(bit, value)| value.unwrap_or(initial.0[bit]))
                .collect();
            for (late, net) in info.value.0.iter().zip(&nets) {
                let nir::Net::Late(from) = late else {
                    unreachable!()
                };
                connections.push(nir::Connection {
                    from: *from,
                    to: *net,
                });
            }
            resolved.insert(*signal, nir::Value(self.bump.alloc_slice_copy(&nets)));
        }
        self.errors
            .extend(cycles::check_cycles(&self.cells, &connections));
        if !self.errors.is_empty() {
            return (None, self.errors);
        }
        let mut modules = vec![None; self.modules.len()];
        let top = self.freeze_module(0, &resolved, &mut modules);
        let bindings: Vec<_> = self
            .signal_order
            .iter()
            .map(|signal| nir::SignalBinding {
                module: modules[self.signals[signal].module].unwrap(),
                signal: *signal,
                value: resolved[signal],
            })
            .collect();
        (
            Some(nir::Netlist {
                top,
                connections: self.bump.alloc_slice_copy(&connections),
                signals: self.bump.alloc_slice_copy(&bindings),
            }),
            self.errors,
        )
    }

    fn freeze_module(
        &self,
        index: usize,
        values: &HashMap<pir::Signal<'d>, nir::Value<'d>>,
        modules: &mut [Option<nir::Module<'d>>],
    ) -> nir::Module<'d> {
        let module = &self.modules[index];
        let children: Vec<_> = module
            .children
            .iter()
            .map(|&child| self.freeze_module(child, values, modules))
            .collect();
        let ports: Vec<_> = module
            .source
            .ports
            .iter()
            .map(|port| nir::Port {
                name: port.name,
                flow: port.flow,
                value: values[&port.signal],
            })
            .chain(module.clock_ports.iter().copied())
            .collect();
        let result = nir::Module {
            name: module.source.name,
            ports: self.bump.alloc_slice_copy(&ports),
            cells: self.bump.alloc_slice_copy(&module.cells),
            children: self.bump.alloc_slice_copy(&children),
        };
        modules[index] = Some(result);
        result
    }
}
