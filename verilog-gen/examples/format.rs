use verilog_gen::{
    Ast, BinaryOp, Expr, Flow, Ident, Lit, LitInt, Module, Origin, OriginMap, Param, Port,
    Position, Radix, Span, Ty,
};

fn int(origin: Origin, value: u64) -> Expr {
    Expr::Lit(Lit::Int(LitInt {
        origin,
        width: None,
        radix: Radix::Dec,
        value,
    }))
}

fn main() {
    let mut origins = OriginMap::new();
    let origin = origins.insert(Span {
        file: "examples/counter.rs".into(),
        start: Position { line: 1, column: 1 },
        end: Position {
            line: 10,
            column: 2,
        },
    });
    let width = Ident::new("WIDTH");
    let ast = Ast::Module(Module {
        origin,
        name: Ident::new("counter"),
        params: [Param {
            origin,
            ty: Ty::IntUnsigned,
            name: width,
            value: int(origin, 8),
        }]
        .into_iter()
        .collect(),
        ports: [Port {
            origin,
            flow: Flow::Output,
            name: Ident::new("count"),
            ty: Ty::LogicPacked(
                Expr::Binary {
                    lhs: Box::new(Expr::Name(width)),
                    op: BinaryOp::Sub,
                    rhs: Box::new(int(origin, 1)),
                },
                int(origin, 0),
            ),
        }]
        .into_iter()
        .collect(),
    });
    print!("{}", verilog_gen::lower(&ast, &origins));
}
