use crate::{
    domain::{Clock, Domain},
    elab::handle::{ClockHandle, RegisterHandle},
    value::{Shape, Raw},
};
use std::{any::TypeId, ptr};

use super::with_context;

#[must_use]
pub fn clock_input<D: 'static>(name: &str) -> ClockHandle {
    with_context(|cx| {
        let (index, domain) = cx.intern_domain(TypeId::of::<D>(), name);
        if cx.parameters.iter().any(|parameter| matches!(parameter, air::Parameter::Clock { domain: existing } if ptr::eq(*existing, domain))) {
            assert!(
                domain.name == name,
                "clock domains are singletons: domain is already bound to clock {:?}",
                domain.name
            );
        } else {
            assert!(
                cx.parameters.iter().all(|parameter| match parameter {
                    air::Parameter::Clock { domain } => domain.name != name,
                    air::Parameter::Value { name: existing, .. }
                    | air::Parameter::Interface { name: existing, .. } => *existing != name,
                }),
                "duplicate module parameter name"
            );
            cx.ensure_clock(domain);
        }
        ClockHandle {
            design_id: cx.design_id,
            index,
        }
    })
}

pub fn register<D: Domain, T: Shape>(
    clock: Clock<D>,
    init: Option<T>,
) -> (RegisterHandle, Raw<T>) {
    with_context(|cx| {
        let handle = clock.handle.expect("clock has no handle");
        cx.check_design(handle.design_id);
        let domain = cx.domains[handle.index];
        let canonical = cx
            .domain_cache
            .get(&clock.identity())
            .expect("unregistered clock domain")
            .1;
        assert!(ptr::eq(domain, canonical), "clock domain identity mismatch");
        add_register::<T>(cx, domain, init)
    })
}

/// Registers into the enclosing module's clock domain without a branded
/// [`Clock`] value; used by the async method/rule FSM machinery, whose
/// state registers belong to the module's clock.
pub fn register_default<T: Shape>(init: Option<T>) -> (RegisterHandle, Raw<T>) {
    with_context(|cx| {
        let domain = cx.module_clock();
        add_register::<T>(cx, domain, init)
    })
}

fn add_register<'dsn, T: Shape>(
    cx: &mut super::Context<'dsn>,
    domain: &'dsn air::Domain<'dsn>,
    init: Option<T>,
) -> (RegisterHandle, Raw<T>) {
    let (handle, value) = {
        cx.ensure_clock(domain);
        let ty = cx.ty::<T>();
        let reset = match init {
            Some(init) => air::RegisterReset::Synchronous(init.air_value(cx)),
            None => air::RegisterReset::Uninitialized,
        };
        let index = cx.register_handles.len();
        let name = cx.alloc_str(&format!("reg_{index}"));
        let register = cx.add_register(air::Register {
            name,
            ty,
            domain,
            reset,
        });
        cx.record_register_handle(register);
        let handle = RegisterHandle {
            design_id: cx.design_id,
            index,
        };
        let expr = cx.alloc(air::Expr {
            ty,
            kind: air::ExprKind::Register(register),
        });
        (handle, cx.push_expr(expr))
    };
    (handle, Raw::from_handle(value))
}

pub fn read_register<T: Shape>(handle: RegisterHandle) -> Raw<T> {
    with_context(|cx| {
        cx.check_design(handle.design_id);
        let (register, owner) = cx.register_handles[handle.index];
        assert_eq!(owner, cx.id, "cannot read a register from another module");
        let ty = cx.ty::<T>();
        assert!(ptr::eq(register.ty, ty), "register read type mismatch");
        let expr = cx.alloc(air::Expr {
            ty,
            kind: air::ExprKind::Register(register),
        });
        Raw::from_handle(cx.push_expr(expr))
    })
}

pub fn write_register<T: Shape>(handle: RegisterHandle, value: Raw<T>) {
    with_context(|cx| {
        cx.check_design(handle.design_id);
        let register = cx.register_for_write(handle.index);
        let value = cx.typed_expr::<T>(value.handle());
        assert!(
            ptr::eq(register.ty, value.ty),
            "register write type mismatch"
        );
        cx.emit(air::Statement::RegisterWrite { register, value });
    });
}
