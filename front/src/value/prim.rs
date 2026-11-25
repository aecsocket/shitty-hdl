use crate::value::{IntoValue, IntoValues, Shape, Value, ValueView};

macro_rules! impl_shape {
    ($($T:ident),*) => {
        impl<$($T: Shape),*> Shape for ($($T,)*) {
            type View = (
                $(<$T as Shape>::View,)*
            );
        }

        impl<$($T: IntoValues),*> IntoValues for ($($T,)*) {
            type Out = ($($T::Out,)*);
            type Values = ($($T::Values,)*);

            #[allow(non_snake_case)]
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
}

impl IntoValues for bool {
    type Out = Self;
    type Values = Value<Self>;

    fn into_values(self) -> Self::Values {
        self.into_value()
    }
}
