use crate::{
    expr::{Value, IntoValue},
    ir::{self, DesignId, ValueId, WriteDomain, runtime},
    value::Shape,
};

#[track_caller]
pub fn if_<R: IntoValue>(cond: impl IntoValue<Out = bool>, then: impl FnOnce() -> R) -> If<R::Out> {
    let cond = cond.into_value().imp;
    let (owner, parent, domain) = runtime::parent();
    let value = {
        let _scope = runtime::condition(cond.id);
        then().into_value().imp
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
    branches: Vec<(ValueId, ir::Value<T>)>,
}

impl<T: Shape> If<T> {
    #[track_caller]
    pub fn else_if<R: IntoValue<Out = T>>(
        mut self,
        cond: impl IntoValue<Out = bool>,
        then: impl FnOnce() -> R,
    ) -> Self {
        runtime::check_parent(self.owner, self.parent, self.domain);
        let cond = cond.into_value().imp;
        let active = runtime::boolean(crate::ir::BinaryOp::And, self.remaining, cond.id);
        let value = {
            let _scope = runtime::condition(active);
            then().into_value().imp
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
    pub fn else_<R: IntoValue<Out = T>>(self, otherwise: impl FnOnce() -> R) -> Value<T> {
        runtime::check_parent(self.owner, self.parent, self.domain);
        let mut result = {
            let _scope = runtime::condition(self.remaining);
            otherwise().into_value().imp
        };
        for (condition, value) in self.branches.into_iter().rev() {
            result = ir::Value::mux(condition, value, result);
        }
        Value { imp: result }
    }
}
