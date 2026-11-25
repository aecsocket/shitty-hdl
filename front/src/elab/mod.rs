mod context;

use std::cell::RefCell;

use scoped_tls_hkt::scoped_thread_local;

pub use bumpalo::Bump;
pub use context::Context;

scoped_thread_local!(static CURRENT: for<'scope> &'scope (dyn ActiveContext + 'scope));

#[must_use]
pub fn elaborate(arena: &Bump, f: impl FnOnce()) -> air::Module<'_> {
    let context = RefCell::new(Context::new(arena));
    CURRENT.set(&context as &dyn ActiveContext, f);
    context.into_inner().finish()
}

pub fn with_context<R>(f: impl for<'dsn> FnOnce(&mut Context<'dsn>) -> R) -> R {
    assert!(
        CURRENT.is_set(),
        "HDL operations require an active elab::elaborate call"
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
