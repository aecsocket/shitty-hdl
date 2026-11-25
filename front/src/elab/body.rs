use std::{
    ptr,
    task::{Context as TaskContext, Poll, Waker},
};

use crate::{
    domain::Domain,
    module::AsyncCx,
    value::{IntoValues, Shape, Value},
};

use super::context::Branch;
use super::{ActionHandle, BranchHandle, Context, DesignId, ExprHandle, with_context};

struct BodyScope {
    design_id: DesignId,
    depth: usize,
    active: bool,
}

impl BodyScope {
    fn begin() -> Self {
        with_context(|cx| {
            let depth = cx.frames.len();
            cx.begin_body();
            Self {
                design_id: cx.design_id,
                depth,
                active: true,
            }
        })
    }
}

impl Drop for BodyScope {
    fn drop(&mut self) {
        if self.active {
            with_context(|cx| {
                cx.check_design(self.design_id);
                cx.frames.truncate(self.depth);
            });
        }
    }
}

pub fn define_action<D: Domain, In: Shape, Out: Shape>(
    guard: Option<ExprHandle>,
    f: impl AsyncFnOnce(AsyncCx, Value<D, In>) -> Out::Values<D>,
) -> ActionHandle {
    let definition = with_context(|cx| {
        assert!(
            cx.frames.is_empty(),
            "define actions at module scope, not inside action bodies"
        );
        let name = format!("action_{}", cx.methods.len());
        let signature = cx.action_signature::<In, Out>(&name);
        let definition = cx.definitions.len();
        cx.definitions.push(signature);
        definition
    });
    let mut scope = BodyScope::begin();
    let input = with_context(|cx| {
        let signature = cx.definitions[definition];
        let input_ty = cx.ty::<In>();
        let inputs: Vec<_> = signature
            .inputs
            .iter()
            .map(|argument| {
                cx.alloc(air::Expr {
                    ty: argument.ty,
                    kind: air::ExprKind::Argument(argument),
                })
            })
            .collect();
        let kind = match input_ty {
            air::Type::Unit => air::ExprKind::Unit,
            air::Type::Tuple(_) => air::ExprKind::Tuple(cx.alloc_slice(inputs)),
            _ => inputs[0].kind,
        };
        let input = cx.alloc(air::Expr { ty: input_ty, kind });
        cx.push_expr(input)
    });
    let mut future = std::pin::pin!(f(AsyncCx::new(), Value::from_handle(input)));
    let result = {
        let mut task = TaskContext::from_waker(Waker::noop());
        match future.as_mut().poll(&mut task) {
            Poll::Ready(value) => value,
            Poll::Pending => {
                panic!("action bodies cannot suspend; use a Method for multi-cycle bodies")
            }
        }
    };
    let result = Out::pack_values(result).handle();
    let handle = with_context(|cx| {
        let result = cx.typed_expr::<Out>(result);
        cx.emit(air::Statement::Return(
            if matches!(result.ty, air::Type::Unit) {
                None
            } else {
                Some(result)
            },
        ));
        let body = cx.end_body();
        let signature = cx.definitions[definition];
        let guard = guard.map(|handle| cx.typed_expr::<bool>(handle));
        let method = cx.add_method(air::Method {
            signature,
            guard,
            body: air::MethodBody::Action(body),
        });
        cx.push_action(air::MethodRef::Local(method))
    });
    scope.active = false;
    handle
}

pub fn define_action_sync<D: Domain, In: Shape, Out: Shape>(
    guard: Option<ExprHandle>,
    f: impl FnOnce(AsyncCx, Value<D, In>) -> Out::Values<D>,
) -> ActionHandle {
    let definition = with_context(|cx| {
        assert!(
            cx.frames.is_empty(),
            "define actions at module scope, not inside action bodies"
        );
        let name = format!("action_{}", cx.methods.len());
        let signature = cx.action_signature::<In, Out>(&name);
        let definition = cx.definitions.len();
        cx.definitions.push(signature);
        definition
    });
    let mut scope = BodyScope::begin();
    let input = with_context(|cx| {
        let signature = cx.definitions[definition];
        let input_ty = cx.ty::<In>();
        let inputs: Vec<_> = signature
            .inputs
            .iter()
            .map(|argument| {
                cx.alloc(air::Expr {
                    ty: argument.ty,
                    kind: air::ExprKind::Argument(argument),
                })
            })
            .collect();
        let kind = match input_ty {
            air::Type::Unit => air::ExprKind::Unit,
            air::Type::Tuple(_) => air::ExprKind::Tuple(cx.alloc_slice(inputs)),
            _ => inputs[0].kind,
        };
        let input = cx.alloc(air::Expr { ty: input_ty, kind });
        cx.push_expr(input)
    });
    let result = f(AsyncCx::new(), Value::from_handle(input));
    let result = Out::pack_values(result).handle();
    let handle = with_context(|cx| {
        let result = cx.typed_expr::<Out>(result);
        cx.emit(air::Statement::Return(
            if matches!(result.ty, air::Type::Unit) {
                None
            } else {
                Some(result)
            },
        ));
        let body = cx.end_body();
        let signature = cx.definitions[definition];
        let guard = guard.map(|handle| cx.typed_expr::<bool>(handle));
        let method = cx.add_method(air::Method {
            signature,
            guard,
            body: air::MethodBody::Action(body),
        });
        cx.push_action(air::MethodRef::Local(method))
    });
    scope.active = false;
    handle
}

pub fn call_action<D: Domain, In: Shape, Out: Shape>(
    handle: ActionHandle,
    args: In::Values<D>,
) -> Out::Values<D> {
    let args = In::pack_values(args).handle();
    let result = with_context(|cx| {
        let method = cx.action_ref(handle);
        let signature = match method {
            air::MethodRef::Local(method) => method.signature,
            air::MethodRef::Interface { signature, .. } => signature,
        };
        let ty = cx.ty::<Out>();
        assert!(
            matches!(signature.kind, air::MethodKind::Action(output) if ptr::eq(output, ty)),
            "action result type mismatch"
        );
        let input = cx.typed_expr::<In>(args);
        let arguments = split_input(cx, input);
        assert_eq!(
            signature.inputs.len(),
            arguments.len(),
            "action argument count mismatch"
        );
        for (parameter, argument) in signature.inputs.iter().zip(&arguments) {
            assert!(
                ptr::eq(parameter.ty, argument.ty),
                "action argument type mismatch"
            );
        }
        let args = cx.alloc_slice(arguments);
        if matches!(ty, air::Type::Unit) {
            cx.emit(air::Statement::ActionCall {
                method,
                args,
                result: None,
            });
            let unit = cx.unit();
            cx.push_expr(unit)
        } else {
            let name = cx.alloc_str(&format!("call_result_{}", cx.expressions.len()));
            let local = cx.alloc(air::Local { name, ty });
            cx.emit(air::Statement::ActionCall {
                method,
                args,
                result: Some(local),
            });
            let expr = cx.alloc(air::Expr {
                ty,
                kind: air::ExprKind::Local(local),
            });
            cx.push_expr(expr)
        }
    });
    Out::make_values(Value::from_handle(result))
}

pub fn pack_input<In: Shape>(args: &[super::ExprHandle]) -> super::ExprHandle {
    with_context(|cx| {
        let ty = cx.ty::<In>();
        let exprs: Vec<_> = args.iter().map(|handle| cx.expr(*handle)).collect();
        let kind = match ty {
            air::Type::Unit => air::ExprKind::Unit,
            air::Type::Tuple(_) => air::ExprKind::Tuple(cx.alloc_slice(exprs)),
            _ => exprs[0].kind,
        };
        let expr = cx.alloc(air::Expr { ty, kind });
        cx.push_expr(expr)
    })
}

fn split_input<'dsn>(
    cx: &Context<'dsn>,
    input: &'dsn air::Expr<'dsn>,
) -> Vec<&'dsn air::Expr<'dsn>> {
    match input.ty {
        air::Type::Unit => Vec::new(),
        air::Type::Tuple(elements) => elements
            .iter()
            .enumerate()
            .map(|(index, ty)| {
                cx.alloc(air::Expr {
                    ty,
                    kind: air::ExprKind::TupleElement {
                        value: input,
                        index,
                    },
                })
            })
            .collect(),
        _ => vec![input],
    }
}

pub fn capture_branch<D: Domain, R: IntoValues<D>>(f: impl FnOnce() -> R) -> BranchHandle {
    let mut scope = BodyScope::begin();
    let result = R::Out::pack_values::<D>(f().into_values()).handle();
    let handle = with_context(|cx| {
        let result = cx.typed_expr::<R::Out>(result);
        let block = cx.end_body();
        let handle = BranchHandle {
            design_id: cx.design_id,
            index: cx.branches.len(),
        };
        cx.branches.push(Branch {
            module: cx.id,
            parent_scope: cx.current_scope(),
            block,
            result,
        });
        handle
    });
    scope.active = false;
    handle
}

#[must_use]
pub fn finish_if<D: Domain, T: Shape>(
    cases: Vec<(BranchHandle, BranchHandle)>,
    fallback: Option<BranchHandle>,
) -> Value<D, T> {
    Value::from_handle(with_context(|cx| {
        assert!(!cases.is_empty(), "empty hardware conditional");
        let ty = cx.ty::<T>();
        let is_unit = matches!(ty, air::Type::Unit);
        let joined = if fallback.is_some() && !is_unit {
            let name = cx.alloc_str(&format!("if_result_{}", cx.expressions.len()));
            let local = cx.alloc(air::Local { name, ty });
            let value = cx.alloc(air::Expr {
                ty,
                kind: air::ExprKind::Undefined,
            });
            cx.emit(air::Statement::Let { local, value });
            Some(local)
        } else {
            None
        };
        let mut tail = fallback.map(|handle| branch_block(cx, handle, joined));
        for (condition, branch) in cases.into_iter().rev() {
            let condition_branch = cx.branch(condition);
            let condition = condition_branch.result;
            let mut prefix = condition_branch.block.statements.to_vec();
            assert!(
                ptr::eq(condition.ty, cx.ty::<bool>()),
                "conditional requires a boolean value"
            );
            let then_body = branch_block(cx, branch, joined);
            let statement = air::Statement::If {
                condition,
                then_body,
                else_body: tail,
            };
            prefix.push(statement);
            tail = Some(air::Block {
                statements: cx.alloc_slice(prefix),
            });
        }
        for statement in tail.expect("missing conditional body").statements {
            cx.emit(*statement);
        }
        let kind = match joined {
            Some(local) => air::ExprKind::Local(local),
            None if is_unit => air::ExprKind::Unit,
            None => air::ExprKind::Undefined,
        };
        let result = cx.alloc(air::Expr { ty, kind });
        cx.push_expr(result)
    }))
}

fn branch_block<'dsn>(
    cx: &Context<'dsn>,
    handle: BranchHandle,
    joined: Option<&'dsn air::Local<'dsn>>,
) -> air::Block<'dsn> {
    let branch = cx.branch(handle);
    if let Some(local) = joined {
        assert!(
            ptr::eq(local.ty, branch.result.ty),
            "conditional branch result type mismatch"
        );
        let mut statements = branch.block.statements.to_vec();
        statements.push(air::Statement::Assign {
            local,
            value: branch.result,
        });
        air::Block {
            statements: cx.alloc_slice(statements),
        }
    } else {
        branch.block
    }
}
