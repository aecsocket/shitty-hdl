use crate::value::Shape;

macro_rules! impl_shape {
    ($($T:ident),*) => {
        impl<$($T: Shape),*> Shape for ($($T,)*) {
            type Value = (
                $(<$T as Shape>::Value,)*
            );
        }
    };
}

variadics_please::all_tuples!(impl_shape, 0, 15, T);
