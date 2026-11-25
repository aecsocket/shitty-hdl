use crate::{
    elab::{self, Context, ExprHandle},
    domain::Domain,
    value::{IntoValues, Shape, Value, ValueView},
};

macro_rules! impl_shape {
    ($($T:ident),*) => {
        impl<$($T: Shape),*> Shape for ($($T,)*) {
            type View = ($(<$T as Shape>::View,)*);
            type Values<D: Domain> = ($(<$T as Shape>::Values<D>,)*);

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
            fn make_values<D: Domain>(value: Value<D, Self>) -> Self::Values<D> {
                let mut index = 0;
                ($({
                    let element = elab::tuple_element::<D, Self, $T>(value, index);
                    index += 1;
                    $T::make_values(element)
                },)*)
            }

            #[allow(non_snake_case)]
            fn pack_values<D: Domain>(values: Self::Values<D>) -> Value<D, Self> {
                elab::tuple(&Self::value_handles::<D>(values))
            }

            fn value_handles<D: Domain>(values: Self::Values<D>) -> Vec<ExprHandle> {
                let ($($T,)*) = values;
                #[allow(unused_mut)]
                let mut handles = Vec::new();
                $(
                    handles.extend(<$T as Shape>::value_handles($T));
                )*
                handles
            }
        }


    };
}

variadics_please::all_tuples!(impl_shape, 0, 15, T);

macro_rules! impl_into_values_tuple {
    ($($T:ident),*) => {
        impl<D: Domain, $($T: Shape),*> IntoValues<D> for ($(Value<D, $T>,)*) {
            type Out = ($($T,)*);
            type Values = ($(<$T as Shape>::Values<D>,)*);

            #[allow(non_snake_case, clippy::unused_unit)]
            fn into_values(self) -> Self::Values {
                let ($($T,)*) = self;
                ($($T::make_values($T),)*)
            }
        }
    };
}

variadics_please::all_tuples!(impl_into_values_tuple, 1, 15, T);

impl Shape for bool {
    type View = ValueView<Self>;
    type Values<D: Domain> = Value<D, Self>;

    fn air_type<'dsn>(cx: &mut Context<'dsn>) -> &'dsn air::Type<'dsn> {
        cx.alloc(air::Type::Bool)
    }

    fn air_value<'dsn>(self, cx: &mut Context<'dsn>) -> &'dsn air::Expr<'dsn> {
        cx.boolean(self)
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


