use crate::{
    elaborate,
    ir::{
        BinaryOp, CompiledDesign, Design, DomainId, Error, PackedBits, PortDirection, SignalId,
        UnaryOp, ValueKind, WriteDomain,
    },
    module::Module,
};
use std::collections::HashSet;

pub struct Simulator {
    compiled: CompiledDesign,
    state: Vec<PackedBits>,
    value_order: Vec<usize>,
}

impl Simulator {
    pub fn new<Dut: Module>(make_dut: impl FnOnce() -> Dut) -> Result<(Self, Dut), Error> {
        let (design, dut) = elaborate(make_dut)?;
        let sim = Self::from_design(design)?;
        Ok((sim, dut))
    }

    pub fn from_design(design: Design) -> Result<Self, Error> {
        let compiled = design.compile()?;
        let state = compiled
            .design
            .signals
            .iter()
            .map(|s| s.init.clone())
            .collect();
        let value_order = compiled.design.value_order()?;
        let mut sim = Self {
            compiled,
            state,
            value_order,
        };
        sim.settle()?;
        Ok(sim)
    }
    pub fn design(&self) -> &Design {
        &self.compiled.design
    }
    pub fn get_signal(&self, id: SignalId) -> Result<&PackedBits, Error> {
        self.state
            .get(id.0)
            .ok_or_else(|| Error(format!("invalid signal ID {}", id.0)))
    }
    pub fn set_signal(&mut self, id: SignalId, value: PackedBits) -> Result<(), Error> {
        let current = self.get_signal(id)?;
        if current.width() != value.width() {
            return Err(Error(format!("signal {}: width mismatch", id.0)));
        }
        if self.compiled.drivers.iter().any(|d| d.signal == id) {
            return Err(Error(format!("signal {} is driven", id.0)));
        }
        if self.compiled.design.signals[id.0].port != Some(PortDirection::Input) {
            return Err(Error(format!("signal {} is not an input", id.0)));
        }
        self.state[id.0] = value;
        self.settle()
    }
    pub fn settle(&mut self) -> Result<(), Error> {
        // Re-evaluate against the current bit frontier. A vector cache cannot
        // survive a bit commit: its other bits may have read not-yet-settled state.
        for &(signal, bit) in &self.compiled.comb_order {
            let driver = self
                .compiled
                .drivers
                .iter()
                .find(|d| d.signal == signal && d.domain == WriteDomain::Comb)
                .ok_or_else(|| Error("missing combinational driver".into()))?;
            let values = evaluate(&self.compiled.design, &self.value_order, &self.state);
            self.state[signal.0].set_bit(bit, values[driver.value.0].bit(bit));
        }
        Ok(())
    }
    pub fn step(&mut self, domains: &[DomainId]) -> Result<(), Error> {
        for d in domains {
            if !self
                .compiled
                .design
                .domains
                .iter()
                .any(|domain| domain.id == *d)
            {
                return Err(Error(format!("invalid clock domain {d:?}")));
            }
        }
        self.settle()?;
        let triggered: HashSet<_> = domains.iter().copied().collect();
        let reset_asserted: HashSet<_> = self
            .compiled
            .design
            .domains
            .iter()
            .filter(|domain| domain.reset.is_some_and(|reset| self.state[reset.0].bit(0)))
            .map(|domain| domain.id)
            .collect();
        let values = evaluate(&self.compiled.design, &self.value_order, &self.state);
        for driver in &self.compiled.drivers {
            if let WriteDomain::Clocked(domain) = driver.domain
                && triggered.contains(&domain)
            {
                self.state[driver.signal.0] = if reset_asserted.contains(&domain) {
                    self.compiled.design.signals[driver.signal.0].init.clone()
                } else {
                    values[driver.value.0].clone()
                };
            }
        }
        self.settle()
    }
    pub fn tick(&mut self, domain: DomainId) -> Result<(), Error> {
        self.step(&[domain])
    }
}

fn evaluate(design: &Design, order: &[usize], state: &[PackedBits]) -> Vec<PackedBits> {
    let mut values: Vec<PackedBits> = design
        .values
        .iter()
        .map(|v| PackedBits::zero(v.ty.width))
        .collect();
    for &i in order {
        let v = &design.values[i];
        let result = match &v.kind {
            ValueKind::Constant(b) => b.clone(),
            ValueKind::Read(s) => state[s.0].clone(),
            ValueKind::Unary { op, value } => match op {
                UnaryOp::Not => values[value.0].not(),
                UnaryOp::Neg => values[value.0].neg(),
            },
            ValueKind::Binary { op, lhs, rhs } => {
                let a = &values[lhs.0];
                let b = &values[rhs.0];
                match op {
                    BinaryOp::Add => a.add(b),
                    BinaryOp::Sub => a.add(&b.neg()),
                    BinaryOp::Mul => a.mul(b),
                    BinaryOp::And => a.bitop(b, |a, b| a & b),
                    BinaryOp::Or => a.bitop(b, |a, b| a | b),
                    BinaryOp::Xor => a.bitop(b, |a, b| a ^ b),
                    BinaryOp::Shl => a.shift(b, true, false),
                    BinaryOp::Shr => a.shift(b, false, false),
                    BinaryOp::ArithmeticShr => a.shift(b, false, true),
                    _ => {
                        let cmp = a.cmp(b, design.values[lhs.0].ty.signed);
                        let yes = match op {
                            BinaryOp::Eq => cmp.is_eq(),
                            BinaryOp::Ne => !cmp.is_eq(),
                            BinaryOp::Lt => cmp.is_lt(),
                            BinaryOp::Le => !cmp.is_gt(),
                            BinaryOp::Gt => cmp.is_gt(),
                            BinaryOp::Ge => !cmp.is_lt(),
                            _ => unreachable!(),
                        };
                        PackedBits::from_u64(1, yes as u64)
                    }
                }
            }
            ValueKind::Mux {
                condition,
                then_value,
                else_value,
            } => values[if values[condition.0].bit(0) {
                then_value.0
            } else {
                else_value.0
            }]
            .clone(),
            ValueKind::Slice { value, offset } => values[value.0].slice(*offset, v.ty.width),
            ValueKind::Extend { value, sign } => values[value.0].extend(v.ty.width, *sign),
            ValueKind::Insert {
                base,
                value,
                offset,
            } => {
                let mut out = values[base.0].clone();
                out.replace(*offset, &values[value.0]);
                out
            }
        };
        values[i] = result;
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;

    struct DomainA;
    struct DomainB;
    struct DomainUnused;
    fn ty(width: u32) -> ValueType {
        ValueType {
            width,
            signed: false,
        }
    }
    fn signal(d: &mut Design, width: u32, init: u64) -> SignalId {
        d.add_signal(Signal {
            name: format!("s{}", d.signals.len()),
            ty: ty(width),
            init: PackedBits::from_u64(width, init),
            port: None,
            source: None,
        })
    }
    fn constant(d: &mut Design, width: u32, n: u64) -> ValueId {
        d.add_value(
            ty(width),
            ValueKind::Constant(PackedBits::from_u64(width, n)),
            None,
        )
    }
    fn read(d: &mut Design, s: SignalId) -> ValueId {
        d.add_value(d.signals[s.0].ty, ValueKind::Read(s), None)
    }
    fn write(
        d: &mut Design,
        s: SignalId,
        offset: u32,
        width: u32,
        v: ValueId,
        c: Option<ValueId>,
        domain: WriteDomain,
    ) {
        d.writes.push(Write {
            target: Target {
                signal: s,
                offset,
                width,
            },
            value: v,
            condition: c,
            domain,
            source: None,
        });
    }
    #[test]
    fn domain_identity_and_discovery_order() {
        let mut d = Design::new("domains");
        let b = d.add_domain::<DomainB>("first".into());
        let a = d.add_domain::<DomainA>("second".into());
        assert_ne!(a, b);
        assert_eq!(d.add_domain::<DomainB>("ignored".into()), b);
        assert_eq!(d.domains.len(), 2);
        assert_eq!(d.domains[0].id, b);
        assert_eq!(d.domains[0].name, "first");
        assert_eq!(d.domains[1].id, a);
        let (ast, _) = crate::verilog::to_ast(&d).unwrap();
        let verilog_gen::Ast::Module(module) = ast;
        assert_eq!(module.ports.len(), 2);
        assert_eq!(
            module.ports[0].name,
            verilog_gen::Ident::try_new("first").unwrap()
        );
        assert_eq!(
            module.ports[1].name,
            verilog_gen::Ident::try_new("second").unwrap()
        );
        let mut other = Design::new("other");
        assert_eq!(other.add_domain::<DomainA>("different".into()), a);
        let mut sim = Simulator::from_design(other).unwrap();
        sim.tick(a).unwrap();
    }

    #[test]
    fn missing_and_duplicate_domains_are_rejected() {
        let mut d = Design::new("invalid_domains");
        d.add_domain::<DomainA>("clk".into());
        let a = signal(&mut d, 8, 0);
        let value = constant(&mut d, 8, 1);
        write(
            &mut d,
            a,
            0,
            8,
            value,
            None,
            WriteDomain::Clocked(DomainId::of::<DomainUnused>()),
        );
        assert!(d.compile().unwrap_err().0.contains("invalid domain"));
        assert!(crate::verilog::lower(&d).is_err());
        d.writes.clear();
        d.domains.push(d.domains[0].clone());
        assert!(
            d.compile()
                .unwrap_err()
                .0
                .contains("duplicate clock domain")
        );
        assert!(crate::verilog::lower(&d).is_err());
        assert!(Simulator::from_design(d).is_err());
    }

    #[test]
    fn simultaneous_domains_and_swap() {
        let mut d = Design::new("swap");
        let a = signal(&mut d, 8, 3);
        let b = signal(&mut d, 8, 7);
        let x = d.add_domain::<DomainA>("x".into());
        let y = d.add_domain::<DomainB>("y".into());
        let av = read(&mut d, a);
        let bv = read(&mut d, b);
        write(&mut d, a, 0, 8, bv, None, WriteDomain::Clocked(x));
        write(&mut d, b, 0, 8, av, None, WriteDomain::Clocked(y));
        let mut s = Simulator::from_design(d.clone()).unwrap();
        s.step(&[x, y, x]).unwrap();
        assert_eq!(s.get_signal(a).unwrap().words(), &[7]);
        assert_eq!(s.get_signal(b).unwrap().words(), &[3]);
        let mut s = Simulator::from_design(d).unwrap();
        s.tick(x).unwrap();
        assert_eq!(s.get_signal(b).unwrap().words(), &[7]);
        s.tick(y).unwrap();
        assert_eq!(s.get_signal(b).unwrap().words(), &[7]);
        assert!(s.tick(DomainId::of::<DomainUnused>()).is_err());
        assert!(s.get_signal(SignalId(99)).is_err());
    }
    #[test]
    fn bit_feedforward_and_input_settle() {
        let mut d = Design::new("bits");
        let input = signal(&mut d, 4, 2);
        d.signals[input.0].port = Some(PortDirection::Input);
        let a = signal(&mut d, 8, 0);
        let iv = read(&mut d, input);
        let av = read(&mut d, a);
        let low = d.add_value(
            ty(4),
            ValueKind::Slice {
                value: av,
                offset: 0,
            },
            None,
        );
        write(&mut d, a, 4, 4, low, None, WriteDomain::Comb);
        write(&mut d, a, 0, 4, iv, None, WriteDomain::Comb);
        let mut s = Simulator::from_design(d).unwrap();
        assert_eq!(s.get_signal(a).unwrap().words(), &[0x22]);
        s.set_signal(input, PackedBits::from_u64(4, 9)).unwrap();
        assert_eq!(s.get_signal(a).unwrap().words(), &[0x99]);
        assert!(s.set_signal(a, PackedBits::zero(8)).is_err());
        assert!(s.set_signal(input, PackedBits::zero(3)).is_err());
    }
    #[test]
    fn cycles_domains_and_overwrites() {
        let mut d = Design::new("cycle");
        let a = signal(&mut d, 8, 0);
        let av = read(&mut d, a);
        write(&mut d, a, 0, 8, av, None, WriteDomain::Comb);
        assert!(d.compile().unwrap_err().0.contains("cycle"));
        let c = constant(&mut d, 8, 42);
        write(&mut d, a, 0, 8, c, None, WriteDomain::Comb);
        assert!(d.compile().is_ok());
        let clock = d.add_domain::<DomainA>("clock".into());
        write(&mut d, a, 0, 8, c, None, WriteDomain::Clocked(clock));
        assert!(d.compile().is_err());
    }
    #[test]
    fn masked_priority_reset_enable() {
        let mut d = Design::new("priority");
        let clk = d.add_domain::<DomainA>("clk".into());
        let a = signal(&mut d, 8, 0x12);
        let en = signal(&mut d, 1, 0);
        let reset = signal(&mut d, 1, 0);
        d.signals[en.0].port = Some(PortDirection::Input);
        d.signals[reset.0].port = Some(PortDirection::Input);
        let ev = read(&mut d, en);
        let rv = read(&mut d, reset);
        let lo = constant(&mut d, 4, 0xa);
        let hi = constant(&mut d, 4, 0xb);
        let zero = constant(&mut d, 8, 0);
        write(&mut d, a, 0, 4, lo, Some(ev), WriteDomain::Clocked(clk));
        write(&mut d, a, 4, 4, hi, Some(ev), WriteDomain::Clocked(clk));
        write(&mut d, a, 0, 8, zero, Some(rv), WriteDomain::Clocked(clk));
        let mut s = Simulator::from_design(d).unwrap();
        s.tick(clk).unwrap();
        assert_eq!(s.get_signal(a).unwrap().words(), &[0x12]);
        s.set_signal(en, PackedBits::from_u64(1, 1)).unwrap();
        s.tick(clk).unwrap();
        assert_eq!(s.get_signal(a).unwrap().words(), &[0xba]);
        s.set_signal(reset, PackedBits::from_u64(1, 1)).unwrap();
        s.tick(clk).unwrap();
        assert_eq!(s.get_signal(a).unwrap().words(), &[0]);
    }
    #[test]
    fn wide_arithmetic_and_signs() {
        let mut d = Design::new("wide");
        let t = ty(130);
        let a = d.add_value(
            t,
            ValueKind::Constant(PackedBits::from_words(130, vec![u64::MAX, 0, 0])),
            None,
        );
        let one = constant(&mut d, 130, 1);
        let add = d.add_value(
            t,
            ValueKind::Binary {
                op: BinaryOp::Add,
                lhs: a,
                rhs: one,
            },
            None,
        );
        let sub = d.add_value(
            t,
            ValueKind::Binary {
                op: BinaryOp::Sub,
                lhs: one,
                rhs: add,
            },
            None,
        );
        let mul = d.add_value(
            t,
            ValueKind::Binary {
                op: BinaryOp::Mul,
                lhs: add,
                rhs: add,
            },
            None,
        );
        let cmp = d.add_value(
            ty(1),
            ValueKind::Binary {
                op: BinaryOp::Gt,
                lhs: add,
                rhs: a,
            },
            None,
        );
        let signed = ValueType {
            width: 65,
            signed: true,
        };
        let neg = d.add_value(
            signed,
            ValueKind::Constant(PackedBits::from_words(65, vec![u64::MAX, 1])),
            None,
        );
        let ext = d.add_value(
            ValueType {
                width: 130,
                signed: true,
            },
            ValueKind::Extend {
                value: neg,
                sign: true,
            },
            None,
        );
        let huge = d.add_value(
            t,
            ValueKind::Constant(PackedBits::from_words(130, vec![0, 1])),
            None,
        );
        let shr = d.add_value(
            t,
            ValueKind::Binary {
                op: BinaryOp::Shr,
                lhs: add,
                rhs: huge,
            },
            None,
        );
        let out = evaluate(&d, &d.value_order().unwrap(), &[]);
        assert_eq!(out[add.0].words(), &[0, 1, 0]);
        assert_eq!(out[sub.0].words(), &[1, u64::MAX, 3]);
        assert_eq!(out[mul.0].words(), &[0, 0, 1]);
        assert!(out[cmp.0].bit(0));
        assert_eq!(out[ext.0].words(), &[u64::MAX, u64::MAX, 3]);
        assert_eq!(out[shr.0], PackedBits::zero(130));
        assert!(out[neg.0].cmp(&PackedBits::zero(65), true).is_lt());
    }
    #[test]
    fn alu_matches_native_reference() {
        for width in [0, 1, 7, 63, 64, 65, 100, 127] {
            let mask = if width == 0 { 0 } else { (1u128 << width) - 1 };
            let mut seed = 0x123456789abcdef0u128;
            for _ in 0..12 {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let a = seed & mask;
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let b = seed & mask;
                let mut d = Design::new("reference");
                let av = d.add_value(
                    ty(width),
                    ValueKind::Constant(PackedBits::from_words(
                        width,
                        vec![a as u64, (a >> 64) as u64],
                    )),
                    None,
                );
                let bv = d.add_value(
                    ty(width),
                    ValueKind::Constant(PackedBits::from_words(
                        width,
                        vec![b as u64, (b >> 64) as u64],
                    )),
                    None,
                );
                let cases = [
                    (BinaryOp::Add, a.wrapping_add(b) & mask),
                    (BinaryOp::Sub, a.wrapping_sub(b) & mask),
                    (BinaryOp::Mul, a.wrapping_mul(b) & mask),
                    (BinaryOp::And, a & b),
                    (BinaryOp::Or, a | b),
                    (BinaryOp::Xor, a ^ b),
                ];
                let mut checks = vec![];
                for (op, expected) in cases {
                    checks.push((
                        d.add_value(
                            ty(width),
                            ValueKind::Binary {
                                op,
                                lhs: av,
                                rhs: bv,
                            },
                            None,
                        ),
                        expected,
                    ));
                }
                for (op, expected) in [
                    (BinaryOp::Eq, a == b),
                    (BinaryOp::Ne, a != b),
                    (BinaryOp::Lt, a < b),
                    (BinaryOp::Le, a <= b),
                    (BinaryOp::Gt, a > b),
                    (BinaryOp::Ge, a >= b),
                ] {
                    checks.push((
                        d.add_value(
                            ty(1),
                            ValueKind::Binary {
                                op,
                                lhs: av,
                                rhs: bv,
                            },
                            None,
                        ),
                        expected as u128,
                    ));
                }
                for amount in [0, 1, 63, 64, 126, 127, 128, 200] {
                    let count = constant(&mut d, 32, amount);
                    for (op, expected) in [
                        (
                            BinaryOp::Shl,
                            a.checked_shl(amount as u32).unwrap_or(0) & mask,
                        ),
                        (BinaryOp::Shr, a.checked_shr(amount as u32).unwrap_or(0)),
                    ] {
                        checks.push((
                            d.add_value(
                                ty(width),
                                ValueKind::Binary {
                                    op,
                                    lhs: av,
                                    rhs: count,
                                },
                                None,
                            ),
                            expected,
                        ));
                    }
                }
                let neg = d.add_value(
                    ty(width),
                    ValueKind::Unary {
                        op: UnaryOp::Neg,
                        value: av,
                    },
                    None,
                );
                checks.push((neg, a.wrapping_neg() & mask));
                let not = d.add_value(
                    ty(width),
                    ValueKind::Unary {
                        op: UnaryOp::Not,
                        value: av,
                    },
                    None,
                );
                checks.push((not, !a & mask));
                d.compile().unwrap();
                let out = evaluate(&d, &d.value_order().unwrap(), &[]);
                for (id, expected) in checks {
                    let words = out[id.0].words();
                    let actual = words.first().copied().unwrap_or(0) as u128
                        | ((words.get(1).copied().unwrap_or(0) as u128) << 64);
                    assert_eq!(actual, expected, "width {width}, value {id:?}");
                }
            }
        }
    }
    #[test]
    fn setters_preserve_undriven_non_input_initial_values() {
        let mut d = Design::new("undriven");
        let internal = signal(&mut d, 8, 17);
        let output = signal(&mut d, 8, 29);
        d.signals[output.0].port = Some(PortDirection::Output);
        let input = signal(&mut d, 8, 3);
        d.signals[input.0].port = Some(PortDirection::Input);
        let mut sim = Simulator::from_design(d).unwrap();
        for id in [internal, output] {
            assert!(
                sim.set_signal(id, PackedBits::zero(8))
                    .unwrap_err()
                    .0
                    .contains("not an input")
            );
            assert!(
                sim.set_signal(id, PackedBits::zero(7))
                    .unwrap_err()
                    .0
                    .contains("width mismatch")
            );
        }
        assert!(sim.set_signal(SignalId(99), PackedBits::zero(8)).is_err());
        sim.set_signal(input, PackedBits::from_u64(8, 42)).unwrap();
        sim.step(&[]).unwrap();
        assert_eq!(sim.get_signal(internal).unwrap().words(), &[17]);
        assert_eq!(sim.get_signal(output).unwrap().words(), &[29]);
        assert_eq!(sim.get_signal(input).unwrap().words(), &[42]);
    }
    #[test]
    fn deep_expression_compile_is_stack_safe() {
        let mut d = Design::new("deep");
        let input = signal(&mut d, 1, 0);
        d.signals[input.0].port = Some(PortDirection::Input);
        let output = signal(&mut d, 1, 0);
        let mut value = read(&mut d, input);
        for _ in 0..20_000 {
            value = d.add_value(
                ty(1),
                ValueKind::Unary {
                    op: UnaryOp::Not,
                    value,
                },
                None,
            );
        }
        write(&mut d, output, 0, 1, value, None, WriteDomain::Comb);
        assert_eq!(d.compile().unwrap().comb_order, vec![(output, 0)]);
    }
    #[test]
    fn constant_shift_dependencies_are_bit_exact() {
        for op in [BinaryOp::Shl, BinaryOp::Shr, BinaryOp::ArithmeticShr] {
            let mut d = Design::new("shift feedforward");
            let a = signal(&mut d, 8, 0);
            let av = read(&mut d, a);
            let count = constant(&mut d, 8, 1);
            let shifted = d.add_value(
                ty(8),
                ValueKind::Binary {
                    op,
                    lhs: av,
                    rhs: count,
                },
                None,
            );
            let (offset, seed_offset) = if op == BinaryOp::Shl { (1, 0) } else { (0, 7) };
            let field = d.add_value(
                ty(7),
                ValueKind::Slice {
                    value: shifted,
                    offset,
                },
                None,
            );
            let one = constant(&mut d, 1, 1);
            write(&mut d, a, offset, 7, field, None, WriteDomain::Comb);
            write(&mut d, a, seed_offset, 1, one, None, WriteDomain::Comb);
            let sim = Simulator::from_design(d).unwrap();
            assert_eq!(sim.get_signal(a).unwrap().words(), &[255]);
        }
        for op in [BinaryOp::Shl, BinaryOp::Shr, BinaryOp::ArithmeticShr] {
            let mut d = Design::new("huge shift");
            let a = signal(&mut d, 8, 0x80);
            let av = read(&mut d, a);
            let count = d.add_value(
                ty(130),
                ValueKind::Constant(PackedBits::from_words(130, vec![0, 0, 1])),
                None,
            );
            let shifted = d.add_value(
                ty(8),
                ValueKind::Binary {
                    op,
                    lhs: av,
                    rhs: count,
                },
                None,
            );
            write(&mut d, a, 0, 8, shifted, None, WriteDomain::Comb);
            if op == BinaryOp::ArithmeticShr {
                assert!(d.compile().unwrap_err().0.contains("cycle"));
                let one = constant(&mut d, 1, 1);
                write(&mut d, a, 7, 1, one, None, WriteDomain::Comb);
            }
            let sim = Simulator::from_design(d).unwrap();
            assert_eq!(
                sim.get_signal(a).unwrap().words(),
                &[if op == BinaryOp::ArithmeticShr {
                    255
                } else {
                    0
                }]
            );
        }
    }
    #[test]
    fn validation_and_zero_width() {
        let mut d = Design::new("empty");
        let a = signal(&mut d, 0, 0);
        let z = constant(&mut d, 0, 0);
        write(&mut d, a, 0, 0, z, None, WriteDomain::Comb);
        assert!(Simulator::from_design(d.clone()).is_ok());
        d.values.push(ValueData {
            ty: ty(1),
            kind: ValueKind::Read(SignalId(99)),
            source: None,
        });
        assert!(d.compile().is_err());
        let mut d = Design::new("bad");
        d.add_value(
            ty(1),
            ValueKind::Unary {
                op: UnaryOp::Not,
                value: ValueId(99),
            },
            None,
        );
        assert!(d.compile().is_err());
    }
}
