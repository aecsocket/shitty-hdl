use crate::{
    domain::Domain,
    value::{IntoValue, U, Value},
};
use std::ptr;

use super::{ExprHandle, with_context};
use crate::value::Shape;

pub fn literal<D: Domain, T: Shape>(value: &T) -> Value<D, T> {
    Value::from_handle(with_context(|cx| {
        let expr = value.air_value(cx);
        cx.push_expr(expr)
    }))
}

#[must_use]
pub fn tuple<D: Domain, T: Shape>(values: &[ExprHandle]) -> Value<D, T> {
    Value::from_handle(with_context(|cx| {
        let values = values.iter().map(|handle| cx.expr(*handle)).collect();
        let expr = cx.tuple_value::<T>(values);
        cx.push_expr(expr)
    }))
}

#[must_use]
pub fn tuple_element<D: Domain, T: Shape, Element: Shape>(
    value: Value<D, T>,
    index: usize,
) -> Value<D, Element> {
    Value::from_handle(with_context(|cx| {
        let value = cx.typed_expr::<T>(value.handle());
        let air::Type::Tuple(elements) = value.ty else {
            panic!("expected tuple value")
        };
        let ty = cx.ty::<Element>();
        assert!(ptr::eq(elements[index], ty), "tuple element type mismatch");
        let expr = cx.alloc(air::Expr {
            ty,
            kind: air::ExprKind::TupleElement { value, index },
        });
        cx.push_expr(expr)
    }))
}

#[must_use]
pub fn unary<D: Domain, T: Shape>(op: air::UnaryOp, value: Value<D, T>) -> Value<D, T> {
    Value::from_handle(with_context(|cx| {
        let operand = cx.typed_expr::<T>(value.handle());
        let expr = cx.alloc(air::Expr {
            ty: operand.ty,
            kind: air::ExprKind::Unary { op, operand },
        });
        cx.push_expr(expr)
    }))
}

#[must_use]
pub fn binary<D: Domain, T: Shape>(
    op: air::BinaryOp,
    lhs: Value<D, T>,
    rhs: Value<D, T>,
) -> Value<D, T> {
    Value::from_handle(with_context(|cx| {
        let lhs = cx.typed_expr::<T>(lhs.handle());
        let rhs = cx.typed_expr::<T>(rhs.handle());
        let expr = cx.alloc(air::Expr {
            ty: lhs.ty,
            kind: air::ExprKind::Binary { op, lhs, rhs },
        });
        cx.push_expr(expr)
    }))
}

#[must_use]
pub fn compare<D: Domain, T: Shape>(
    op: air::BinaryOp,
    lhs: Value<D, T>,
    rhs: Value<D, T>,
) -> Value<D, bool> {
    Value::from_handle(with_context(|cx| {
        let lhs = cx.typed_expr::<T>(lhs.handle());
        let rhs = cx.typed_expr::<T>(rhs.handle());
        let ty = cx.ty::<bool>();
        let expr = cx.alloc(air::Expr {
            ty,
            kind: air::ExprKind::Binary { op, lhs, rhs },
        });
        cx.push_expr(expr)
    }))
}

#[must_use]
pub fn cast<D: Domain, From: Shape, To: Shape>(
    kind: air::CastKind,
    value: Value<D, From>,
) -> Value<D, To> {
    Value::from_handle(with_context(|cx| {
        let value = cx.typed_expr::<From>(value.handle());
        let ty = cx.ty::<To>();
        match kind {
            air::CastKind::Truncate => assert!(
                width(ty) <= width(value.ty),
                "truncation cannot increase width"
            ),
            air::CastKind::ZeroExtend | air::CastKind::SignExtend => assert!(
                width(ty) >= width(value.ty),
                "extension cannot decrease width"
            ),
            air::CastKind::Pack | air::CastKind::Unpack | air::CastKind::Reinterpret => {
                assert_eq!(width(ty), width(value.ty), "cast width mismatch");
            }
        }
        let expr = cx.alloc(air::Expr {
            ty,
            kind: air::ExprKind::Cast { kind, value },
        });
        cx.push_expr(expr)
    }))
}

#[must_use]
pub fn uint_from_u128<D: Domain, const N: usize>(value: u128) -> Value<D, U<N>> {
    assert!(
        N >= 128 || value >> N == 0,
        "integer operand does not fit U<N>"
    );
    Value::from_handle(with_context(|cx| {
        let mut words = vec![0; N.div_ceil(64)];
        #[expect(
            clippy::cast_possible_truncation,
            reason = "intentional word extraction"
        )]
        let (lo, hi) = (value as u64, (value >> 64) as u64);
        if let Some(word) = words.get_mut(0) {
            *word = lo;
        }
        if let Some(word) = words.get_mut(1) {
            *word = hi;
        }
        let expr = cx.bits::<U<N>>(&words);
        cx.push_expr(expr)
    }))
}

#[must_use]
pub fn shift<D: Domain, T: Shape>(
    op: air::BinaryOp,
    value: Value<D, T>,
    amount: usize,
) -> Value<D, T> {
    let amount = uint_from_u128::<D, 64>(amount as u128);
    Value::from_handle(with_context(|cx| {
        let lhs = cx.typed_expr::<T>(value.handle());
        let rhs = cx.typed_expr::<U<64>>(amount.handle());
        let expr = cx.alloc(air::Expr {
            ty: lhs.ty,
            kind: air::ExprKind::Binary { op, lhs, rhs },
        });
        cx.push_expr(expr)
    }))
}

#[must_use]
pub fn shift_resize<D: Domain, const FROM: usize, const TO: usize>(
    op: air::BinaryOp,
    value: Value<D, U<FROM>>,
    amount: usize,
) -> Value<D, U<TO>> {
    match op {
        air::BinaryOp::ShiftLeft => shift(
            op,
            cast::<D, U<FROM>, U<TO>>(air::CastKind::ZeroExtend, value),
            amount,
        ),
        air::BinaryOp::ShiftRight => {
            cast::<D, U<FROM>, U<TO>>(air::CastKind::Truncate, shift(op, value, amount))
        }
        _ => panic!("expected shift operation"),
    }
}

pub fn display(format: &str, arguments: &[super::ExprHandle], newline: bool) {
    with_context(|cx| {
        let arguments: Vec<_> = arguments
            .iter()
            .map(|argument| cx.expr(*argument))
            .collect();
        cx.emit(air::Statement::Display {
            newline,
            format: cx.alloc_str(format),
            arguments: cx.alloc_slice(arguments),
        });
    });
}

pub fn assert<D: Domain>(condition: impl IntoValue<D, Out = bool>, message: &str) {
    let condition = condition.into_value();
    with_context(|cx| {
        let condition = cx.typed_expr::<bool>(condition.handle());
        cx.emit(air::Statement::Assert {
            condition,
            message: cx.alloc_str(message),
        });
    });
}

pub fn finish() {
    with_context(|cx| cx.emit(air::Statement::Finish));
}

pub fn width(ty: &air::Type<'_>) -> u64 {
    match ty {
        air::Type::Unit => 0,
        air::Type::Bool => 1,
        air::Type::Bit { width } | air::Type::Uint { width } | air::Type::Sint { width } => {
            u64::from(*width)
        }
        air::Type::Tuple(elements) => elements.iter().map(|ty| width(ty)).sum(),
        air::Type::Vector { element, length } => width(element)
            .checked_mul(*length as u64)
            .expect("type width overflow"),
        air::Type::Named(named) => match &named.kind {
            air::NamedTypeKind::Struct(fields) => fields.iter().map(|field| width(field.ty)).sum(),
            air::NamedTypeKind::Enum(variants) => {
                let count = variants.len().saturating_sub(1);
                u64::from(count.bit_width())
            }
            air::NamedTypeKind::TaggedUnion(variants) => {
                let count = variants.len().saturating_sub(1);
                let tag = u64::from(count.bit_width());
                tag + variants
                    .iter()
                    .filter_map(|variant| variant.payload)
                    .map(width)
                    .max()
                    .unwrap_or(0)
            }
            air::NamedTypeKind::Alias(ty) => width(ty),
        },
    }
}
