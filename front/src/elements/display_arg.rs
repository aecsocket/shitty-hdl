use std::fmt::{Binary, Display, LowerHex, Octal};

use crate::{
    domain::Domain,
    elab::handle::ExprHandle,
    reg::Reg,
    value::{IntoValue, Shape, Value},
};

pub struct DisplaySpec {
    pub hardware: String,
    pub width: usize,
    pub kind: char,
}

/// Autoref specialization, two-trait variant.
///
/// Method resolution tries receivers by value before `&x`, so a capture that
/// receivers by value before `&x`, so a capture that is `IntoValue` hits
/// `DisplayArgHardware` first, while plain Rust values fall through to
/// `DisplayArgText` on `&T` and are formatted at elaboration time.
pub trait DisplayArgHardware<D: Domain> {
    fn push_display_arg(self, fmt: &mut String, args: &mut Vec<ExprHandle>, spec: &DisplaySpec);
}

impl<D: Domain, T: Shape> DisplayArgHardware<D> for T {
    fn push_display_arg(self, fmt: &mut String, args: &mut Vec<ExprHandle>, spec: &DisplaySpec) {
        fmt.push_str(&spec.hardware);
        args.push(<T as IntoValue<D, T>>::into_value(self).handle());
    }
}

impl<D: Domain, T: Shape> DisplayArgHardware<D> for Value<D, T> {
    fn push_display_arg(self, fmt: &mut String, args: &mut Vec<ExprHandle>, spec: &DisplaySpec) {
        fmt.push_str(&spec.hardware);
        args.push(self.handle());
    }
}

impl<D: Domain, T: Shape> DisplayArgHardware<D> for Reg<D, T> {
    fn push_display_arg(self, fmt: &mut String, args: &mut Vec<ExprHandle>, spec: &DisplaySpec) {
        fmt.push_str(&spec.hardware);
        args.push(self.read().handle());
    }
}

pub trait DisplayArgText {
    fn push_display_arg(self, fmt: &mut String, args: &mut Vec<ExprHandle>, spec: &DisplaySpec);
}

impl<T: Display + LowerHex + Binary + Octal> DisplayArgText for &T {
    fn push_display_arg(self, fmt: &mut String, _args: &mut Vec<ExprHandle>, spec: &DisplaySpec) {
        let text = match spec.kind {
            'x' => format!("{:width$x}", self, width = spec.width),
            'b' => format!("{:width$b}", self, width = spec.width),
            'o' => format!("{:width$o}", self, width = spec.width),
            _ => format!("{:width$}", self, width = spec.width),
        };
        fmt.push_str(&text.replace('%', "%%"));
    }
}
