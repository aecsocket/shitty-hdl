pub mod bits;

pub(crate) mod expr;
pub(crate) mod runtime;
pub(crate) mod signal;

pub use bits::PackedBits;
pub use expr::Value;
pub(crate) use expr::shape_type;
pub use runtime::elaborate_named as elaborate;
use std::{
    any::TypeId,
    collections::{BTreeSet, HashMap, VecDeque},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DesignId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SignalId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValueId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DomainId(TypeId);
impl DomainId {
    pub fn of<L: 'static>() -> Self {
        Self(TypeId::of::<L>())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueType {
    pub width: u32,
    pub signed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
}
#[derive(Clone, Debug)]
pub struct Signal {
    pub name: String,
    pub ty: ValueType,
    pub init: PackedBits,
    pub port: Option<PortDirection>,
    pub source: Option<SourceLocation>,
}
#[derive(Clone, Debug)]
pub struct ClockDomain {
    pub name: String,
    pub id: DomainId,
}
#[derive(Clone, Debug)]
pub struct ValueData {
    pub ty: ValueType,
    pub kind: ValueKind,
    pub source: Option<SourceLocation>,
}
#[derive(Clone, Debug)]
pub enum ValueKind {
    Constant(PackedBits),
    Read(SignalId),
    Unary {
        op: UnaryOp,
        value: ValueId,
    },
    Binary {
        op: BinaryOp,
        lhs: ValueId,
        rhs: ValueId,
    },
    Mux {
        condition: ValueId,
        then_value: ValueId,
        else_value: ValueId,
    },
    Slice {
        value: ValueId,
        offset: u32,
    },
    Extend {
        value: ValueId,
        sign: bool,
    },
    Insert {
        base: ValueId,
        value: ValueId,
        offset: u32,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    ArithmeticShr,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WriteDomain {
    Comb,
    Clocked(DomainId),
}
#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub signal: SignalId,
    pub offset: u32,
    pub width: u32,
}
#[derive(Clone, Debug)]
pub struct Write {
    pub target: Target,
    pub value: ValueId,
    pub condition: Option<ValueId>,
    pub domain: WriteDomain,
    pub source: Option<SourceLocation>,
}
#[derive(Clone, Debug)]
pub struct Driver {
    pub signal: SignalId,
    pub value: ValueId,
    pub domain: WriteDomain,
}
#[derive(Clone, Debug)]
pub struct Design {
    pub id: DesignId,
    pub name: String,
    pub signals: Vec<Signal>,
    pub values: Vec<ValueData>,
    pub domains: Vec<ClockDomain>,
    pub writes: Vec<Write>,
}
#[derive(Clone, Debug)]
pub struct CompiledDesign {
    pub design: Design,
    pub drivers: Vec<Driver>,
    pub comb_order: Vec<(SignalId, u32)>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
fn require(ok: bool, message: impl Into<String>) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error(message.into()))
    }
}
fn in_bounds(offset: u32, width: u32, total: u32) -> bool {
    offset <= total && width <= total - offset
}
impl ValueKind {
    pub(crate) fn operands(&self) -> Vec<ValueId> {
        match *self {
            Self::Constant(_) | Self::Read(_) => vec![],
            Self::Unary { value, .. } | Self::Slice { value, .. } | Self::Extend { value, .. } => {
                vec![value]
            }
            Self::Binary { lhs, rhs, .. } => vec![lhs, rhs],
            Self::Mux {
                condition,
                then_value,
                else_value,
            } => vec![condition, then_value, else_value],
            Self::Insert { base, value, .. } => vec![base, value],
        }
    }
}
impl Design {
    pub fn new(name: impl Into<String>) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            id: DesignId(NEXT.fetch_add(1, Ordering::Relaxed)),
            name: name.into(),
            signals: vec![],
            values: vec![],
            domains: vec![],
            writes: vec![],
        }
    }
    pub fn add_value(
        &mut self,
        ty: ValueType,
        kind: ValueKind,
        source: Option<SourceLocation>,
    ) -> ValueId {
        let id = ValueId(self.values.len());
        self.values.push(ValueData { ty, kind, source });
        id
    }
    pub fn add_signal(&mut self, signal: Signal) -> SignalId {
        let id = SignalId(self.signals.len());
        self.signals.push(signal);
        id
    }
    pub fn add_domain<L: 'static>(&mut self, name: String) -> DomainId {
        let id = DomainId::of::<L>();
        if !self.domains.iter().any(|domain| domain.id == id) {
            self.domains.push(ClockDomain { name, id });
        }
        id
    }
    pub(crate) fn value_order(&self) -> Result<Vec<usize>, Error> {
        let mut degree = vec![0; self.values.len()];
        let mut users = vec![vec![]; self.values.len()];
        for (i, v) in self.values.iter().enumerate() {
            for op in v.kind.operands() {
                require(
                    op.0 < self.values.len(),
                    format!("value {i}: invalid operand {}", op.0),
                )?;
                degree[i] += 1;
                users[op.0].push(i);
            }
        }
        let mut ready: VecDeque<_> = degree
            .iter()
            .enumerate()
            .filter_map(|(i, &d)| (d == 0).then_some(i))
            .collect();
        let mut order = vec![];
        while let Some(i) = ready.pop_front() {
            order.push(i);
            for &j in &users[i] {
                degree[j] -= 1;
                if degree[j] == 0 {
                    ready.push_back(j);
                }
            }
        }
        require(
            order.len() == self.values.len(),
            "cycle in value expression graph",
        )?;
        Ok(order)
    }
    fn validate(&self) -> Result<(), Error> {
        let mut domains = BTreeSet::new();
        for domain in &self.domains {
            require(
                domains.insert(domain.id),
                format!("duplicate clock domain {:?}", domain.id),
            )?;
        }
        self.value_order()?;
        for (i, s) in self.signals.iter().enumerate() {
            require(
                s.init.width() == s.ty.width,
                format!("signal {i}: initial width mismatch"),
            )?;
        }
        let boolean = ValueType {
            width: 1,
            signed: false,
        };
        for (i, v) in self.values.iter().enumerate() {
            let ty = |id: ValueId| self.values[id.0].ty;
            let valid = match &v.kind {
                ValueKind::Constant(b) => b.width() == v.ty.width,
                ValueKind::Read(s) => self.signals.get(s.0).is_some_and(|s| s.ty == v.ty),
                ValueKind::Unary { value, .. } => ty(*value) == v.ty,
                ValueKind::Binary { op, lhs, rhs } => match op {
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge => ty(*lhs) == ty(*rhs) && v.ty == boolean,
                    BinaryOp::Shl | BinaryOp::Shr | BinaryOp::ArithmeticShr => ty(*lhs) == v.ty,
                    _ => ty(*lhs) == ty(*rhs) && ty(*lhs) == v.ty,
                },
                ValueKind::Mux {
                    condition,
                    then_value,
                    else_value,
                } => {
                    ty(*condition) == boolean && ty(*then_value) == v.ty && ty(*else_value) == v.ty
                }
                ValueKind::Slice { value, offset } => {
                    in_bounds(*offset, v.ty.width, ty(*value).width)
                }
                ValueKind::Extend { value, sign } => {
                    v.ty.width >= ty(*value).width
                        && *sign == ty(*value).signed
                        && v.ty.signed == ty(*value).signed
                }
                ValueKind::Insert {
                    base,
                    value,
                    offset,
                } => ty(*base) == v.ty && in_bounds(*offset, ty(*value).width, v.ty.width),
            };
            require(
                valid,
                format!("value {i}: invalid ID, type, width, or bounds"),
            )?;
        }
        for (i, w) in self.writes.iter().enumerate() {
            let s = self
                .signals
                .get(w.target.signal.0)
                .ok_or_else(|| Error(format!("write {i}: invalid signal")))?;
            let v = self
                .values
                .get(w.value.0)
                .ok_or_else(|| Error(format!("write {i}: invalid value")))?;
            require(
                v.ty.width == w.target.width
                    && in_bounds(w.target.offset, w.target.width, s.ty.width),
                format!("write {i}: width or bounds mismatch"),
            )?;
            if let Some(c) = w.condition {
                require(
                    self.values.get(c.0).is_some_and(|v| v.ty == boolean),
                    format!("write {i}: condition must be unsigned width 1"),
                )?;
            }
            if let WriteDomain::Clocked(d) = w.domain {
                require(
                    domains.contains(&d),
                    format!("write {i}: invalid domain {d:?}"),
                )?;
            }
            require(
                s.port != Some(PortDirection::Input),
                format!("input signal {} cannot have a driver", s.name),
            )?;
        }
        Ok(())
    }
    fn fold_constants(&mut self) -> Result<(), Error> {
        for i in self.value_order()? {
            let v = &self.values[i];
            let constant = |id: ValueId| match &self.values[id.0].kind {
                ValueKind::Constant(bits) => Some(bits),
                _ => None,
            };
            let folded = match &v.kind {
                ValueKind::Constant(_) | ValueKind::Read(_) => None,
                ValueKind::Unary { op, value } => constant(*value).map(|bits| match op {
                    UnaryOp::Not => bits.not(),
                    UnaryOp::Neg => bits.neg(),
                }),
                ValueKind::Binary { op, lhs, rhs } => {
                    constant(*lhs).zip(constant(*rhs)).map(|(a, b)| match op {
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
                            let cmp = a.cmp(b, self.values[lhs.0].ty.signed);
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
                    })
                }
                ValueKind::Mux {
                    condition,
                    then_value,
                    else_value,
                } => {
                    if let Some(condition) = constant(*condition) {
                        let chosen = if condition.bit(0) {
                            *then_value
                        } else {
                            *else_value
                        };
                        self.values[i].kind = match constant(chosen) {
                            Some(bits) => ValueKind::Constant(bits.clone()),
                            None => ValueKind::Slice {
                                value: chosen,
                                offset: 0,
                            },
                        };
                    }
                    continue;
                }
                ValueKind::Slice { value, offset } => {
                    constant(*value).map(|bits| bits.slice(*offset, v.ty.width))
                }
                ValueKind::Extend { value, sign } => {
                    constant(*value).map(|bits| bits.extend(v.ty.width, *sign))
                }
                ValueKind::Insert {
                    base,
                    value,
                    offset,
                } => constant(*base).zip(constant(*value)).map(|(base, value)| {
                    let mut bits = base.clone();
                    bits.replace(*offset, value);
                    bits
                }),
            };
            if let Some(bits) = folded {
                self.values[i].kind = ValueKind::Constant(bits);
            }
        }
        Ok(())
    }
    pub fn compile(&self) -> Result<CompiledDesign, Error> {
        self.validate()?;
        let mut design = self.clone();
        design.fold_constants()?;
        let mut drivers: Vec<Driver> = vec![];
        let mut indices: HashMap<SignalId, usize> = HashMap::new();
        for w in &self.writes {
            if w.target.width == 0
        || w.condition.is_some_and(
          |id| matches!(&design.values[id.0].kind, ValueKind::Constant(bits) if !bits.bit(0)),
        )
      {
        continue;
      }
            let s = &self.signals[w.target.signal.0];
            let index = if let Some(&i) = indices.get(&w.target.signal) {
                require(
                    drivers[i].domain == w.domain,
                    format!("signal {} driven from different domains", s.name),
                )?;
                i
            } else {
                let kind = match w.domain {
                    WriteDomain::Comb => ValueKind::Constant(s.init.clone()),
                    WriteDomain::Clocked(_) => ValueKind::Read(w.target.signal),
                };
                let value = design.add_value(s.ty, kind, None);
                let i = drivers.len();
                drivers.push(Driver {
                    signal: w.target.signal,
                    value,
                    domain: w.domain,
                });
                indices.insert(w.target.signal, i);
                i
            };
            let old = drivers[index].value;
            // A full replacement must not retain dependencies on earlier writes.
            let next = if w.target.offset == 0
                && w.target.width == s.ty.width
                && design.values[w.value.0].ty == s.ty
            {
                w.value
            } else {
                design.add_value(
                    s.ty,
                    ValueKind::Insert {
                        base: old,
                        value: w.value,
                        offset: w.target.offset,
                    },
                    w.source.clone(),
                )
            };
            let next = if let Some(condition) = w.condition {
                if let ValueKind::Constant(ref b) = design.values[condition.0].kind {
                    if b.bit(0) { next } else { old }
                } else {
                    design.add_value(
                        s.ty,
                        ValueKind::Mux {
                            condition,
                            then_value: next,
                            else_value: old,
                        },
                        w.source.clone(),
                    )
                }
            } else {
                next
            };
            drivers[index].value = next;
        }
        design.fold_constants()?;
        let comb_order = bit_order(&design, &drivers)?;
        Ok(CompiledDesign {
            design,
            drivers,
            comb_order,
        })
    }
}

type BitNode = (SignalId, u32);
fn dependencies(
    design: &Design,
    id: ValueId,
    bit: u32,
    memo: &mut HashMap<(ValueId, u32), BTreeSet<BitNode>>,
) -> BTreeSet<BitNode> {
    let root = (id, bit);
    let mut pending = vec![(root, false)];
    while let Some(((id, bit), expanded)) = pending.pop() {
        if memo.contains_key(&(id, bit)) {
            continue;
        }
        let mut out = BTreeSet::new();
        let mut children = Vec::new();
        let v = &design.values[id.0];
        let mut add = |id: ValueId, b: u32| children.push((id, b));
        match v.kind {
            ValueKind::Constant(_) => {}
            ValueKind::Read(s) => {
                out.insert((s, bit));
            }
            ValueKind::Slice { value, offset } => add(value, offset + bit),
            ValueKind::Extend { value, sign } => {
                let width = design.values[value.0].ty.width;
                if bit < width {
                    add(value, bit);
                } else if sign && width > 0 {
                    add(value, width - 1);
                }
            }
            ValueKind::Insert {
                base,
                value,
                offset,
            } => {
                let width = design.values[value.0].ty.width;
                if bit >= offset && bit - offset < width {
                    add(value, bit - offset);
                } else {
                    add(base, bit);
                }
            }
            ValueKind::Mux {
                condition,
                then_value,
                else_value,
            } => {
                add(condition, 0);
                add(then_value, bit);
                add(else_value, bit);
            }
            ValueKind::Unary { op, value } => match op {
                UnaryOp::Not => add(value, bit),
                UnaryOp::Neg => {
                    for b in 0..=bit {
                        add(value, b);
                    }
                }
            },
            ValueKind::Binary { op, lhs, rhs } => match op {
                BinaryOp::And | BinaryOp::Or | BinaryOp::Xor => {
                    add(lhs, bit);
                    add(rhs, bit);
                }
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => {
                    for b in 0..=bit {
                        add(lhs, b);
                        add(rhs, b);
                    }
                }
                BinaryOp::Shl | BinaryOp::Shr | BinaryOp::ArithmeticShr
                    if matches!(design.values[rhs.0].kind, ValueKind::Constant(_)) =>
                {
                    let ValueKind::Constant(ref count) = design.values[rhs.0].kind else {
                        unreachable!()
                    };
                    let amount = if count.words().iter().skip(1).any(|&w| w != 0) {
                        u64::MAX
                    } else {
                        count.words().first().copied().unwrap_or(0)
                    };
                    let width = design.values[lhs.0].ty.width;
                    if op == BinaryOp::Shl {
                        if amount <= bit as u64 {
                            add(lhs, bit - amount as u32);
                        }
                    } else if amount < width as u64 && bit as u64 + amount < width as u64 {
                        add(lhs, bit + amount as u32);
                    } else if op == BinaryOp::ArithmeticShr && width > 0 {
                        add(lhs, width - 1);
                    }
                }
                _ => {
                    for b in 0..design.values[lhs.0].ty.width {
                        add(lhs, b);
                    }
                    for b in 0..design.values[rhs.0].ty.width {
                        add(rhs, b);
                    }
                }
            },
        }
        if expanded {
            for child in children {
                out.extend(memo[&child].iter().copied());
            }
            memo.insert((id, bit), out);
        } else {
            pending.push(((id, bit), true));
            for child in children {
                if !memo.contains_key(&child) {
                    pending.push((child, false));
                }
            }
        }
    }
    memo[&root].clone()
}
fn bit_order(design: &Design, drivers: &[Driver]) -> Result<Vec<BitNode>, Error> {
    let mut nodes = vec![];
    let mut indices = HashMap::new();
    for d in drivers.iter().filter(|d| d.domain == WriteDomain::Comb) {
        for b in 0..design.signals[d.signal.0].ty.width {
            indices.insert((d.signal, b), nodes.len());
            nodes.push((d.signal, b, d.value));
        }
    }
    let mut degree = vec![0; nodes.len()];
    let mut users = vec![vec![]; nodes.len()];
    let mut memo = HashMap::new();
    for (i, &(_, b, v)) in nodes.iter().enumerate() {
        for dep in dependencies(design, v, b, &mut memo) {
            if let Some(&j) = indices.get(&dep) {
                degree[i] += 1;
                users[j].push(i);
            }
        }
    }
    let mut ready: VecDeque<_> = degree
        .iter()
        .enumerate()
        .filter_map(|(i, &d)| (d == 0).then_some(i))
        .collect();
    let mut out = vec![];
    while let Some(i) = ready.pop_front() {
        out.push((nodes[i].0, nodes[i].1));
        for &j in &users[i] {
            degree[j] -= 1;
            if degree[j] == 0 {
                ready.push_back(j);
            }
        }
    }
    if out.len() != nodes.len() {
        let i = degree.iter().position(|&d| d != 0).unwrap();
        return Err(Error(format!(
            "combinational cycle involving signal {} bit {}",
            design.signals[nodes[i].0.0].name, nodes[i].1
        )));
    }
    Ok(out)
}
