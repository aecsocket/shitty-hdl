use crate::{
    elab::{self, Context},
    value::{IntoValue, IntoValues, Shape, Value, ValueView},
};

macro_rules! impl_shape {
    ($($T:ident),*) => {
        impl<$($T: Shape),*> Shape for ($($T,)*) {
            type View = ($(<$T as Shape>::View,)*);

            fn air_type<'dsn>(cx: &mut Context<'dsn>) -> &'dsn air::Type<'dsn> {
                let elements = vec![$(cx.ty::<$T>(),)*];
                if elements.is_empty() {
                    cx.alloc(air::Type::Unit)
                } else {
                    let elements = cx.alloc_slice(elements);
                    cx.alloc(air::Type::Tuple(elements))
                }
            }

            #[allow(non_snake_case, unused_variables)]
            fn air_value<'dsn>(self, cx: &mut Context<'dsn>) -> &'dsn air::Expr<'dsn> {
                let ($($T,)*) = self;
                let elements = vec![$($T::air_value($T, cx),)*];
                cx.tuple_value::<Self>(elements)
            }

            #[allow(unused_assignments, unused_mut, unused_variables, clippy::unused_unit)]
            fn make_values(value: Value<Self>) -> Self::Values {
                let mut index = 0;
                ($({
                    let element = elab::tuple_element::<Self, $T>(value, index);
                    index += 1;
                    $T::make_values(element)
                },)*)
            }

            #[allow(non_snake_case)]
            fn pack_values(values: Self::Values) -> Value<Self> {
                let ($($T,)*) = values;
                elab::tuple::<Self>(&[$($T::pack_values($T).handle(),)*])
            }
        }

        impl<$($T: IntoValues),*> IntoValues for ($($T,)*) {
            type Out = ($($T::Out,)*);
            type Values = ($($T::Values,)*);

            #[allow(non_snake_case, clippy::unused_unit)]
            fn into_values(self) -> Self::Values {
                let ($($T,)*) = self;
                ($($T.into_values(),)*)
            }
        }
    };
}

variadics_please::all_tuples!(impl_shape, 0, 15, T);

impl Shape for bool {
    type View = ValueView<Self>;

    fn air_type<'dsn>(cx: &mut Context<'dsn>) -> &'dsn air::Type<'dsn> {
        cx.alloc(air::Type::Bool)
    }

    fn air_value<'dsn>(self, cx: &mut Context<'dsn>) -> &'dsn air::Expr<'dsn> {
        cx.boolean(self)
    }

    fn make_values(value: Value<Self>) -> Self::Values {
        value
    }

    fn pack_values(values: Self::Values) -> Value<Self> {
        values
    }
}

impl IntoValues for bool {
    type Out = Self;
    type Values = Value<Self>;

    fn into_values(self) -> Self::Values {
        self.into_value()
    }
}
