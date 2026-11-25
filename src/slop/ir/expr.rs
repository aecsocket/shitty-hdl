use std::marker::PhantomData;

use crate::{
    ir::{self, DesignId, ValueId, ValueKind, ValueType, runtime},
    value::Shape,
};

#[derive(Debug)]
pub struct Expr<T: Shape> {
    pub(crate) owner: DesignId,
    pub(crate) id: ValueId,
    _phantom: PhantomData<T>,
}

impl<T: Shape> Clone for Expr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Shape> Copy for Expr<T> {}

pub(crate) fn shape_type<T: Shape>() -> ValueType {
    ValueType {
        width: T::WIDTH,
        signed: T::SIGNED,
    }
}

impl<T: Shape> Expr<T> {
    #[track_caller]
    pub(crate) fn constant(value: T) -> Self {
        let bits = value.to_bits();
        assert_eq!(
            bits.width(),
            T::WIDTH,
            "Shape::to_bits returned an incorrect width"
        );
        Self::node(ValueKind::Constant(bits))
    }

    #[track_caller]
    pub(crate) fn checked(self) -> Self {
        runtime::check_owner(self.owner);
        self
    }

    #[track_caller]
    pub(crate) fn read(signal: crate::signal::SignalId, offset: u32) -> Self {
        let (owner, id) = runtime::read(signal, offset, shape_type::<T>());
        Self::from_id(owner, id)
    }

    #[track_caller]
    pub(crate) fn assign(self, signal: crate::signal::SignalId, offset: u32) {
        runtime::write(signal, offset, T::WIDTH, self.owner, self.id);
    }

    pub(crate) fn from_id(owner: DesignId, id: ValueId) -> Self {
        Self {
            owner,
            id,
            _phantom: PhantomData,
        }
    }

    #[track_caller]
    pub(crate) fn node(kind: ValueKind) -> Self {
        let (owner, id) = runtime::value(shape_type::<T>(), kind);
        Self::from_id(owner, id)
    }

    #[track_caller]
    pub(crate) fn binary(self, rhs: Self, op: ir::BinaryOp) -> Self {
        runtime::check_owner(self.owner);
        runtime::check_owner(rhs.owner);
        Self::node(ValueKind::Binary {
            op,
            lhs: self.id,
            rhs: rhs.id,
        })
    }

    #[track_caller]
    pub(crate) fn compare(self, rhs: Self, op: ir::BinaryOp) -> Expr<bool> {
        runtime::check_owner(self.owner);
        runtime::check_owner(rhs.owner);
        Expr::node(ValueKind::Binary {
            op,
            lhs: self.id,
            rhs: rhs.id,
        })
    }

    #[track_caller]
    pub(crate) fn mux(condition: ValueId, then_value: Self, else_value: Self) -> Self {
        then_value.checked();
        else_value.checked();
        Self::node(ValueKind::Mux {
            condition,
            then_value: then_value.id,
            else_value: else_value.id,
        })
    }

    #[track_caller]
    pub(crate) fn resize<To: Shape>(self, extend: bool) -> Expr<To> {
        runtime::check_owner(self.owner);
        if extend {
            Expr::node(ValueKind::Extend {
                value: self.id,
                sign: T::SIGNED,
            })
        } else {
            Expr::node(ValueKind::Slice {
                value: self.id,
                offset: 0,
            })
        }
    }
}
