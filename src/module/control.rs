use crate::{
    expr::{Expr, IntoExpr},
    ir::{self, DesignId, ValueId, WriteDomain, runtime},
    value::Shape,
};

#[track_caller]
pub fn if_<R: IntoExpr>(cond: impl IntoExpr<Out = bool>, then: impl FnOnce() -> R) -> If<R::Out> {
    let cond = cond.into_expr().imp;
    let (owner, parent, domain) = runtime::parent();
    let value = {
        let _scope = runtime::condition(cond.id);
        then().into_expr().imp
    };
    If {
        owner,
        parent,
        domain,
        remaining: runtime::invert(cond.id),
        branches: vec![(cond.id, value)],
    }
}

pub struct If<T: Shape> {
    owner: DesignId,
    parent: Option<ValueId>,
    domain: WriteDomain,
    remaining: ValueId,
    branches: Vec<(ValueId, ir::Expr<T>)>,
}

impl<T: Shape> If<T> {
    #[track_caller]
    pub fn else_if<R: IntoExpr<Out = T>>(
        mut self,
        cond: impl IntoExpr<Out = bool>,
        then: impl FnOnce() -> R,
    ) -> Self {
        runtime::check_parent(self.owner, self.parent, self.domain);
        let cond = cond.into_expr().imp;
        let active = runtime::boolean(crate::ir::BinaryOp::And, self.remaining, cond.id);
        let value = {
            let _scope = runtime::condition(active);
            then().into_expr().imp
        };
        self.branches.push((cond.id, value));
        self.remaining = runtime::boolean(
            crate::ir::BinaryOp::And,
            self.remaining,
            runtime::invert(cond.id),
        );
        self
    }

    #[track_caller]
    pub fn else_<R: IntoExpr<Out = T>>(self, otherwise: impl FnOnce() -> R) -> Expr<T> {
        runtime::check_parent(self.owner, self.parent, self.domain);
        let mut result = {
            let _scope = runtime::condition(self.remaining);
            otherwise().into_expr().imp
        };
        for (condition, value) in self.branches.into_iter().rev() {
            result = ir::Expr::mux(condition, value, result);
        }
        Expr { imp: result }
    }
}
