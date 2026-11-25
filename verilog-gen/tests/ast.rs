use astli_parse::SyntaxTree;
use verilog_gen::*;

struct Fixture {
    origins: OriginMap,
    origin: Origin,
}

impl Fixture {
    fn new() -> Self {
        let mut origins = OriginMap::new();
        let origin = origins.insert(Span {
            file: "test.rs".into(),
            start: Position { line: 1, column: 1 },
            end: Position { line: 2, column: 1 },
        });
        Self { origins, origin }
    }

    fn int(&self, value: u64) -> Expr {
        Expr::Lit(Lit::Int(LitInt {
            origin: self.origin,
            width: None,
            radix: Radix::Dec,
            value,
        }))
    }

    fn decl(&self, name: &str) -> Item {
        Item::Decl(Decl {
            origin: self.origin,
            kind: DeclKind::Variable,
            ty: Ty::LogicPacked(self.int(7), self.int(0)),
            name: Ident::new(name),
            dimensions: vec![],
            value: None,
        })
    }

    fn assignment(&self, target: Target, value: Expr, kind: AssignKind) -> Stmt {
        Stmt::Assign {
            assign: Assign {
                origin: self.origin,
                target,
                value,
            },
            kind,
        }
    }

    fn module(&self, items: Vec<Item>) -> Ast {
        Ast::Module(Module {
            origin: self.origin,
            name: Ident::new("top"),
            params: Default::default(),
            ports: Default::default(),
            items: items.into_iter().collect(),
        })
    }

    fn check(&self, ast: &Ast) -> String {
        let raw = ast.to_string();
        let raw_tree = SyntaxTree::parse("raw.sv", raw);
        assert!(
            raw_tree.diagnostics().is_empty(),
            "{:#?}",
            raw_tree.diagnostics()
        );
        assert!(raw_tree.unparsed().is_empty(), "{:#?}", raw_tree.unparsed());
        let output = lower(ast, &self.origins);
        let tree = SyntaxTree::parse("formatted.sv", output.clone());
        assert!(
            tree.diagnostics().is_empty(),
            "{:#?}\n{output}",
            tree.diagnostics()
        );
        assert!(tree.unparsed().is_empty(), "{:#?}", tree.unparsed());
        assert_eq!(
            output,
            astli_fmt::format(&tree, &Default::default()).unwrap()
        );
        assert!(output.contains("test.rs:1.1-2.1"));
        output
    }
}

fn name(value: &str) -> Expr {
    Expr::Name(Ident::new(value))
}
fn target(value: &str) -> Target {
    Target::Name(Ident::new(value))
}

#[test]
fn declarations_assignments_and_instances() {
    let f = Fixture::new();
    let ast = f.module(vec![
        f.decl("a"),
        f.decl("b"),
        Item::LocalParam(Param {
            origin: f.origin,
            ty: Ty::IntUnsigned,
            name: Ident::new("WIDTH"),
            value: f.int(8),
        }),
        Item::Decl(Decl {
            origin: f.origin,
            kind: DeclKind::Variable,
            ty: Ty::LogicSignedPacked(f.int(7), f.int(0)),
            name: Ident::new("memory"),
            dimensions: vec![Range {
                msb: f.int(0),
                lsb: f.int(15),
            }],
            value: None,
        }),
        Item::Decl(Decl {
            origin: f.origin,
            kind: DeclKind::Wire,
            ty: Ty::LogicPacked(f.int(7), f.int(0)),
            name: Ident::new("y"),
            dimensions: vec![],
            value: None,
        }),
        Item::Assign(Assign {
            origin: f.origin,
            target: target("y"),
            value: name("a"),
        }),
        Item::Instance(Instance {
            origin: f.origin,
            module: Ident::new("child"),
            name: Ident::new("child_inst"),
            params: vec![NamedArg {
                name: Ident::new("WIDTH"),
                value: f.int(8),
            }],
            ports: vec![
                Connection {
                    name: Ident::new("a"),
                    value: Some(name("a")),
                },
                Connection {
                    name: Ident::new("unused"),
                    value: None,
                },
            ],
        }),
    ]);
    let output = f.check(&ast);
    assert!(output.contains("localparam int unsigned WIDTH"));
    assert!(output.contains("child_inst"));
    assert!(output.contains(".unused()"));
    assert!(output.contains("[0:15]"));
}

#[test]
fn procedural_control_flow_and_select_targets() {
    let f = Fixture::new();
    let nested = Stmt::If(If {
        origin: f.origin,
        condition: name("enable"),
        then_body: vec![f.assignment(target("q"), name("a"), AssignKind::Nonblocking)],
        else_body: vec![f.assignment(target("q"), name("b"), AssignKind::Nonblocking)],
    });
    let ast = f.module(vec![
        f.decl("clk"),
        f.decl("rst_n"),
        f.decl("enable"),
        f.decl("q"),
        f.decl("a"),
        f.decl("b"),
        f.decl("y"),
        Item::Process(Process {
            origin: f.origin,
            kind: ProcessKind::Clocked(vec![
                Event {
                    edge: Edge::Pos,
                    signal: name("clk"),
                },
                Event {
                    edge: Edge::Neg,
                    signal: name("rst_n"),
                },
            ]),
            body: vec![Stmt::If(If {
                origin: f.origin,
                condition: Expr::Unary {
                    op: UnaryOp::LogicalNot,
                    value: Box::new(name("rst_n")),
                },
                then_body: vec![f.assignment(
                    target("q"),
                    Expr::Lit(Lit::Fill(Fill::Zero)),
                    AssignKind::Nonblocking,
                )],
                else_body: vec![nested],
            })],
        }),
        Item::Process(Process {
            origin: f.origin,
            kind: ProcessKind::Comb,
            body: vec![
                f.assignment(
                    target("y"),
                    Expr::Lit(Lit::Fill(Fill::Zero)),
                    AssignKind::Blocking,
                ),
                Stmt::Case(Case {
                    origin: f.origin,
                    value: name("enable"),
                    arms: vec![CaseArm {
                        values: vec![f.int(0), f.int(1)],
                        body: vec![Stmt::Block(vec![f.assignment(
                            Target::Slice {
                                value: Box::new(target("y")),
                                msb: f.int(3),
                                lsb: f.int(0),
                            },
                            Expr::IndexedSlice {
                                value: Box::new(name("a")),
                                base: Box::new(f.int(0)),
                                width: Box::new(f.int(4)),
                                direction: SliceDirection::Up,
                            },
                            AssignKind::Blocking,
                        )])],
                    }],
                    default: Some(vec![f.assignment(
                        Target::Index {
                            value: Box::new(target("y")),
                            index: f.int(0),
                        },
                        Expr::Index {
                            value: Box::new(name("b")),
                            index: Box::new(f.int(7)),
                        },
                        AssignKind::Blocking,
                    )]),
                }),
            ],
        }),
    ]);
    let output = f.check(&ast);
    for text in [
        "always_ff",
        "posedge clk or negedge rst_n",
        "always_comb",
        "if",
        "else",
        "case",
        "default",
        "+:",
    ] {
        assert!(output.contains(text), "missing {text}: {output}");
    }
}

#[test]
fn concatenated_and_indexed_slice_targets() {
    let f = Fixture::new();
    let ast = f.module(vec![
        f.decl("a"),
        f.decl("b"),
        f.decl("y"),
        f.decl("z"),
        Item::Process(Process {
            origin: f.origin,
            kind: ProcessKind::Comb,
            body: vec![
                f.assignment(
                    Target::Concat(vec![target("y"), target("z")]),
                    Expr::Concat(vec![name("a"), name("b")]),
                    AssignKind::Blocking,
                ),
                f.assignment(
                    Target::IndexedSlice {
                        value: Box::new(target("y")),
                        base: f.int(7),
                        width: f.int(4),
                        direction: SliceDirection::Down,
                    },
                    Expr::Slice {
                        value: Box::new(name("b")),
                        msb: Box::new(f.int(3)),
                        lsb: Box::new(f.int(0)),
                    },
                    AssignKind::Blocking,
                ),
            ],
        }),
    ]);
    let output = f.check(&ast);
    assert!(output.contains("{y, z}"));
    assert!(output.contains("-:"));
}
