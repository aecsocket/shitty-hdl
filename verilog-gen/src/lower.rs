use std::fmt;

use crate::ast::*;
use crate::{Origin, OriginMap};

pub(crate) struct WithOrigins<'a> {
    pub ast: &'a Ast,
    pub origins: &'a OriginMap,
}

impl fmt::Display for WithOrigins<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ast {
            Ast::Module(module) => format_module(module, Some(self.origins), f),
        }
    }
}

fn source_location(origin: Origin, origins: &OriginMap, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let span = origins
        .get(origin)
        .expect("origin is missing from the supplied OriginMap");
    write!(f, "\"")?;
    // Source locations also appear inside block comments. An octal escape
    // preserves '*' in attributes without allowing a path to terminate one.
    for (index, part) in span.file.split('*').enumerate() {
        if index != 0 {
            write!(f, "\\052")?;
        }
        escaped_string(part, f)?;
    }
    write!(
        f,
        ":{}.{}-{}.{}\"",
        span.start.line, span.start.column, span.end.line, span.end.column
    )
}

fn source_attribute(
    origin: Origin,
    origins: &OriginMap,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    write!(f, "(* src = ")?;
    source_location(origin, origins, f)?;
    write!(f, " *) ")
}

fn source_comment(origin: Origin, origins: &OriginMap, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "/* src = ")?;
    source_location(origin, origins, f)?;
    write!(f, " */ ")
}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Module(module) => write!(f, "{}", module),
        }
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_module(self, None, f)
    }
}

fn format_module(
    module: &Module,
    origins: Option<&OriginMap>,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if let Some(origins) = origins {
        source_attribute(module.origin, origins, f)?;
        writeln!(f)?;
    }
    write!(f, "module ")?;
    write!(f, "{}", module.name)?;
    if !module.params.is_empty() {
        writeln!(f, " #(")?;
        for (index, param) in module.params.iter().enumerate() {
            write!(f, "    ")?;
            if let Some(origins) = origins {
                source_attribute(param.origin, origins, f)?;
                writeln!(f)?;
                write!(f, "    ")?;
            }
            write!(f, "{}", param)?;
            writeln!(
                f,
                "{}",
                if index + 1 == module.params.len() {
                    ""
                } else {
                    ","
                }
            )?;
        }
        write!(f, ")")?;
    }
    if module.ports.is_empty() {
        writeln!(f, " ();")?;
    } else {
        writeln!(f, " (")?;
        for (index, port) in module.ports.iter().enumerate() {
            write!(f, "    ")?;
            if let Some(origins) = origins {
                source_attribute(port.origin, origins, f)?;
                writeln!(f)?;
                write!(f, "    ")?;
            }
            write!(f, "{}", port)?;
            writeln!(
                f,
                "{}",
                if index + 1 == module.ports.len() {
                    ""
                } else {
                    ","
                }
            )?;
        }
        writeln!(f, ");")?;
    }
    for item in &module.items {
        indent(1, f)?;
        format_item(item, origins, 1, f)?;
        writeln!(f)?;
    }
    writeln!(f, "endmodule")
}

impl fmt::Display for LitInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.width.is_none() && self.radix == Radix::Dec {
            return write!(f, "{}", self.value);
        }
        if let Some(width) = self.width {
            write!(f, "{width}")?;
        }
        write!(f, "'{}", self.radix)?;
        match self.radix {
            Radix::Dec => write!(f, "{}", self.value),
            Radix::Bin => write!(f, "{:b}", self.value),
            Radix::Hex => write!(f, "{:x}", self.value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ident;
    use smallvec::SmallVec;
    use std::num::NonZeroU16;

    fn int(value: u64) -> Expr {
        Expr::Lit(Lit::Int(LitInt {
            origin: Origin(0),
            width: None,
            radix: Radix::Dec,
            value,
        }))
    }

    #[test]
    fn lowers_parameterized_module() {
        let name = Ident::new("counter");
        let width = Ident::new("WIDTH");
        let count = Ident::new("count");
        let ast = Ast::Module(Module {
            origin: Origin(0),
            name,
            params: [Param {
                origin: Origin(1),
                ty: Ty::IntUnsigned,
                name: width,
                value: int(8),
            }]
            .into_iter()
            .collect(),
            ports: [Port {
                origin: Origin(2),
                flow: Flow::Output,
                name: count,
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
            items: Default::default(),
        });
        let output = ast.to_string();
        assert_eq!(
            output,
            concat!(
                "module counter #(\n",
                "    parameter int unsigned WIDTH = 8\n",
                ") (\n",
                "    output logic [(WIDTH - 1):0] count\n",
                ");\nendmodule\n",
            )
        );
    }

    #[test]
    fn lowers_empty_module() {
        let module = Module {
            origin: Origin(0),
            name: Ident::new("module"),
            params: SmallVec::new(),
            ports: SmallVec::new(),
            items: Default::default(),
        };
        let output = module.to_string();
        assert_eq!(output, "module \\module  ();\nendmodule\n");
    }

    #[test]
    fn literals_use_radix_digits() {
        for (radix, expected) in [
            (Radix::Dec, "8'd42"),
            (Radix::Bin, "8'b101010"),
            (Radix::Hex, "8'h2a"),
        ] {
            let lit = LitInt {
                origin: Origin(0),
                width: NonZeroU16::new(8),
                radix,
                value: 42,
            };
            let output = lit.to_string();
            assert_eq!(output, expected);
            let unsized_lit = LitInt { width: None, ..lit };
            let expected = match radix {
                Radix::Dec => "42",
                Radix::Bin => "'b101010",
                Radix::Hex => "'h2a",
            };
            assert_eq!(unsized_lit.to_string(), expected);
            for text in [output, unsized_lit.to_string()] {
                let tree = astli_parse::SyntaxTree::parse(
                    "literal.sv",
                    format!("module m; localparam X = {text}; endmodule"),
                );
                assert!(tree.diagnostics().is_empty(), "{:#?}", tree.diagnostics());
                astli_fmt::format(&tree, &astli_fmt::Options::default()).unwrap();
            }
        }
    }
}

fn separated<T: fmt::Display>(
    values: &[T],
    separator: &str,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    for (i, value) in values.iter().enumerate() {
        if i != 0 {
            write!(f, "{separator}")?;
        }
        write!(f, "{value}")?;
    }
    Ok(())
}

fn concat<T: fmt::Display>(values: &[T], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if values.is_empty() {
        return Err(fmt::Error);
    }
    write!(f, "{{")?;
    separated(values, ", ", f)?;
    write!(f, "}}")
}

fn selection_base(value: &Expr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Parenthesizing an arbitrary expression does not make it a legal SV select base.
    match value {
        Expr::Name(_)
        | Expr::Index { .. }
        | Expr::Slice { .. }
        | Expr::IndexedSlice { .. }
        | Expr::Concat(_) => write!(f, "{value}"),
        _ => Err(fmt::Error),
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lit(x) => write!(f, "{x}"),
            Self::Name(x) => write!(f, "{x}"),
            Self::Binary { lhs, op, rhs } => write!(f, "({lhs} {op} {rhs})"),
            Self::Unary { op, value } => write!(f, "({op}{value})"),
            Self::Ternary {
                condition,
                then_value,
                else_value,
            } => write!(f, "({condition} ? {then_value} : {else_value})"),
            Self::Index { value, index } => {
                selection_base(value, f)?;
                write!(f, "[{index}]")
            }
            Self::Slice { value, msb, lsb } => {
                selection_base(value, f)?;
                write!(f, "[{msb}:{lsb}]")
            }
            Self::IndexedSlice {
                value,
                base,
                width,
                direction,
            } => {
                selection_base(value, f)?;
                write!(f, "[{base} {direction} {width}]")
            }
            Self::Concat(values) => concat(values, f),
            Self::Repeat { count, value } => write!(f, "{{{count}{{{value}}}}}"),
            Self::Call { function, args } => {
                write!(f, "{function}(")?;
                separated(args, ", ", f)?;
                write!(f, ")")
            }
            Self::Clog2(value) => write!(f, "$clog2({value})"),
        }
    }
}

fn escaped_string(value: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for ch in value.chars() {
        match ch {
            '\\' => write!(f, "\\\\")?,
            '"' => write!(f, "\\\"")?,
            '\n' => write!(f, "\\n")?,
            '\r' => write!(f, "\\015")?,
            '\t' => write!(f, "\\t")?,
            ch if ch.is_control() => {
                let mut bytes = [0; 4];
                for byte in ch.encode_utf8(&mut bytes).bytes() {
                    write!(f, "\\{byte:03o}")?;
                }
            }
            ch => write!(f, "{ch}")?,
        }
    }
    Ok(())
}

impl fmt::Display for Lit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(x) => write!(f, "{x}"),
            Self::Fill(x) => write!(f, "{x}"),
            Self::Str(x) => {
                write!(f, "\"")?;
                escaped_string(x, f)?;
                write!(f, "\"")
            }
        }
    }
}

macro_rules! display_tokens {
    ($ty:ty, $($variant:ident => $text:literal),+ $(,)?) => {
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(match self { $(Self::$variant => $text),+ })
            }
        }
    };
}
display_tokens!(Fill, Zero => "'0", One => "'1", X => "'x", Z => "'z");
display_tokens!(SliceDirection, Up => "+:", Down => "-:");
display_tokens!(Edge, Pos => "posedge", Neg => "negedge");
display_tokens!(AssignKind, Blocking => "=", Nonblocking => "<=");
display_tokens!(DeclKind, Variable => "", Wire => "wire");

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}]", self.msb, self.lsb)
    }
}
impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(x) => write!(f, "{x}"),
            Self::Index { value, index } => write!(f, "{value}[{index}]"),
            Self::Slice { value, msb, lsb } => write!(f, "{value}[{msb}:{lsb}]"),
            Self::IndexedSlice {
                value,
                base,
                width,
                direction,
            } => write!(f, "{value}[{base} {direction} {width}]"),
            Self::Concat(values) => concat(values, f),
        }
    }
}
impl fmt::Display for Decl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.kind == DeclKind::Wire {
            write!(f, "wire ")?;
        }
        write!(f, "{} {}", self.ty, self.name)?;
        for dimension in &self.dimensions {
            write!(f, " {dimension}")?;
        }
        if let Some(value) = &self.value {
            write!(f, " = {value}")?;
        }
        write!(f, ";")
    }
}
impl fmt::Display for Assign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "assign {} = {};", self.target, self.value)
    }
}
impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.edge, self.signal)
    }
}
impl fmt::Display for ProcessKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Comb => write!(f, "always_comb"),
            Self::Clocked(events) => {
                if events.is_empty() {
                    return Err(fmt::Error);
                }
                write!(f, "always_ff @(")?;
                separated(events, " or ", f)?;
                write!(f, ")")
            }
        }
    }
}
impl fmt::Display for NamedArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ".{}({})", self.name, self.value)
    }
}
impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ".{}(", self.name)?;
        if let Some(value) = &self.value {
            write!(f, "{value}")?;
        }
        write!(f, ")")
    }
}
impl fmt::Display for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.module)?;
        if !self.params.is_empty() {
            write!(f, "#(")?;
            separated(&self.params, ", ", f)?;
            write!(f, ") ")?;
        }
        write!(f, "{} (", self.name)?;
        separated(&self.ports, ", ", f)?;
        write!(f, ");")
    }
}

fn attribute(
    origin: Origin,
    origins: Option<&OriginMap>,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if let Some(origins) = origins {
        source_attribute(origin, origins, f)?;
    }
    Ok(())
}
fn indent(depth: usize, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for _ in 0..depth {
        write!(f, "  ")?;
    }
    Ok(())
}

fn block(
    body: &[Stmt],
    origins: Option<&OriginMap>,
    depth: usize,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    writeln!(f, "begin")?;
    for stmt in body {
        indent(depth + 1, f)?;
        format_stmt(stmt, origins, depth + 1, f)?;
        writeln!(f)?;
    }
    indent(depth, f)?;
    write!(f, "end")
}
fn format_if(
    value: &If,
    origins: Option<&OriginMap>,
    depth: usize,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    attribute(value.origin, origins, f)?;
    write!(f, "if ({}) ", value.condition)?;
    block(&value.then_body, origins, depth, f)?;
    if !value.else_body.is_empty() {
        write!(f, " else ")?;
        block(&value.else_body, origins, depth, f)?;
    }
    Ok(())
}
fn format_arm(
    arm: &CaseArm,
    origins: Option<&OriginMap>,
    depth: usize,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if arm.values.is_empty() {
        return Err(fmt::Error);
    }
    separated(&arm.values, ", ", f)?;
    write!(f, ": ")?;
    block(&arm.body, origins, depth, f)
}
fn format_case(
    value: &Case,
    origins: Option<&OriginMap>,
    depth: usize,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    attribute(value.origin, origins, f)?;
    writeln!(f, "case ({})", value.value)?;
    for arm in &value.arms {
        indent(depth + 1, f)?;
        format_arm(arm, origins, depth + 1, f)?;
        writeln!(f)?;
    }
    if let Some(body) = &value.default {
        indent(depth + 1, f)?;
        write!(f, "default: ")?;
        block(body, origins, depth + 1, f)?;
        writeln!(f)?;
    }
    indent(depth, f)?;
    write!(f, "endcase")
}
fn format_stmt(
    stmt: &Stmt,
    origins: Option<&OriginMap>,
    depth: usize,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match stmt {
        Stmt::Assign { assign, kind } => {
            attribute(assign.origin, origins, f)?;
            write!(f, "{} {kind} {};", assign.target, assign.value)
        }
        Stmt::If(value) => format_if(value, origins, depth, f),
        Stmt::Block(body) => block(body, origins, depth, f),
        Stmt::Case(value) => format_case(value, origins, depth, f),
    }
}
fn format_process(
    value: &Process,
    origins: Option<&OriginMap>,
    depth: usize,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    attribute(value.origin, origins, f)?;
    write!(f, "{} ", value.kind)?;
    block(&value.body, origins, depth, f)
}
fn format_item(
    item: &Item,
    origins: Option<&OriginMap>,
    depth: usize,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match item {
        Item::Decl(value) => {
            attribute(value.origin, origins, f)?;
            write!(f, "{value}")
        }
        Item::Assign(value) => {
            // Yosys does not accept attributes before continuous assignments.
            if let Some(origins) = origins {
                source_comment(value.origin, origins, f)?;
            }
            write!(f, "{value}")
        }
        Item::Process(value) => format_process(value, origins, depth, f),
        Item::Instance(value) => {
            attribute(value.origin, origins, f)?;
            write!(f, "{value}")
        }
        Item::LocalParam(value) => {
            attribute(value.origin, origins, f)?;
            write!(
                f,
                "localparam {} {} = {};",
                value.ty, value.name, value.value
            )
        }
    }
}
macro_rules! display_lowered {
    ($ty:ty, $format:ident) => {
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                $format(self, None, 0, f)
            }
        }
    };
}
display_lowered!(Item, format_item);
display_lowered!(Stmt, format_stmt);
display_lowered!(Process, format_process);
display_lowered!(If, format_if);
display_lowered!(Case, format_case);
display_lowered!(CaseArm, format_arm);

#[cfg(test)]
mod core_tests {
    use super::*;
    use crate::Ident;

    fn name(value: &str) -> Expr {
        Expr::Name(Ident::new(value))
    }

    #[test]
    fn strings_and_selection_safeguards() {
        assert_eq!(
            Lit::Str("\"\\\n\r\t\0".into()).to_string(),
            "\"\\\"\\\\\\n\\015\\t\\000\""
        );
        let invalid = Expr::Index {
            value: Box::new(Expr::Binary {
                lhs: Box::new(name("a")),
                op: BinaryOp::Add,
                rhs: Box::new(name("b")),
            }),
            index: Box::new(name("i")),
        };
        use std::fmt::Write;
        assert!(write!(String::new(), "{invalid}").is_err());
        assert!(write!(String::new(), "{}", Expr::Concat(vec![])).is_err());
        assert!(write!(String::new(), "{}", ProcessKind::Clocked(vec![])).is_err());
    }

    #[test]
    fn lowers_core_process_and_instance() {
        let assign = Assign {
            origin: Origin(0),
            target: Target::Name(Ident::new("q")),
            value: name("d"),
        };
        let process = Process {
            origin: Origin(0),
            kind: ProcessKind::Clocked(vec![Event {
                edge: Edge::Pos,
                signal: name("clk"),
            }]),
            body: vec![Stmt::If(If {
                origin: Origin(0),
                condition: name("enable"),
                then_body: vec![Stmt::Assign {
                    assign,
                    kind: AssignKind::Nonblocking,
                }],
                else_body: vec![Stmt::Case(Case {
                    origin: Origin(0),
                    value: name("mode"),
                    arms: vec![CaseArm {
                        values: vec![Expr::Lit(Lit::Fill(Fill::Zero))],
                        body: vec![],
                    }],
                    default: Some(vec![]),
                })],
            })],
        };
        let instance = Instance {
            origin: Origin(0),
            module: Ident::new("child"),
            name: Ident::new("u_child"),
            params: vec![NamedArg {
                name: Ident::new("WIDTH"),
                value: Expr::Clog2(Box::new(name("DEPTH"))),
            }],
            ports: vec![Connection {
                name: Ident::new("unused"),
                value: None,
            }],
        };
        let text = format!(
            "module m(input logic clk, enable, d, mode, output logic q);\n{process}\n{instance}\nendmodule"
        );
        let tree = astli_parse::SyntaxTree::parse("core.sv", text);
        assert!(tree.diagnostics().is_empty(), "{:#?}", tree.diagnostics());
        astli_fmt::format(&tree, &astli_fmt::Options::default()).unwrap();
    }
}

#[cfg(test)]
mod raw_layout_tests {
    use super::*;
    use crate::{Ident, Position, Span};

    #[test]
    fn comments_and_nested_indentation() {
        let mut origins = OriginMap::new();
        let origin = origins.insert(Span {
            file: "source.sv".into(),
            start: Position { line: 1, column: 1 },
            end: Position { line: 1, column: 2 },
        });
        let assign = Assign {
            origin,
            target: Target::Name(Ident::new("q")),
            value: Expr::Name(Ident::new("d")),
        };
        let ast = Ast::Module(Module {
            origin,
            name: Ident::new("m"),
            params: Default::default(),
            ports: Default::default(),
            items: [
                Item::Assign(assign.clone()),
                Item::Process(Process {
                    origin,
                    kind: ProcessKind::Comb,
                    body: vec![Stmt::If(If {
                        origin,
                        condition: Expr::Name(Ident::new("enable")),
                        then_body: vec![Stmt::Assign {
                            assign,
                            kind: AssignKind::Blocking,
                        }],
                        else_body: vec![],
                    })],
                }),
            ]
            .into_iter()
            .collect(),
        });
        let text = WithOrigins {
            ast: &ast,
            origins: &origins,
        }
        .to_string();
        assert!(
            text.contains("\n  /* src = \"source.sv:1.1-1.2\" */ assign q = d;\n"),
            "{text}"
        );
        assert!(text.contains("\n    (* src = \"source.sv:1.1-1.2\" *) if (enable) begin\n      (* src = \"source.sv:1.1-1.2\" *) q = d;\n    end\n  end\n"), "{text}");
    }
}
