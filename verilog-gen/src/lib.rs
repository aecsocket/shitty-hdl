mod ast;
mod ident;
mod lower;

pub use ast::*;
pub use ident::{BadIdent, Ident};

/// Emits the AST as formatted SystemVerilog.
///
/// # Panics
/// Panics if the emitted source cannot be parsed or the formatter refuses it.
#[must_use]
pub fn lower(ast: &Ast) -> String {
    let tree = astli_parse::SyntaxTree::parse("generated.sv", ast.to_string());
    assert!(
        tree.diagnostics().is_empty(),
        "generated invalid SystemVerilog: {:#?}",
        tree.diagnostics(),
    );
    astli_fmt::format(&tree, &astli_fmt::Options::default())
        .expect("formatting generated SystemVerilog should preserve its tokens")
}
