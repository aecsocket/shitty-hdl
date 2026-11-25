use crate::{
    expr::{EqExt, Expr, IntoExpr},
    ir::BinaryOp,
};

impl<Lhs: IntoExpr> EqExt for Lhs {
    #[track_caller]
    fn eq(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        Expr {
            imp: self
                .into_expr()
                .imp
                .compare(rhs.into_expr().imp, BinaryOp::Eq),
        }
    }

    #[track_caller]
    fn ne(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        Expr {
            imp: self
                .into_expr()
                .imp
                .compare(rhs.into_expr().imp, BinaryOp::Ne),
        }
    }
}
