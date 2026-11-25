use crate::value::{Shape, Value, ValueView};

macro_rules! impl_shape {
    ($($T:ident),*) => {
        impl<$($T: Shape),*> Shape for ($($T,)*) {
            type View = (
                $(<$T as Shape>::View,)*
            );
            type Values = (
                $(<$T as Shape>::Values,)*
            );
        }
    };
}

variadics_please::all_tuples!(impl_shape, 0, 15, T);

impl Shape for bool {
    type View = ValueView<Self>;
    type Values = Value<Self>;
}
