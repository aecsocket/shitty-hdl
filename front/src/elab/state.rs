use crate::{
    domain::Clock,
    elab::handle::{ClockHandle, RegisterHandle},
    value::{Shape, Value},
};
use std::{any::TypeId, ptr};

use super::with_context;

#[must_use]
pub(crate) fn clock_input<D: 'static>(name: &str) -> ClockHandle {
    with_context(|cx| {
        let (index, domain) = cx.intern_domain(TypeId::of::<D>(), name);
        if !cx.parameters.iter().any(|parameter| matches!(parameter, air::Parameter::Clock { domain: existing } if ptr::eq(*existing, domain))) {
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

pub(crate) fn register<T: Shape>(clock: Clock, init: Option<T>) -> (RegisterHandle, Value<T>) {
    let (handle, value) = with_context(|cx| {
        let domain = match clock.handle {
            None => match cx.domain_cache.get(&clock.identity) {
                Some(&(_, domain)) => domain,
                None => cx.intern_domain(clock.identity, "clk").1,
            },
            Some(handle) => {
                cx.check_design(handle.design_id);
                let domain = cx.domains[handle.index];
                let canonical = cx
                    .domain_cache
                    .get(&clock.identity)
                    .expect("unregistered clock domain")
                    .1;
                assert!(ptr::eq(domain, canonical), "clock domain identity mismatch");
                domain
            }
        };
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
    });
    (handle, Value::from_handle(value))
}

pub(crate) fn write_register<T: Shape>(handle: RegisterHandle, value: Value<T>) {
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
