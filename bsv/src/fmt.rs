use crate::{
    Binding, ClockBinding, Declaration, Expr, ExprKind, Field, FieldValue, Instance, Interface,
    InterfaceExpr, InterfaceMember, LocalInterface, Method, MethodBody, MethodSignature, Module,
    Package, Parameter, Register, ResetBinding, Rule, Schedule, Statement, Type, Typedef,
    TypedefKind, Variant,
};
use std::fmt::{self, Write};

pub trait WriteBsv {
    #[allow(clippy::missing_errors_doc)]
    fn write_bsv(&self, out: &mut dyn Write) -> fmt::Result;
}

#[allow(clippy::missing_errors_doc)]
pub fn write<N: WriteBsv + ?Sized>(out: &mut dyn Write, node: &N) -> fmt::Result {
    node.write_bsv(out)
}

pub struct DisplayBsv<'a, N: ?Sized>(pub &'a N);

impl<N: WriteBsv + ?Sized> fmt::Display for DisplayBsv<'_, N> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.write_bsv(out)
    }
}

struct Printer<'a> {
    out: &'a mut dyn Write,
    depth: usize,
    line_start: bool,
    column: usize,
    flat: bool,
}

impl Write for Printer<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for part in text.split_inclusive('\n') {
            if self.line_start && part != "\n" {
                for _ in 0..self.depth {
                    self.out.write_str("    ")?;
                }
                self.column = self.depth * 4;
            }
            self.out.write_str(part)?;
            self.line_start = part.ends_with('\n');
            if self.line_start {
                self.column = 0;
            } else {
                self.column += part.chars().count();
            }
        }
        Ok(())
    }
}

trait Emit {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result;
}

impl Printer<'_> {
    fn node<N: Emit + ?Sized>(&mut self, node: &N) -> fmt::Result {
        node.emit(self)
    }

    fn fits<N: Emit + ?Sized>(&self, node: &N, reserved: usize) -> Result<bool, fmt::Error> {
        if self.flat {
            return Ok(true);
        }
        let mut text = String::new();
        node.emit(&mut Printer {
            out: &mut text,
            depth: 0,
            line_start: true,
            column: 0,
            flat: true,
        })?;
        let column = if self.line_start {
            self.depth * 4
        } else {
            self.column
        };
        Ok(!text.contains('\n') && column + text.chars().count() + reserved <= 100)
    }

    fn list<N: Emit>(&mut self, nodes: &[N]) -> fmt::Result {
        if self.flat {
            for (i, node) in nodes.iter().enumerate() {
                if i != 0 {
                    self.write_str(", ")?;
                }
                self.node(node)?;
            }
            return Ok(());
        }
        self.nested(|p| {
            for (i, node) in nodes.iter().enumerate() {
                if i != 0 {
                    p.write_char(',')?;
                }
                if !p.fits(node, if i == 0 { 1 } else { 2 })? {
                    p.write_char('\n')?;
                } else if i != 0 {
                    p.write_char(' ')?;
                }
                p.node(node)?;
            }
            Ok(())
        })
    }

    fn nested(&mut self, f: impl FnOnce(&mut Self) -> fmt::Result) -> fmt::Result {
        self.depth += 1;
        let result = f(self);
        self.depth -= 1;
        result
    }

    fn quoted(&mut self, text: &str) -> fmt::Result {
        self.write_char('"')?;
        for c in text.chars() {
            match c {
                '"' => self.write_str("\\\"")?,
                '\\' => self.write_str("\\\\")?,
                '\n' => self.write_str("\\n")?,
                '\r' => self.write_str("\\r")?,
                '\t' => self.write_str("\\t")?,
                c if c.is_control() => {
                    let mut bytes = [0; 4];
                    for byte in c.encode_utf8(&mut bytes).bytes() {
                        write!(self, "\\{byte:03o}")?;
                    }
                }
                c => self.write_char(c)?,
            }
        }
        self.write_char('"')
    }

    fn action_block(&mut self, body: &[Statement<'_>]) -> fmt::Result {
        let body = match body.split_last() {
            Some((Statement::Return(None), rest)) => rest,
            _ => body,
        };
        self.node(body)
    }

    fn signature(&mut self, signature: &MethodSignature<'_>) -> fmt::Result {
        self.write_str("method ")?;
        self.node(&signature.output)?;
        write!(self, " {}(", signature.name)?;
        self.list(signature.inputs)?;
        self.write_char(')')
    }

    fn guard(&mut self, guard: Option<&Expr<'_>>) -> fmt::Result {
        if let Some(guard) = guard {
            self.write_str(" if (")?;
            self.node(guard)?;
            self.write_char(')')?;
        }
        Ok(())
    }

    fn constructor(
        &mut self,
        name: &str,
        arguments: &[Expr<'_>],
        clock: Option<&str>,
        reset: Option<&str>,
    ) -> fmt::Result {
        write!(self, "{name}(")?;
        self.list(arguments)?;
        if let Some(clock) = clock {
            if !arguments.is_empty() {
                self.write_str(", ")?;
            }
            write!(self, "clocked_by {clock}")?;
        }
        if let Some(reset) = reset {
            if !arguments.is_empty() || clock.is_some() {
                self.write_str(", ")?;
            }
            write!(self, "reset_by {reset}")?;
        }
        self.write_str(");\n")
    }

    fn bindings(&mut self, members: &[Binding<'_>]) -> fmt::Result {
        for (i, member) in members.iter().enumerate() {
            if i != 0 {
                self.write_char('\n')?;
            }
            self.node(member)?;
        }
        Ok(())
    }
}

macro_rules! standalone {
    ($($name:ident),* $(,)?) => {$ (
        impl WriteBsv for $name<'_> {
            fn write_bsv(&self, out: &mut dyn Write) -> fmt::Result {
                self.emit(&mut Printer { out, depth: 0, line_start: true, column: 0, flat: false })
            }
        }
        impl fmt::Display for $name<'_> {
            fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.write_bsv(out)
            }
        }
    )*};
}

standalone!(
    Type,
    Package,
    Field,
    Variant,
    Typedef,
    TypedefKind,
    Interface,
    InterfaceMember,
    MethodSignature,
    ClockBinding,
    ResetBinding,
    Module,
    Parameter,
    Declaration,
    Register,
    Instance,
    LocalInterface,
    InterfaceExpr,
    Binding,
    Method,
    Rule,
    Schedule,
    Expr,
    FieldValue,
    Statement,
);

impl WriteBsv for [Statement<'_>] {
    fn write_bsv(&self, out: &mut dyn Write) -> fmt::Result {
        self.emit(&mut Printer {
            out,
            depth: 0,
            line_start: true,
            column: 0,
            flat: false,
        })
    }
}

impl Emit for Type<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        match self {
            Self::Inferred => p.write_str("let"),
            Self::Void => p.write_str("void"),
            Self::Bool => p.write_str("Bool"),
            Self::Bit(n) => write!(p, "Bit#({n})"),
            Self::UInt(n) => write!(p, "UInt#({n})"),
            Self::Int(n) => write!(p, "Int#({n})"),
            Self::Named(name) => p.write_str(name),
            Self::Applied { name, arguments } => {
                write!(p, "{name}#(")?;
                p.list(arguments)?;
                p.write_char(')')
            }
            Self::Vector { length, element } => {
                write!(p, "Vector#({length}, ")?;
                p.node(*element)?;
                p.write_char(')')
            }
            Self::Action => p.write_str("Action"),
            Self::ActionValue(ty) => {
                p.write_str("ActionValue#(")?;
                p.node(*ty)?;
                p.write_char(')')
            }
        }
    }
}

impl Emit for Package<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        writeln!(p, "package {};", self.name)?;
        for import in self.imports {
            writeln!(p, "import {import}::*;")?;
        }
        for typedef in self.typedefs {
            p.write_char('\n')?;
            p.node(typedef)?;
        }
        for interface in self.interfaces {
            p.write_char('\n')?;
            p.node(interface)?;
        }
        for module in self.modules {
            p.write_char('\n')?;
            p.node(module)?;
        }
        p.write_str("\nendpackage\n")
    }
}

impl Emit for Field<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        p.node(&self.ty)?;
        write!(p, " {}", self.name)
    }
}

impl Emit for Variant<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        if let Some(ty) = self.payload {
            p.node(&ty)?;
        } else {
            p.write_str("void")?;
        }
        writeln!(p, " {};", self.name)
    }
}

impl Emit for TypedefKind<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        match self {
            Self::Struct(fields) => {
                p.write_str("struct {\n")?;
                p.nested(|p| {
                    for field in *fields {
                        p.node(field)?;
                        p.write_str(";\n")?;
                    }
                    Ok(())
                })?;
                p.write_char('}')
            }
            Self::Enum(variants) => write!(p, "enum {{ {} }}", variants.join(", ")),
            Self::TaggedUnion(variants) => {
                p.write_str("union tagged {\n")?;
                p.nested(|p| {
                    for variant in *variants {
                        p.node(variant)?;
                    }
                    Ok(())
                })?;
                p.write_char('}')
            }
            Self::Alias(ty) => p.node(ty),
        }
    }
}

impl Emit for Typedef<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        p.write_str("typedef ")?;
        p.node(&self.kind)?;
        write!(p, " {}", self.name)?;
        if !self.deriving.is_empty() {
            write!(p, " deriving ({})", self.deriving.join(", "))?;
        }
        p.write_str(";\n")
    }
}

impl Emit for Interface<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        writeln!(p, "interface {};", self.name)?;
        p.nested(|p| {
            for member in self.members {
                p.node(member)?;
            }
            Ok(())
        })?;
        p.write_str("endinterface\n")
    }
}

impl Emit for InterfaceMember<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        match self {
            Self::Method(signature) => p.node(signature),
            Self::Interface { name, ty } => writeln!(p, "interface {ty} {name};"),
        }
    }
}

impl Emit for MethodSignature<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        if self.always_ready || self.result_port.is_some() {
            p.write_str("(* ")?;
            if self.always_ready {
                p.write_str("always_ready")?;
            }
            if let Some(port) = self.result_port {
                if self.always_ready {
                    p.write_str(", ")?;
                }
                p.write_str("result = ")?;
                p.quoted(port)?;
            }
            p.write_str(" *)\n")?;
        }
        p.signature(self)?;
        p.write_str(";\n")
    }
}

impl Emit for Parameter<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        match self {
            Self::Value(field) => {
                p.write_str("(* port = ")?;
                p.quoted(field.name)?;
                p.write_str(", clocked_by = \"no_clock\", reset_by = \"no_reset\" *) ")?;
                p.node(field)
            }
            Self::Interface { name, ty } => write!(p, "{ty} {name}"),
            Self::Clock { name, port } => write!(p, "(* osc = \"{port}\" *) Clock {name}"),
        }
    }
}

impl Emit for ClockBinding<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        writeln!(p, "Clock {} <- {};", self.name, self.constructor)
    }
}

impl Emit for ResetBinding<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        writeln!(
            p,
            "Reset {} <- mkAsyncReset(2, bsv_default_reset, {});",
            self.name, self.clock
        )
    }
}

impl Emit for Module<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        for schedule in self.scheduling {
            p.node(schedule)?;
        }
        write!(p, "module {}(", self.name)?;
        if !self.parameters.is_empty() {
            p.list(self.parameters)?;
            p.write_str(", ")?;
        }
        writeln!(p, "{} bsv_result);", self.interface)?;
        p.nested(|p| {
            for clock in self.clock_bindings {
                p.node(clock)?;
                p.write_char('\n')?;
            }
            if !self.reset_bindings.is_empty() {
                p.write_str("Reset bsv_default_reset <- exposeCurrentReset;\n\n")?;
            }
            for reset in self.reset_bindings {
                p.node(reset)?;
                p.write_char('\n')?;
            }
            for declaration in self.declarations {
                p.node(declaration)?;
                p.write_char('\n')?;
            }
            for register in self.registers {
                p.node(register)?;
                p.write_char('\n')?;
            }
            for instance in self.instances {
                p.node(instance)?;
                p.write_char('\n')?;
            }
            for interface in self.local_interfaces {
                p.node(interface)?;
                p.write_char('\n')?;
            }
            for rule in self.rules {
                p.node(rule)?;
                p.write_char('\n')?;
            }
            p.write_str("return ")?;
            p.node(&self.implementation)?;
            p.write_str(";\n")
        })?;
        p.write_str("endmodule\n")
    }
}

impl Emit for Declaration<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        p.node(&self.field)?;
        p.write_str(" = ")?;
        p.node(&self.value)?;
        p.write_str(";\n")
    }
}

impl Emit for Register<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        p.write_str("Reg#(")?;
        p.node(&self.ty)?;
        write!(p, ") {} <- ", self.name)?;
        let arguments: Vec<_> = self.initial.into_iter().copied().collect();
        p.constructor(self.constructor, &arguments, self.clock, self.reset)
    }
}

impl Emit for Instance<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        p.node(&self.interface)?;
        write!(p, " {} <- ", self.name)?;
        p.constructor(self.constructor, self.arguments, self.clock, self.reset)
    }
}

impl Emit for LocalInterface<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        write!(p, "{} {} = ", self.value.ty, self.name)?;
        p.node(&self.value)?;
        p.write_str(";\n")
    }
}

impl Emit for InterfaceExpr<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        writeln!(p, "(interface {};", self.ty)?;
        p.nested(|p| p.bindings(self.members))?;
        p.write_str("endinterface)")
    }
}

impl Emit for Binding<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        match self {
            Self::Method(method) => p.node(method),
            Self::Interface { name, value } => {
                writeln!(p, "interface {} {name};", value.ty)?;
                p.nested(|p| p.bindings(value.members))?;
                p.write_str("endinterface\n")
            }
        }
    }
}

impl Emit for Method<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        if let (MethodBody::Forward { target, .. }, None) = (self.body, self.guard) {
            return writeln!(p, "method {} = {target};", self.signature.name);
        }
        p.signature(&self.signature)?;
        p.guard(self.guard)?;
        p.write_str(";\n")?;
        p.nested(|p| match self.body {
            MethodBody::Value(value) => {
                p.write_str("return ")?;
                p.node(value)?;
                p.write_str(";\n")
            }
            MethodBody::Action(body) => {
                if matches!(self.signature.output, Type::Action | Type::Void) {
                    p.action_block(body)
                } else {
                    p.node(body)
                }
            }
            MethodBody::Forward { target, .. } => {
                let mut result = String::from("bsv_forward_result");
                if matches!(self.signature.output, Type::ActionValue(_)) {
                    while self
                        .signature
                        .inputs
                        .iter()
                        .any(|input| input.name == result)
                        || target.split('.').any(|part| part == result)
                    {
                        result.push('_');
                    }
                    write!(p, "let {result} <- ")?;
                } else if !matches!(self.signature.output, Type::Action | Type::Void) {
                    p.write_str("return ")?;
                }
                write!(p, "{target}(")?;
                for (i, input) in self.signature.inputs.iter().enumerate() {
                    if i != 0 {
                        p.write_str(", ")?;
                    }
                    p.write_str(input.name)?;
                }
                p.write_str(");\n")?;
                if matches!(self.signature.output, Type::ActionValue(_)) {
                    writeln!(p, "return {result};")?;
                }
                Ok(())
            }
        })?;
        p.write_str("endmethod\n")
    }
}

impl Emit for Rule<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        write!(p, "rule {}", self.name)?;
        if let Some(guard) = self.guard {
            p.write_str(" (")?;
            p.node(guard)?;
            p.write_char(')')?;
        }
        p.write_str(";\n")?;
        p.nested(|p| p.action_block(self.body))?;
        p.write_str("endrule\n")
    }
}

impl Emit for Schedule<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        write!(p, "(* {} = ", self.attribute)?;
        p.quoted(&format!("{}, {}", self.first, self.second))?;
        p.write_str(" *)\n")
    }
}

impl Emit for Expr<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        match self.kind {
            ExprKind::Undefined | ExprKind::Unit => p.write_char('?'),
            ExprKind::Bool(value) => p.write_str(if value { "True" } else { "False" }),
            ExprKind::Integer(value) => write!(p, "{value}"),
            ExprKind::Bits { width, words_le } => {
                write!(p, "{width}'h")?;
                let digits = u64::from(width).div_ceil(4);
                if digits == 0 {
                    return p.write_char('0');
                }
                for digit in (0..digits).rev() {
                    let word = words_le.get((digit / 16) as usize).copied().unwrap_or(0);
                    let mut nibble = (word >> ((digit % 16) * 4)) & 15;
                    if digit == digits - 1 && width % 4 != 0 {
                        nibble &= (1 << (width % 4)) - 1;
                    }
                    write!(p, "{nibble:x}")?;
                }
                Ok(())
            }
            ExprKind::Path(path) => p.write_str(path),
            ExprKind::Call {
                function,
                arguments,
            } => {
                write!(p, "{function}(")?;
                p.list(arguments)?;
                p.write_char(')')
            }
            ExprKind::Unary { operator, operand } => {
                write!(p, "({operator}(")?;
                p.node(operand)?;
                p.write_str("))")
            }
            ExprKind::Binary { operator, lhs, rhs } => {
                let wrap = !p.fits(self, 1)?;
                p.write_char('(')?;
                if wrap {
                    p.write_char('\n')?;
                    p.nested(|p| {
                        p.node(lhs)?;
                        write!(p, "\n{operator} ")?;
                        p.node(rhs)
                    })?;
                    p.write_char('\n')?;
                } else {
                    p.node(lhs)?;
                    write!(p, " {operator} ")?;
                    p.node(rhs)?;
                }
                p.write_char(')')
            }
            ExprKind::Concat(values) => {
                p.write_char('{')?;
                p.list(values)?;
                p.write_char('}')
            }
            ExprKind::Annotated(value) => {
                let wrap = !p.fits(self, 1)?;
                p.node(&self.ty)?;
                p.write_str("'(")?;
                if wrap {
                    p.write_char('\n')?;
                    p.nested(|p| p.node(value))?;
                    p.write_char('\n')?;
                } else {
                    p.node(value)?;
                }
                p.write_char(')')
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let wrap = !p.fits(self, 1)?;
                p.write_char('(')?;
                if wrap {
                    p.write_char('\n')?;
                    p.nested(|p| {
                        p.node(condition)?;
                        p.write_str("\n? ")?;
                        p.node(then_value)?;
                        p.write_str("\n: ")?;
                        p.node(else_value)
                    })?;
                    p.write_char('\n')?;
                } else {
                    p.node(condition)?;
                    p.write_str(" ? ")?;
                    p.node(then_value)?;
                    p.write_str(" : ")?;
                    p.node(else_value)?;
                }
                p.write_char(')')
            }
            ExprKind::Struct { name, fields } => {
                write!(p, "{name} {{ ")?;
                p.list(fields)?;
                p.write_str(" }")
            }
            ExprKind::Tagged { tag, payload } => {
                write!(p, "(tagged {tag}")?;
                if let Some(payload) = payload {
                    p.write_str(" (")?;
                    p.node(payload)?;
                    p.write_char(')')?;
                }
                p.write_char(')')
            }
            ExprKind::Field { value, field } => {
                p.write_char('(')?;
                p.node(value)?;
                write!(p, ").{field}")
            }
            ExprKind::Index { value, index } => {
                p.write_char('(')?;
                p.node(value)?;
                p.write_str(")[")?;
                p.node(index)?;
                p.write_char(']')
            }
            ExprKind::TagTest {
                value,
                tag,
                has_payload,
            } => {
                p.write_str("(case (")?;
                p.node(value)?;
                write!(p, ") matches tagged {tag}")?;
                if has_payload {
                    p.write_str(" .*")?;
                }
                p.write_str(": True; default: False; endcase)")
            }
            ExprKind::TagPayload { value, tag } => {
                p.write_str("(case (")?;
                p.node(value)?;
                write!(
                    p,
                    ") matches tagged {tag} .bsv_payload: bsv_payload; default: ?; endcase)"
                )
            }
        }
    }
}

impl Emit for FieldValue<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        write!(p, "{}: ", self.name)?;
        p.node(&self.value)
    }
}

impl Emit for [Statement<'_>] {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        for statement in self {
            p.node(statement)?;
        }
        Ok(())
    }
}

impl Emit for Statement<'_> {
    fn emit(&self, p: &mut Printer<'_>) -> fmt::Result {
        match self {
            Self::Let { local, value } => {
                p.node(local)?;
                p.write_str(" = ")?;
                p.node(value)?;
                p.write_str(";\n")
            }
            Self::Assign { name, value } | Self::RegisterWrite { name, value } => {
                write!(
                    p,
                    "{name} {} ",
                    if matches!(self, Self::Assign { .. }) {
                        "="
                    } else {
                        "<="
                    }
                )?;
                p.node(value)?;
                p.write_str(";\n")
            }
            Self::ActionCall {
                action_value,
                target,
                arguments,
                result,
            } => {
                if let Some(result) = result {
                    if !action_value {
                        return Err(fmt::Error);
                    }
                    p.node(result)?;
                    p.write_str(" <- ")?;
                }
                write!(p, "{target}(")?;
                p.list(arguments)?;
                p.write_str(");\n")
            }
            Self::If {
                condition,
                then_body,
                else_body,
            } => {
                p.write_str("if (")?;
                p.node(condition)?;
                p.write_str(") begin\n")?;
                p.nested(|p| p.node(*then_body))?;
                if let Some(else_body) = else_body {
                    p.write_str("end else begin\n")?;
                    p.nested(|p| p.node(*else_body))?;
                }
                p.write_str("end\n")
            }
            Self::Return(Some(value)) => {
                p.write_str("return ")?;
                p.node(value)?;
                p.write_str(";\n")
            }
            Self::Return(None) => Err(fmt::Error),
            Self::Display { format, arguments, newline } => {
                p.write_str(if *newline { "$display(" } else { "$write(" })?;
                p.quoted(format)?;
                if !arguments.is_empty() {
                    p.write_str(", ")?;
                    p.list(arguments)?;
                }
                p.write_str(");\n")
            }
            Self::Assert {
                function,
                condition,
                message,
            } => {
                write!(p, "{function}(")?;
                p.node(condition)?;
                p.write_str(", ")?;
                p.quoted(message)?;
                p.write_str(");\n")
            }
            Self::Finish => p.write_str("$finish(0);\n"),
        }
    }
}
