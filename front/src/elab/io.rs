use std::ptr;

use crate::{domain::Clock, value::Shape};

use super::{ActionHandle, Context, ExprHandle, with_context};

impl<'dsn> Context<'dsn> {
    fn io_domain(&mut self, clock: Clock) -> &'dsn air::Domain<'dsn> {
        let domain = match clock.handle {
            None => match self.domain_cache.get(&clock.identity) {
                Some(&(_, domain)) => domain,
                None => self.intern_domain(clock.identity, "clk").1,
            },
            Some(handle) => {
                self.check_design(handle.design_id);
                let domain = self.domains[handle.index];
                let canonical = self
                    .domain_cache
                    .get(&clock.identity)
                    .expect("unregistered clock domain")
                    .1;
                assert!(ptr::eq(domain, canonical), "clock domain identity mismatch");
                domain
            }
        };
        self.ensure_clock(domain);
        domain
    }

    fn check_io_name(&self, name: &str) {
        assert!(
            self.outputs.iter().all(|output| output.name != name)
                && self.parameters.iter().all(|parameter| match parameter {
                    air::Parameter::Value { name: existing, .. }
                    | air::Parameter::Interface { name: existing, .. } => *existing != name,
                    air::Parameter::Clock { domain } => domain.name != name,
                }),
            "duplicate physical IO port name"
        );
    }

    fn io_register(
        &mut self,
        domain: &'dsn air::Domain<'dsn>,
    ) -> (&'dsn air::Register<'dsn>, &'dsn air::Expr<'dsn>) {
        let ty = self.ty::<bool>();
        let reset = air::RegisterReset::Synchronous(false.air_value(self));
        let name = self.alloc_str(&format!("reg_{}", self.register_handles.len()));
        let register = self.add_register(air::Register {
            name,
            ty,
            domain,
            reset,
        });
        self.record_register_handle(register);
        let value = self.alloc(air::Expr {
            ty,
            kind: air::ExprKind::Register(register),
        });
        (register, value)
    }

    fn io_polarity(&self, value: &'dsn air::Expr<'dsn>, active_low: bool) -> &'dsn air::Expr<'dsn> {
        if active_low {
            self.alloc(air::Expr {
                ty: value.ty,
                kind: air::ExprKind::Unary {
                    op: air::UnaryOp::Not,
                    operand: value,
                },
            })
        } else {
            value
        }
    }
}

pub(crate) fn output(clock: Clock, name: &str, active_low: bool) -> (ExprHandle, ActionHandle) {
    with_context(|cx| {
        cx.in_root(|cx| {
            let domain = cx.io_domain(clock);
            cx.check_io_name(name);
            let (register, value) = cx.io_register(domain);
            let output = cx.alloc(air::Output {
                name: cx.alloc_str(name),
                value: cx.io_polarity(value, active_low),
            });
            cx.outputs.push(output);
            let name = format!("io_write_{name}");
            let signature = cx.action_signature::<bool, ()>(&name);
            let argument = cx.alloc(air::Expr {
                ty: signature.inputs[0].ty,
                kind: air::ExprKind::Argument(&signature.inputs[0]),
            });
            let statements = cx.alloc_slice([
                air::Statement::RegisterWrite {
                    register,
                    value: argument,
                },
                air::Statement::Return(None),
            ]);
            let method = cx.add_method(air::Method {
                signature,
                guard: None,
                body: air::MethodBody::Action(air::Block { statements }),
            });
            let writer = cx.push_action(air::MethodRef::Local(method));
            (cx.push_expr(value), writer)
        })
    })
}

pub(crate) fn input(clock: Clock, name: &str, active_low: bool) -> ExprHandle {
    with_context(|cx| {
        cx.in_root(|cx| {
            let domain = cx.io_domain(clock);
            cx.check_io_name(name);
            let ty = cx.ty::<bool>();
            let parameter = cx.add_parameter(air::Parameter::Value {
                name: cx.alloc_str(name),
                ty,
            });
            let value = cx.alloc(air::Expr {
                ty,
                kind: air::ExprKind::Parameter(parameter),
            });
            let value = cx.io_polarity(value, active_low);
            let (first, first_value) = cx.io_register(domain);
            let (second, second_value) = cx.io_register(domain);
            let statements = cx.alloc_slice([
                air::Statement::RegisterWrite {
                    register: first,
                    value,
                },
                air::Statement::RegisterWrite {
                    register: second,
                    value: first_value,
                },
            ]);
            let name = cx.alloc_str(&format!("rule_{}", cx.rules.len()));
            cx.add_rule(air::Rule {
                name,
                guard: None,
                body: air::Block { statements },
            });
            cx.push_expr(second_value)
        })
    })
}
