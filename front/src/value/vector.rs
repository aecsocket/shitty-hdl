use std::array::from_fn;
use std::ptr;

use crate::{
    domain::Domain,
    elab::{Context, ExprHandle, with_context},
    value::{IntoValue, Shape, Uint, Value, ValueView},
};

pub struct Vector<T, const N: usize>(pub [T; N]);

impl<T: Shape, const N: usize> Vector<T, N> {
    #[must_use]
    pub fn from_fn(f: impl FnMut(usize) -> T) -> Self {
        Self(from_fn(f))
    }

    #[must_use]
    pub fn select<D: Domain, const W: usize>(
        &self,
        index: impl IntoValue<D, Out = Uint<W>>,
    ) -> Value<D, T> {
        assert!(
            N == 1 || 1_usize << W >= N,
            "index width cannot address table"
        );
        let index = index.into_value();
        Value::from_handle(with_context(|cx| {
            let index = cx.typed_expr::<Uint<W>>(index.handle());
            let value = self.air_value(cx);
            let air::Type::Vector { element, .. } = value.ty else {
                panic!("select requires a vector value")
            };
            let expr = cx.alloc(air::Expr {
                ty: element,
                kind: air::ExprKind::Index { value, index },
            });
            cx.push_expr(expr)
        }))
    }
}

impl<T, const N: usize> FromIterator<T> for Vector<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(
            <[T; N]>::try_from(iter.into_iter().collect::<Vec<T>>())
                .unwrap_or_else(|_| panic!("iterator length does not match vector")),
        )
    }
}

impl<T: Shape, const N: usize> Shape for Vector<T, N> {
    type View = ValueView<Self>;
    type Values<D: Domain> = Value<D, Self>;

    fn air_type<'dsn>(cx: &mut Context<'dsn>) -> &'dsn air::Type<'dsn> {
        let element = cx.ty::<T>();
        cx.alloc(air::Type::Vector { element, length: N })
    }

    fn air_value<'dsn>(&self, cx: &mut Context<'dsn>) -> &'dsn air::Expr<'dsn> {
        let ty = cx.ty::<Self>();
        let air::Type::Vector { element, length } = ty else {
            panic!("vector literal requires a vector type")
        };
        assert_eq!(*length, N, "vector length mismatch");
        let mut elements = Vec::new();
        for value in &self.0 {
            let expr = value.air_value(cx);
            assert!(ptr::eq(expr.ty, *element), "vector element type mismatch");
            elements.push(expr);
        }
        let elements = cx.alloc_slice(elements);
        cx.alloc(air::Expr {
            ty,
            kind: air::ExprKind::Vector(elements),
        })
    }

    fn make_values<D: Domain>(value: Value<D, Self>) -> Self::Values<D> {
        value
    }

    fn pack_values<D: Domain>(values: Self::Values<D>) -> Value<D, Self> {
        values
    }

    fn value_handles<D: Domain>(values: Self::Values<D>) -> Vec<ExprHandle> {
        vec![values.handle()]
    }
}
