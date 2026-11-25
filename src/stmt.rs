pub struct Stmt {}

pub trait IntoStmts {
    fn into_stmts(self) -> impl Iterator<Item = Stmt>;
}

impl IntoStmts for Stmt {
    fn into_stmts(self) -> impl Iterator<Item = Stmt> {
        std::iter::once(self)
    }
}

impl<const N: usize> IntoStmts for [Stmt; N] {
    fn into_stmts(self) -> impl Iterator<Item = Stmt> {
        self.into_iter()
    }
}
