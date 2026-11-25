use verilog_gen::{
    Ast, BinaryOp, Expr, Flow, Ident, Lit, LitInt, Module, Origin, Param, Port, Radix, Ty,
};

fn int(value: u64) -> Expr {
    Expr::Lit(Lit::Int(LitInt {
        origin: Origin(0),
        width: None,
        radix: Radix::Dec,
        value,
    }))
}

fn main() {
    let width = Ident::new("WIDTH");
    let ast = Ast::Module(Module {
        origin: Origin(0),
        name: Ident::new("counter"),
        params: [Param {
            origin: Origin(0),
            ty: Ty::IntUnsigned,
            name: width,
            value: int(8),
        }]
        .into_iter()
        .collect(),
        ports: [Port {
            origin: Origin(0),
            flow: Flow::Output,
            name: Ident::new("count"),
            ty: Ty::LogicPacked(
                Expr::Binary {
                    lhs: Box::new(Expr::Name(width)),
                    op: BinaryOp::Sub,
                    rhs: Box::new(int(1)),
                },
                int(0),
            ),
        }]
        .into_iter()
        .collect(),
    });
    print!("{}", verilog_gen::lower(&ast));
}
