use crate::expr::{Expr, IntoExpr};

pub trait EqExt: IntoExpr {
    fn eq(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;

    fn ne(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool>;
}

impl<Lhs: IntoExpr> EqExt for Lhs {
    fn eq(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        todo!();
    }

    fn ne(self, rhs: impl IntoExpr<Out = Self::Out>) -> Expr<bool> {
        todo!();
    }
}
