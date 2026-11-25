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

#[cfg(test)]
mod tests {
    use super::*;

    fn span(line: u32) -> Span {
        Span {
            file: "src/counter.rs".into(),
            start: Position { line, column: 1 },
            end: Position { line, column: 8 },
        }
    }

    #[test]
    fn lower_emits_source_attributes() {
        let mut origins = OriginMap::new();
        let module_origin = origins.insert(span(1));
        let param_origin = origins.insert(span(2));
        let port_origin = origins.insert(span(3));
        let ast = Ast::Module(Module {
            origin: module_origin,
            name: Ident::new("counter"),
            params: [Param {
                origin: param_origin,
                ty: Ty::IntUnsigned,
                name: Ident::new("WIDTH"),
                value: Expr::Lit(Lit::Int(LitInt {
                    origin: param_origin,
                    width: None,
                    radix: Radix::Dec,
                    value: 8,
                })),
            }]
            .into_iter()
            .collect(),
            ports: [Port {
                origin: port_origin,
                flow: Flow::Input,
                ty: Ty::Logic,
                name: Ident::new("clk"),
            }]
            .into_iter()
            .collect(),
        });
        let output = lower(&ast, &origins);
        for line in 1..=3 {
            assert!(output.contains(&format!("src/counter.rs:{line}.1-{line}.8")));
        }
        assert!(!ast.to_string().contains("src ="));
        let tree = astli_parse::SyntaxTree::parse("generated.sv", output.clone());
        assert!(tree.diagnostics().is_empty());
        assert_eq!(
            output,
            astli_fmt::format(&tree, &Default::default()).unwrap()
        );
    }

    #[test]
    #[should_panic(expected = "origin is missing")]
    fn missing_origin_is_rejected() {
        let mut origins = OriginMap::new();
        let origin = origins.insert(span(1));
        let ast = Ast::Module(Module {
            origin,
            name: Ident::new("counter"),
            params: Default::default(),
            ports: Default::default(),
        });
        lower(&ast, &OriginMap::new());
    }
}
