use crate::ir::{self, Error, ValueId, ValueKind, WriteDomain};
use std::collections::{HashMap, HashSet};
use verilog_gen as v;

pub fn lower(design: &ir::Design) -> Result<String, Error> {
    let (ast, origins) = to_ast(design)?;
    Ok(v::lower(&ast, &origins))
}

fn ident(name: &str) -> Result<v::Ident, Error> {
    v::Ident::try_new(name).map_err(|e| Error(format!("invalid Verilog identifier {name:?}: {e}")))
}

fn fresh(used: &mut HashSet<String>, base: &str) -> Result<v::Ident, Error> {
    let mut name = base.to_owned();
    let mut suffix = 0usize;
    while !used.insert(name.clone()) {
        suffix += 1;
        name = format!("{base}_{suffix}");
    }
    ident(&name)
}

fn origin(map: &mut v::OriginMap, source: Option<&ir::SourceLocation>) -> Result<v::Origin, Error> {
    let (file, line, column) = source.map_or(("<generated>", 1, 1), |s| {
        (s.file.as_str(), s.line, s.column)
    });
    if line == 0 || column == 0 {
        return Err(Error("Verilog source locations must be one-based".into()));
    }
    let point = v::Position { line, column };
    Ok(map.insert(v::Span {
        file: file.into(),
        start: point,
        end: point,
    }))
}

fn number(value: u64, width: Option<u16>, origin: v::Origin) -> v::Expr {
    v::Expr::Lit(v::Lit::Int(v::LitInt {
        origin,
        width: width.and_then(std::num::NonZero::new),
        radix: v::Radix::Hex,
        value,
    }))
}

fn concat(mut parts: Vec<v::Expr>) -> v::Expr {
    if parts.len() == 1 {
        parts.remove(0)
    } else {
        v::Expr::Concat(parts)
    }
}

fn constant(bits: &ir::PackedBits, origin: v::Origin) -> v::Expr {
    concat(
        bits.words()
            .iter()
            .enumerate()
            .rev()
            .map(|(i, &word)| {
                let width = (bits.width() - i as u32 * 64).min(64) as u16;
                number(word, Some(width), origin)
            })
            .collect(),
    )
}

fn ty(ty: ir::ValueType, origin: v::Origin) -> v::Ty {
    let hi = number(u64::from(ty.width - 1), None, origin);
    let lo = number(0, None, origin);
    if ty.signed {
        v::Ty::LogicSignedPacked(hi, lo)
    } else {
        v::Ty::LogicPacked(hi, lo)
    }
}

fn slice(name: v::Ident, offset: u32, width: u32, origin: v::Origin) -> v::Expr {
    v::Expr::Slice {
        value: Box::new(v::Expr::Name(name)),
        msb: Box::new(number(u64::from(offset + width - 1), None, origin)),
        lsb: Box::new(number(u64::from(offset), None, origin)),
    }
}

struct Values<'a> {
    design: &'a ir::Design,
    names: &'a [Option<v::Ident>],
    signals: &'a [v::Ident],
    signed_names: &'a [Option<v::Ident>],
}
impl Values<'_> {
    fn name(&self, id: ValueId) -> Result<v::Ident, Error> {
        self.names[id.0].ok_or_else(|| {
            Error(format!(
                "value {} has no nonzero-width Verilog representation",
                id.0
            ))
        })
    }
    fn get(&self, id: ValueId) -> Result<v::Expr, Error> {
        Ok(v::Expr::Name(self.name(id)?))
    }
    fn expression(&self, value: &ir::Value, o: v::Origin) -> Result<v::Expr, Error> {
        use ir::BinaryOp as B;
        Ok(match &value.kind {
            ValueKind::Constant(bits) => constant(bits, o),
            ValueKind::Read(s) => v::Expr::Name(self.signals[s.0]),
            ValueKind::Unary { op, value } => v::Expr::Unary {
                op: match op {
                    ir::UnaryOp::Not => v::UnaryOp::BitNot,
                    ir::UnaryOp::Neg => v::UnaryOp::Neg,
                },
                value: Box::new(self.get(*value)?),
            },
            ValueKind::Binary { op, lhs, rhs } => {
                if self.design.values[lhs.0].ty.width == 0 {
                    match op {
                        B::Eq | B::Le | B::Ge => number(1, Some(1), o),
                        B::Ne | B::Lt | B::Gt => number(0, Some(1), o),
                        _ => {
                            return Err(Error(
                                "unsupported nonzero result from zero-width binary operand".into(),
                            ));
                        }
                    }
                } else if self.design.values[rhs.0].ty.width == 0 {
                    match op {
                        B::Shl | B::Shr | B::ArithmeticShr => self.get(*lhs)?,
                        _ => return Err(Error("unsupported zero-width binary operand".into())),
                    }
                } else {
                    let op = match op {
                        B::Add => v::BinaryOp::Add,
                        B::Sub => v::BinaryOp::Sub,
                        B::Mul => v::BinaryOp::Mul,
                        B::Eq => v::BinaryOp::Eq,
                        B::Ne => v::BinaryOp::Ne,
                        B::Lt => v::BinaryOp::Lt,
                        B::Le => v::BinaryOp::Le,
                        B::Gt => v::BinaryOp::Gt,
                        B::Ge => v::BinaryOp::Ge,
                        B::And => v::BinaryOp::BitAnd,
                        B::Or => v::BinaryOp::BitOr,
                        B::Xor => v::BinaryOp::BitXor,
                        B::Shl => v::BinaryOp::Shl,
                        B::Shr => v::BinaryOp::Shr,
                        B::ArithmeticShr => v::BinaryOp::ArithmeticShr,
                    };
                    v::Expr::Binary {
                        lhs: Box::new(if matches!(op, v::BinaryOp::ArithmeticShr) {
                            self.signed_names[lhs.0]
                                .map(v::Expr::Name)
                                .unwrap_or(self.get(*lhs)?)
                        } else {
                            self.get(*lhs)?
                        }),
                        op,
                        rhs: Box::new(self.get(*rhs)?),
                    }
                }
            }
            ValueKind::Mux {
                condition,
                then_value,
                else_value,
            } => v::Expr::Ternary {
                condition: Box::new(self.get(*condition)?),
                then_value: Box::new(self.get(*then_value)?),
                else_value: Box::new(self.get(*else_value)?),
            },
            ValueKind::Slice {
                value: input,
                offset,
            } => slice(self.name(*input)?, *offset, value.ty.width, o),
            ValueKind::Extend { value: input, sign } => {
                let width = self.design.values[input.0].ty.width;
                let extra = value.ty.width - width;
                if width == 0 {
                    constant(&ir::PackedBits::zero(value.ty.width), o)
                } else if extra == 0 {
                    self.get(*input)?
                } else {
                    let padding = if *sign {
                        v::Expr::Repeat {
                            count: Box::new(number(u64::from(extra), None, o)),
                            value: Box::new(slice(self.name(*input)?, width - 1, 1, o)),
                        }
                    } else {
                        constant(&ir::PackedBits::zero(extra), o)
                    };
                    concat(vec![padding, self.get(*input)?])
                }
            }
            ValueKind::Insert {
                base,
                value: input,
                offset,
            } => {
                let width = self.design.values[input.0].ty.width;
                if width == 0 {
                    self.get(*base)?
                } else {
                    let mut parts = Vec::new();
                    let high = value.ty.width - offset - width;
                    if high > 0 {
                        parts.push(slice(self.name(*base)?, offset + width, high, o));
                    }
                    parts.push(self.get(*input)?);
                    if *offset > 0 {
                        parts.push(slice(self.name(*base)?, 0, *offset, o));
                    }
                    concat(parts)
                }
            }
        })
    }
}

pub fn to_ast(design: &ir::Design) -> Result<(v::Ast, v::OriginMap), Error> {
    let compiled = design.compile()?;
    let d = &compiled.design;
    let mut origins = v::OriginMap::new();
    let generated = origin(&mut origins, None)?;
    let mut module = v::Module {
        origin: generated,
        name: ident(&d.name)?,
        params: Default::default(),
        ports: Default::default(),
        items: Vec::new(),
    };
    let mut used = HashSet::new();
    let mut public_names = Vec::new();
    for s in &d.signals {
        // Keep the first occurrence of a user name, deterministically suffix collisions.
        public_names.push(fresh(&mut used, &s.name)?);
    }
    let mut clocks = HashMap::new();
    for domain in &d.domains {
        let name = fresh(&mut used, &domain.name)?;
        clocks.insert(domain.id, name);
        module.ports.push(v::Port {
            origin: generated,
            flow: v::Flow::Input,
            ty: v::Ty::Logic,
            name,
        });
    }
    let mut signals = public_names.clone();
    let mut clocked = vec![false; d.signals.len()];
    for driver in &compiled.drivers {
        if matches!(driver.domain, WriteDomain::Clocked(_)) {
            clocked[driver.signal.0] = true;
        }
    }
    for (i, s) in d.signals.iter().enumerate() {
        if s.ty.width == 0 {
            continue;
        }
        let o = origin(&mut origins, s.source.as_ref())?;
        if let Some(direction) = s.port {
            module.ports.push(v::Port {
                origin: o,
                flow: match direction {
                    ir::PortDirection::Input => v::Flow::Input,
                    ir::PortDirection::Output => v::Flow::Output,
                },
                ty: ty(s.ty, o),
                name: public_names[i],
            });
        }
        if clocked[i] {
            signals[i] = fresh(&mut used, &format!("__ir_state_{i}"))?;
        }
        if s.port.is_none() || clocked[i] {
            module.items.push(v::Item::Decl(v::Decl {
                origin: o,
                kind: if clocked[i] {
                    v::DeclKind::Variable
                } else {
                    v::DeclKind::Wire
                },
                ty: ty(s.ty, o),
                name: signals[i],
                dimensions: vec![],
                value: clocked[i].then(|| constant(&s.init, o)),
            }));
        }
        if clocked[i] && s.port == Some(ir::PortDirection::Output) {
            module.items.push(v::Item::Assign(v::Assign {
                origin: o,
                target: v::Target::Name(public_names[i]),
                value: v::Expr::Name(signals[i]),
            }));
        }
        if s.port != Some(ir::PortDirection::Input)
            && !compiled.drivers.iter().any(|driver| driver.signal.0 == i)
        {
            module.items.push(v::Item::Assign(v::Assign {
                origin: o,
                target: v::Target::Name(signals[i]),
                value: constant(&s.init, o),
            }));
        }
    }
    let mut names = Vec::new();
    for (i, value) in d.values.iter().enumerate() {
        names.push(if value.ty.width == 0 {
            None
        } else {
            Some(fresh(&mut used, &format!("__ir_value_{i}"))?)
        });
    }
    // Unlike Verilog, the IR's arithmetic shift always replicates the top bit.
    let mut signed_names = vec![None; d.values.len()];
    for value in &d.values {
        if let ValueKind::Binary {
            op: ir::BinaryOp::ArithmeticShr,
            lhs,
            ..
        } = value.kind
        {
            let source_ty = d.values[lhs.0].ty;
            if source_ty.width > 0 && !source_ty.signed && signed_names[lhs.0].is_none() {
                let name = fresh(&mut used, &format!("__ir_signed_{}", lhs.0))?;
                signed_names[lhs.0] = Some(name);
                module.items.push(v::Item::Decl(v::Decl {
                    origin: generated,
                    kind: v::DeclKind::Wire,
                    ty: ty(
                        ir::ValueType {
                            signed: true,
                            ..source_ty
                        },
                        generated,
                    ),
                    name,
                    dimensions: vec![],
                    value: Some(v::Expr::Name(
                        names[lhs.0].ok_or_else(|| Error("missing shift operand".into()))?,
                    )),
                }));
            }
        }
    }
    let values = Values {
        design: d,
        names: &names,
        signals: &signals,
        signed_names: &signed_names,
    };
    for (i, value) in d.values.iter().enumerate() {
        let Some(name) = names[i] else {
            continue;
        };
        let o = origin(&mut origins, value.source.as_ref())?;
        module.items.push(v::Item::Decl(v::Decl {
            origin: o,
            kind: v::DeclKind::Wire,
            ty: ty(value.ty, o),
            name,
            dimensions: vec![],
            value: Some(values.expression(value, o)?),
        }));
    }
    for driver in &compiled.drivers {
        if d.signals[driver.signal.0].ty.width == 0 {
            continue;
        }
        let source = d
            .writes
            .iter()
            .rev()
            .find(|w| w.target.signal == driver.signal)
            .and_then(|w| w.source.as_ref())
            .or(d.signals[driver.signal.0].source.as_ref());
        let o = origin(&mut origins, source)?;
        let assign = v::Assign {
            origin: o,
            target: v::Target::Name(signals[driver.signal.0]),
            value: values.get(driver.value)?,
        };
        module.items.push(match driver.domain {
            WriteDomain::Comb => v::Item::Assign(assign),
            WriteDomain::Clocked(domain) => v::Item::Process(v::Process {
                origin: o,
                kind: v::ProcessKind::Clocked(vec![v::Event {
                    edge: v::Edge::Pos,
                    signal: v::Expr::Name(
                        *clocks
                            .get(&domain)
                            .ok_or_else(|| Error(format!("missing clock domain {domain:?}")))?,
                    ),
                }]),
                body: vec![v::Stmt::Assign {
                    assign,
                    kind: v::AssignKind::Nonblocking,
                }],
            }),
        });
    }
    Ok((v::Ast::Module(module), origins))
}
