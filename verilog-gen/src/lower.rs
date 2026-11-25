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

fn source_attribute(
    origin: Origin,
    origins: &OriginMap,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    let span = origins
        .get(origin)
        .expect("origin is missing from the supplied OriginMap");
    write!(f, "(* src = \"")?;
    for ch in span.file.chars() {
        match ch {
            '\\' => write!(f, "\\\\")?,
            '"' => write!(f, "\\\"")?,
            '\n' => write!(f, "\\n")?,
            '\r' => write!(f, "\\r")?,
            '\t' => write!(f, "\\t")?,
            ch => write!(f, "{ch}")?,
        }
    }
    write!(
        f,
        ":{}.{}-{}.{}\" *) ",
        span.start.line, span.start.column, span.end.line, span.end.column
    )
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
