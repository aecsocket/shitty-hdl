use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::ir::{
    self, Design, DesignId, SourceLocation, ValueId, ValueKind, ValueType, WriteDomain,
};

struct Context {
    design: Design,
    condition: Option<ValueId>,
    domain: WriteDomain,
    modules: Vec<String>,
    counts: HashMap<String, usize>,
}

thread_local! {
    static ACTIVE: RefCell<Vec<Rc<RefCell<Context>>>> = const { RefCell::new(Vec::new()) };
}

fn active() -> Rc<RefCell<Context>> {
    ACTIVE
        .with_borrow(|stack| stack.last().cloned())
        .expect("hardware must be constructed inside elaborate()")
}

fn with<R>(f: impl FnOnce(&mut Context) -> R) -> R {
    f(&mut active().borrow_mut())
}

struct ActiveGuard;
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        ACTIVE.with_borrow_mut(|stack| {
            stack.pop();
        });
    }
}

pub fn elaborate<M: crate::module::Module>(
    build: impl FnOnce() -> M,
) -> Result<(Design, M), ir::Error> {
    elaborate_named(M::NAME, build)
}

pub fn elaborate_named<R>(
    name: impl Into<String>,
    build: impl FnOnce() -> R,
) -> Result<(Design, R), ir::Error> {
    let context = Rc::new(RefCell::new(Context {
        design: Design::new(name),
        condition: None,
        domain: WriteDomain::Comb,
        modules: vec![],
        counts: HashMap::new(),
    }));
    ACTIVE.with_borrow_mut(|stack| stack.push(context.clone()));
    let guard = ActiveGuard;
    let result = build();
    let design = context.borrow().design.clone();
    drop(guard);
    design.compile()?;
    Ok((design, result))
}

pub(crate) fn owner() -> DesignId {
    with(|c| c.design.id)
}

pub(crate) fn check_owner(owner: DesignId) {
    assert_eq!(
        self::owner(),
        owner,
        "handle belongs to a different elaboration"
    );
}

#[track_caller]
pub(crate) fn location() -> Option<SourceLocation> {
    let caller = std::panic::Location::caller();
    Some(SourceLocation {
        file: caller.file().into(),
        line: caller.line(),
        column: caller.column(),
    })
}

#[track_caller]
pub(crate) fn value(ty: ValueType, kind: ValueKind) -> (DesignId, ValueId) {
    let source = location();
    with(|c| (c.design.id, c.design.add_value(ty, kind, source)))
}

#[track_caller]
pub(crate) fn signal(
    ty: ValueType,
    init: ir::PackedBits,
    name: Option<String>,
    port: Option<ir::PortDirection>,
) -> crate::signal::SignalId {
    let source = location();
    with(|c| {
        let name = name.unwrap_or_else(|| format!("signal_{}", c.design.signals.len()));
        let name = if c.modules.len() > 1 {
            format!("{}.{}", c.modules[1..].join("."), name)
        } else {
            name
        };
        let port = if c.modules.len() > 1 { None } else { port };
        let raw = c.design.add_signal(ir::Signal {
            name,
            ty,
            init,
            port,
            source,
        });
        crate::signal::SignalId {
            owner: c.design.id,
            raw,
        }
    })
}

#[track_caller]
pub(crate) fn physical_signal(name: &str, direction: ir::PortDirection) -> crate::signal::SignalId {
    let source = location();
    with(|c| {
        let raw = c.design.add_signal(ir::Signal {
            name: name.into(),
            ty: ValueType {
                width: 1,
                signed: false,
            },
            init: ir::PackedBits::zero(1),
            port: Some(direction),
            source,
        });
        crate::signal::SignalId {
            owner: c.design.id,
            raw,
        }
    })
}

pub(crate) fn physical_invert(target: crate::signal::SignalId, input: crate::signal::SignalId) {
    check_owner(target.owner);
    check_owner(input.owner);
    let value = read(
        input,
        0,
        ValueType {
            width: 1,
            signed: false,
        },
    )
    .1;
    let value = invert(value);
    with(|c| {
        c.design.writes.push(ir::Write {
            target: ir::Target {
                signal: target.raw,
                offset: 0,
                width: 1,
            },
            value,
            condition: None,
            domain: WriteDomain::Comb,
            source: None,
        })
    });
}

#[track_caller]
pub(crate) fn read(id: crate::signal::SignalId, offset: u32, ty: ValueType) -> (DesignId, ValueId) {
    check_owner(id.owner);
    let source = location();
    with(|c| {
        let signal_ty = c.design.signals[id.raw.0].ty;
        assert!(
            offset <= signal_ty.width && ty.width <= signal_ty.width - offset,
            "signal view is out of bounds"
        );
        let read = c
            .design
            .add_value(signal_ty, ValueKind::Read(id.raw), source.clone());
        let value = if offset == 0 && ty == signal_ty {
            read
        } else {
            c.design.add_value(
                ty,
                ValueKind::Slice {
                    value: read,
                    offset,
                },
                source,
            )
        };
        (c.design.id, value)
    })
}

#[track_caller]
pub(crate) fn write(
    id: crate::signal::SignalId,
    offset: u32,
    width: u32,
    owner: DesignId,
    value: ValueId,
) {
    check_owner(id.owner);
    check_owner(owner);
    let source = location();
    with(|c| {
        c.design.writes.push(ir::Write {
            target: ir::Target {
                signal: id.raw,
                offset,
                width,
            },
            value,
            condition: c.condition,
            domain: c.domain,
            source,
        })
    });
}

pub(crate) fn boolean(op: ir::BinaryOp, a: ValueId, b: ValueId) -> ValueId {
    value(
        ValueType {
            width: 1,
            signed: false,
        },
        ValueKind::Binary { op, lhs: a, rhs: b },
    )
    .1
}

pub(crate) fn invert(value: ValueId) -> ValueId {
    self::value(
        ValueType {
            width: 1,
            signed: false,
        },
        ValueKind::Unary {
            op: ir::UnaryOp::Not,
            value,
        },
    )
    .1
}

pub(crate) fn parent() -> (DesignId, Option<ValueId>, WriteDomain) {
    with(|c| (c.design.id, c.condition, c.domain))
}

pub(crate) fn check_parent(owner: DesignId, condition: Option<ValueId>, domain: WriteDomain) {
    assert_eq!(
        parent(),
        (owner, condition, domain),
        "conditional chain crossed an elaboration scope"
    );
}

enum Restore {
    Condition(Option<ValueId>),
    Domain(WriteDomain),
    Module(usize),
}

pub struct Scope {
    context: Rc<RefCell<Context>>,
    restore: Restore,
}

impl Drop for Scope {
    fn drop(&mut self) {
        let mut c = self.context.borrow_mut();
        match self.restore {
            Restore::Condition(old) => c.condition = old,
            Restore::Domain(old) => c.domain = old,
            Restore::Module(len) => c.modules.truncate(len),
        }
    }
}

pub(crate) fn condition(value: ValueId) -> Scope {
    let context = active();
    let mut c = context.borrow_mut();
    let old = c.condition;
    c.condition = Some(if let Some(parent) = old {
        c.design.add_value(
            ValueType {
                width: 1,
                signed: false,
            },
            ValueKind::Binary {
                op: ir::BinaryOp::And,
                lhs: parent,
                rhs: value,
            },
            None,
        )
    } else {
        value
    });
    drop(c);
    Scope {
        context,
        restore: Restore::Condition(old),
    }
}

pub(crate) fn domain(domain: WriteDomain) -> Scope {
    let context = active();
    let old = std::mem::replace(&mut context.borrow_mut().domain, domain);
    Scope {
        context,
        restore: Restore::Domain(old),
    }
}

pub fn enter_module(name: &str) -> Scope {
    let context = active();
    let mut c = context.borrow_mut();
    let len = c.modules.len();
    let key = format!("{}.{name}", c.modules.join("."));
    let count = c.counts.entry(key).or_default();
    let name = if *count == 0 {
        name.to_owned()
    } else {
        format!("{name}_{count}")
    };
    *count += 1;
    c.modules.push(name);
    drop(c);
    Scope {
        context,
        restore: Restore::Module(len),
    }
}

pub(crate) fn type_domain<T: 'static>() -> ir::DomainId {
    with(|c| {
        let label: String = std::any::type_name::<T>()
            .rsplit("::")
            .next()
            .unwrap_or("domain")
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect();
        c.design
            .add_domain::<T>(format!("clk{}_{}", c.design.domains.len(), label))
    })
}
