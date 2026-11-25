use astli_parse::SyntaxTree;
use verilog_gen::*;

fn name(value: &str) -> Expr {
    Expr::Name(Ident::new(value))
}

fn check(expr: Expr) {
    let source = format!(
        "module m(input logic [7:0] a, b, output logic [7:0] y); assign y = {expr}; endmodule"
    );
    let tree = SyntaxTree::parse("expr.sv", source.clone());
    assert!(
        tree.diagnostics().is_empty(),
        "{source}: {:#?}",
        tree.diagnostics()
    );
    assert!(
        tree.unparsed().is_empty(),
        "{source}: {:#?}",
        tree.unparsed()
    );
    let formatted = astli_fmt::format(&tree, &Default::default()).unwrap();
    let reparsed = SyntaxTree::parse("expr.sv", formatted.clone());
    assert!(reparsed.diagnostics().is_empty());
    assert_eq!(
        formatted,
        astli_fmt::format(&reparsed, &Default::default()).unwrap()
    );
}

#[test]
fn binary_operators() {
    for op in [
        BinaryOp::Add,
        BinaryOp::Sub,
        BinaryOp::Mul,
        BinaryOp::Div,
        BinaryOp::Rem,
        BinaryOp::Pow,
        BinaryOp::BitAnd,
        BinaryOp::BitOr,
        BinaryOp::BitXor,
        BinaryOp::BitXnor,
        BinaryOp::LogicalAnd,
        BinaryOp::LogicalOr,
        BinaryOp::Eq,
        BinaryOp::Ne,
        BinaryOp::CaseEq,
        BinaryOp::CaseNe,
        BinaryOp::Lt,
        BinaryOp::Le,
        BinaryOp::Gt,
        BinaryOp::Ge,
        BinaryOp::Shl,
        BinaryOp::Shr,
        BinaryOp::ArithmeticShl,
        BinaryOp::ArithmeticShr,
    ] {
        check(Expr::Binary {
            lhs: Box::new(name("a")),
            op,
            rhs: Box::new(name("b")),
        });
    }
}

#[test]
fn unary_operators_and_grouping() {
    for op in [
        UnaryOp::Plus,
        UnaryOp::Neg,
        UnaryOp::LogicalNot,
        UnaryOp::BitNot,
        UnaryOp::ReduceAnd,
        UnaryOp::ReduceNand,
        UnaryOp::ReduceOr,
        UnaryOp::ReduceNor,
        UnaryOp::ReduceXor,
        UnaryOp::ReduceXnor,
    ] {
        check(Expr::Unary {
            op,
            value: Box::new(name("a")),
        });
    }
    let nested = Expr::Binary {
        lhs: Box::new(name("a")),
        op: BinaryOp::Sub,
        rhs: Box::new(Expr::Binary {
            lhs: Box::new(name("a")),
            op: BinaryOp::Sub,
            rhs: Box::new(name("b")),
        }),
    };
    assert_eq!(nested.to_string(), "(a - (a - b))");
    check(nested);
}

#[test]
fn selections_concatenation_and_calls() {
    let zero = || Expr::Lit(Lit::Fill(Fill::Zero));
    let one = || Expr::Lit(Lit::Fill(Fill::One));
    check(Expr::Index {
        value: Box::new(name("a")),
        index: Box::new(zero()),
    });
    check(Expr::Slice {
        value: Box::new(name("a")),
        msb: Box::new(one()),
        lsb: Box::new(zero()),
    });
    for direction in [SliceDirection::Up, SliceDirection::Down] {
        check(Expr::IndexedSlice {
            value: Box::new(name("a")),
            base: Box::new(one()),
            width: Box::new(one()),
            direction,
        });
    }
    check(Expr::Concat(vec![name("a"), name("b")]));
    check(Expr::Repeat {
        count: Box::new(one()),
        value: Box::new(name("a")),
    });
    check(Expr::Ternary {
        condition: Box::new(name("a")),
        then_value: Box::new(name("a")),
        else_value: Box::new(name("b")),
    });
    check(Expr::Call {
        function: Ident::new("f"),
        args: vec![name("a"), name("b")],
    });
    check(Expr::Clog2(Box::new(name("a"))));
}
