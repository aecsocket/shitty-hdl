use crate::io::Output;
use crate::reg::Reg;
use crate::{
    domain::Domain,
    elab,
    value::{FieldView, IntoValue, Shape},
};

/// The `<<=` register write, as a function call. The `hdl` macro lowers
/// `lhs <<= rhs` to [`WriteExt::write`] so that the receiver is consumed
/// by value: async branch closures then capture registers, outputs, and
/// views by copy instead of `&mut`, which lets two branches write the
/// same register.
pub trait WriteExt<D: Domain, Out: Shape>: IntoValue<D, Out> + Sized {
    fn write(self, rhs: impl IntoValue<D, Out>);
}

impl<D: Domain, T: Shape> WriteExt<D, T> for Reg<D, T> {
    fn write(self, rhs: impl IntoValue<D, T>) {
        elab::write_register(self.id, rhs.into_value());
    }
}

impl<D: Domain> WriteExt<D, bool> for Output<D> {
    fn write(self, rhs: impl IntoValue<D, bool>) {
        elab::call_action::<D, bool, ()>(self.writer, rhs.into_value());
    }
}

impl<D: Domain, S: Shape, T: Shape> WriteExt<D, T> for FieldView<D, S, T> {
    fn write(self, rhs: impl IntoValue<D, T>) {
        self.write(rhs);
    }
}
