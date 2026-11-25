use crate::{
    domain::Domain,
    elab::with_context,
    value::{IntoValue, Shape, Value},
};

/// Two-way combinational choice between hardware values.
pub fn mux<D: Domain, T: Shape>(
    condition: impl IntoValue<D, Out = bool>,
    then_value: Value<D, T>,
    else_value: Value<D, T>,
) -> Value<D, T> {
    let condition = condition.into_value();
    Value::from_handle(with_context(|cx| {
        let condition = cx.typed_expr::<bool>(condition.handle());
        let then_value = cx.typed_expr::<T>(then_value.handle());
        let else_value = cx.typed_expr::<T>(else_value.handle());
        let expr = cx.alloc(air::Expr {
            ty: then_value.ty,
            kind: air::ExprKind::Mux {
                condition,
                then_value,
                else_value,
            },
        });
        cx.push_expr(expr)
    }))
}
