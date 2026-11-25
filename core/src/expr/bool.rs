use std::ops::Not;

use crate::{
    expr::{EqExt, IntoValue, Value},
    ir::{BinaryOp, UnaryOp},
    module::{In, Out},
    signal::{Signal, SignalView},
};

macro_rules! impl_not {
    ($($ty:ident)*) => {$(
        impl Not for $ty<bool> {
            type Output = Value<bool>;

            #[track_caller]
            fn not(self) -> Self::Output {
                Value { imp: self.into_value().imp.unary(UnaryOp::Not) }
            }
        }
    )*};
}

impl_not!(Value Signal SignalView In Out);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ir, module::comb, sim::Simulator};

    #[test]
    fn boolean_not() {
        let (design, (input, inverted, restored)) = ir::elaborate_named("boolean_not", || {
            let input = Signal::<bool>::input("input", false);
            let mut inverted = Signal::<bool>::output("inverted", false);
            let mut restored = Signal::<bool>::output("restored", false);
            comb(|| {
                inverted <<= !input;
                restored <<= !(!input.into_value());
            });
            (input, inverted, restored)
        })
        .unwrap();
        let mut sim = Simulator::from_design(design).unwrap();
        for value in [false, true, false] {
            sim.set(input, value).unwrap();
            assert_eq!(sim.get(inverted).unwrap(), !value);
            assert_eq!(sim.get(restored).unwrap(), value);
        }
    }
}

impl<Lhs: IntoValue> EqExt for Lhs {
    #[track_caller]
    fn eq(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        Value {
            imp: self
                .into_value()
                .imp
                .compare(rhs.into_value().imp, BinaryOp::Eq),
        }
    }

    #[track_caller]
    fn ne(self, rhs: impl IntoValue<Out = Self::Out>) -> Value<bool> {
        Value {
            imp: self
                .into_value()
                .imp
                .compare(rhs.into_value().imp, BinaryOp::Ne),
        }
    }
}
