mod body;
pub(crate) mod context;
mod design;
mod expr;
mod fsm;
pub(crate) mod handle;
mod interface;
pub(crate) mod io;
mod primitive;
mod state;
mod types;

pub use bumpalo::Bump;
use scoped_tls_hkt::scoped_thread_local;
use std::cell::RefCell;

pub(crate) use body::{
    call_action, capture_branch, define_action, define_rule, finish_if, pack_input, run_ready,
};
pub(crate) use context::Context;
pub(crate) use design::DesignId;
pub(crate) use expr::{
    assert, binary, cast, compare, display, finish, literal, shift, shift_resize, tuple,
    tuple_element, uint_from_u128, unary,
};
pub(crate) use fsm::{define_async_method, define_async_rule, lower_async_if, unit_value};
pub(crate) use handle::{
    ActionHandle, BranchHandle, ClockHandle, ExprHandle, InterfaceHandle, RegisterHandle,
};
pub(crate) use interface::{input_interface, interface_action, module, subinterface};
pub(crate) use primitive::bram;
pub(crate) use state::{clock_input, read_register, register, write_register};

scoped_thread_local!(static CURRENT: for<'scope> &'scope (dyn ActiveContext + 'scope));

#[must_use]
pub fn elaborate(storage: &Bump, f: impl FnOnce()) -> air::Module<'_> {
    let design_id = DesignId::new();
    let context = RefCell::new(Context::new(storage, design_id));
    CURRENT.set(&context as &dyn ActiveContext, f);
    context.into_inner().finish()
}

pub(crate) fn with_context<R>(f: impl for<'dsn> FnOnce(&mut Context<'dsn>) -> R) -> R {
    assert!(
        CURRENT.is_set(),
        "attempted to access elaboration-time function while not elaborating",
    );
    let mut visitor = Call {
        f: Some(f),
        result: None,
    };
    CURRENT.with(|context| context.access(&mut visitor));
    visitor
        .result
        .expect("elaboration context callback was not invoked")
}

trait Visitor {
    fn visit(&mut self, context: &mut Context<'_>);
}

trait ActiveContext {
    fn access(&self, visitor: &mut dyn Visitor);
}

impl ActiveContext for RefCell<Context<'_>> {
    fn access(&self, visitor: &mut dyn Visitor) {
        let mut context = self
            .try_borrow_mut()
            .expect("reentrant access to the elaboration context");
        visitor.visit(&mut context);
    }
}

struct Call<F, R> {
    f: Option<F>,
    result: Option<R>,
}

impl<F, R> Visitor for Call<F, R>
where
    F: for<'dsn> FnOnce(&mut Context<'dsn>) -> R,
{
    fn visit(&mut self, context: &mut Context<'_>) {
        let f = self
            .f
            .take()
            .expect("elaboration context callback was already invoked");
        self.result = Some(f(context));
    }
}
