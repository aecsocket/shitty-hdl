mod ast;
mod ident;
mod lower;
pub mod origin;

pub use ast::*;
pub use ident::{BadIdent, Ident};
pub use origin::{Origin, OriginMap, Position, Span};

/// Emits the AST as formatted SystemVerilog with source attributes.
///
/// # Panics
/// Panics if an annotated origin is missing from the map, the emitted source
/// cannot be parsed, or the formatter refuses it.
#[must_use]
pub fn lower(ast: &Ast, origins: &OriginMap) -> String {
    let source = lower::WithOrigins { ast, origins }.to_string();
    let tree = astli_parse::SyntaxTree::parse("generated.sv", source);
    assert!(
        tree.diagnostics().is_empty(),
        "generated invalid SystemVerilog: {:#?}",
        tree.diagnostics(),
    );
    astli_fmt::format(&tree, &astli_fmt::Options::default())
        .expect("formatting generated SystemVerilog should preserve its tokens")
}
