use verilog_gen::*;

fn int(origin: Origin, value: u64) -> Expr {
    Expr::Lit(Lit::Int(LitInt {
        origin,
        width: None,
        radix: Radix::Dec,
        value,
    }))
}

fn name(value: &str) -> Expr {
    Expr::Name(Ident::new(value))
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
    let vector = || {
        Ty::LogicPacked(
            Expr::Binary {
                lhs: Box::new(name("WIDTH")),
                op: BinaryOp::Sub,
                rhs: Box::new(int(origin, 1)),
            },
            int(origin, 0),
        )
    };
    let port = |name, flow, ty| Port {
        origin,
        name: Ident::new(name),
        flow,
        ty,
    };
    let update = Stmt::Assign {
        assign: Assign {
            origin,
            target: Target::Name(Ident::new("count")),
            value: name("next_count"),
        },
        kind: AssignKind::Nonblocking,
    };
    let ast = Ast::Module(Module {
        origin,
        name: Ident::new("counter"),
        params: [Param {
            origin,
            ty: Ty::IntUnsigned,
            name: Ident::new("WIDTH"),
            value: int(origin, 8),
        }]
        .into_iter()
        .collect(),
        ports: [
            port("clk", Flow::Input, Ty::Logic),
            port("reset", Flow::Input, Ty::Logic),
            port("enable", Flow::Input, Ty::Logic),
            port("count", Flow::Output, vector()),
            port("overflow", Flow::Output, Ty::Logic),
        ]
        .into_iter()
        .collect(),
        items: [
            Item::Decl(Decl {
                origin,
                kind: DeclKind::Wire,
                ty: vector(),
                name: Ident::new("next_count"),
                dimensions: vec![],
                value: None,
            }),
            Item::Assign(Assign {
                origin,
                target: Target::Name(Ident::new("next_count")),
                value: Expr::Binary {
                    lhs: Box::new(name("count")),
                    op: BinaryOp::Add,
                    rhs: Box::new(Expr::Lit(Lit::Int(LitInt {
                        origin,
                        width: 1.try_into().ok(),
                        radix: Radix::Bin,
                        value: 1,
                    }))),
                },
            }),
            Item::Assign(Assign {
                origin,
                target: Target::Name(Ident::new("overflow")),
                value: Expr::Binary {
                    lhs: Box::new(name("enable")),
                    op: BinaryOp::LogicalAnd,
                    rhs: Box::new(Expr::Unary {
                        op: UnaryOp::ReduceAnd,
                        value: Box::new(name("count")),
                    }),
                },
            }),
            Item::Process(Process {
                origin,
                kind: ProcessKind::Clocked(vec![Event {
                    edge: Edge::Pos,
                    signal: name("clk"),
                }]),
                body: vec![Stmt::If(If {
                    origin,
                    condition: name("reset"),
                    then_body: vec![Stmt::Assign {
                        assign: Assign {
                            origin,
                            target: Target::Name(Ident::new("count")),
                            value: Expr::Lit(Lit::Fill(Fill::Zero)),
                        },
                        kind: AssignKind::Nonblocking,
                    }],
                    else_body: vec![Stmt::If(If {
                        origin,
                        condition: name("enable"),
                        then_body: vec![update],
                        else_body: vec![],
                    })],
                })],
            }),
        ]
        .into_iter()
        .collect(),
    });
    print!("{}", verilog_gen::lower(&ast, &origins));
}
