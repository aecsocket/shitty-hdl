use crate::{
    expr::{EqExt, Value, IntoValue},
    ir::BinaryOp,
};

impl<Lhs: IntoValue> EqExt for Lhs {
    #[track_caller]
    fn eq(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        Value {
            imp: self
                .into_value()
                .imp
                .compare(rhs.into_value().imp, BinaryOp::Eq),
        }
    }

    #[track_caller]
    fn ne(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        Value {
            imp: self
                .into_value()
                .imp
                .compare(rhs.into_value().imp, BinaryOp::Ne),
        }
    }
}
