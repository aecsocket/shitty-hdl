use super::{LowerError, Lowerer};
use crate as nir;

const fn expr_width(shape: pir::Shape) -> u32 {
    match shape {
        pir::Shape::Bits(width) | pir::Shape::Unsigned(width) | pir::Shape::Signed(width) => width,
    }
}

const fn expr_signed(shape: pir::Shape) -> bool {
    matches!(shape, pir::Shape::Signed(_))
}

fn expr_common_width(lhs: pir::Shape, rhs: pir::Shape) -> Result<u32, LowerError> {
    let signed = expr_signed(lhs) || expr_signed(rhs);
    let adjusted = |shape| {
        let width = expr_width(shape);
        if signed && !expr_signed(shape) {
            width
                .checked_add(1)
                .ok_or(LowerError::OperandWidthOverflow { shape })
        } else {
            Ok(width)
        }
    };
    Ok(adjusted(lhs)?.max(adjusted(rhs)?))
}

impl<'d> Lowerer<'d> {
    fn expr_validated<T>(
        &mut self,
        errors: Vec<LowerError>,
        result: Result<T, LowerError>,
    ) -> Result<T, LowerError> {
        let mut errors = errors.into_iter();
        if let Some(primary) = errors.next() {
            self.errors.extend(errors);
            if let Err(error) = result {
                self.errors.push(error);
            }
            Err(primary)
        } else {
            result
        }
    }

    pub(super) fn value(
        &mut self,
        module: usize,
        value: pir::Value<'d>,
    ) -> Result<nir::Value<'d>, LowerError> {
        let shape = value.ty.0.shape;
        let width = expr_width(shape);
        match value.kind {
            pir::ValueKind::Const(bits) => {
                if bits.width() != width {
                    return Err(LowerError::ConstantWidthMismatch {
                        expected: width,
                        actual: bits.width(),
                    });
                }
                Ok(self.constant(bits))
            }
            pir::ValueKind::Signal(signal) => {
                let mut errors = Vec::new();
                if signal.0.ty != value.ty {
                    errors.push(LowerError::SignalTypeMismatch {
                        signal: signal.0.name,
                        expected: signal.0.ty.0.name,
                        actual: value.ty.0.name,
                    });
                }
                let result = self.signal_value(module, signal);
                self.expr_validated(errors, result)
            }
            pir::ValueKind::Unary(unary) => {
                let mut errors = Vec::new();
                let operand_shape = unary.value.ty.0.shape;
                let operand_width = expr_width(operand_shape);
                let operand_signed = expr_signed(operand_shape);
                let op = match unary.op {
                    pir::UnaryOp::Pos => None,
                    pir::UnaryOp::AsUnsigned | pir::UnaryOp::AsSigned => {
                        let expected = match unary.op {
                            pir::UnaryOp::AsUnsigned => pir::Shape::Unsigned(operand_width),
                            _ => pir::Shape::Signed(operand_width),
                        };
                        if shape != expected {
                            errors.push(LowerError::CastShapeMismatch {
                                op: unary.op,
                                expected,
                                actual: shape,
                            });
                        }
                        None
                    }
                    pir::UnaryOp::Neg => Some(nir::UnaryOp::Neg),
                    pir::UnaryOp::Not => Some(nir::UnaryOp::Not),
                    pir::UnaryOp::Bool => Some(nir::UnaryOp::Bool),
                    pir::UnaryOp::ReduceAnd => Some(nir::UnaryOp::ReduceAnd),
                    pir::UnaryOp::ReduceOr => Some(nir::UnaryOp::ReduceOr),
                    pir::UnaryOp::ReduceXor => Some(nir::UnaryOp::ReduceXor),
                };
                let reduction = matches!(
                    unary.op,
                    pir::UnaryOp::Bool
                        | pir::UnaryOp::ReduceAnd
                        | pir::UnaryOp::ReduceOr
                        | pir::UnaryOp::ReduceXor
                );
                if reduction && width != 1 {
                    errors.push(LowerError::UnaryResultWidthMismatch {
                        op: unary.op,
                        expected: 1,
                        actual: width,
                    });
                }
                let operand = self.value(module, *unary.value);
                let operand = self.expr_validated(errors, operand)?;
                let Some(op) = op else {
                    return Ok(self.resize(operand, width, operand_signed));
                };
                let operand = if reduction {
                    operand
                } else {
                    self.resize(operand, width, operand_signed)
                };
                Ok(self.emit(module, nir::Unary { op, value: operand }.into(), width))
            }
            pir::ValueKind::Binary(binary) => {
                let mut errors = Vec::new();
                let lhs_shape = binary.lhs.ty.0.shape;
                let rhs_shape = binary.rhs.ty.0.shape;
                let lhs_signed = expr_signed(lhs_shape);
                let rhs_signed = expr_signed(rhs_shape);
                let signedness = if lhs_signed || rhs_signed {
                    nir::Signedness::Signed
                } else {
                    nir::Signedness::Unsigned
                };
                let comparison = matches!(
                    binary.op,
                    pir::BinaryOp::Eq
                        | pir::BinaryOp::Ne
                        | pir::BinaryOp::Lt
                        | pir::BinaryOp::Le
                        | pir::BinaryOp::Gt
                        | pir::BinaryOp::Ge
                );
                if comparison && width != 1 {
                    errors.push(LowerError::BinaryResultWidthMismatch {
                        op: binary.op,
                        expected: 1,
                        actual: width,
                    });
                }
                let shift = matches!(binary.op, pir::BinaryOp::Shl | pir::BinaryOp::Shr);
                if shift && rhs_signed {
                    errors.push(LowerError::SignedShiftAmount { op: binary.op });
                }
                let op = match binary.op {
                    pir::BinaryOp::Add => nir::BinaryOp::Add,
                    pir::BinaryOp::Sub => nir::BinaryOp::Sub,
                    pir::BinaryOp::Mul => nir::BinaryOp::Mul,
                    pir::BinaryOp::FloorDiv => nir::BinaryOp::FloorDiv(signedness),
                    pir::BinaryOp::Mod => nir::BinaryOp::Mod(signedness),
                    pir::BinaryOp::Eq => nir::BinaryOp::Eq,
                    pir::BinaryOp::Ne => nir::BinaryOp::Ne,
                    pir::BinaryOp::Lt => nir::BinaryOp::Lt(signedness),
                    pir::BinaryOp::Le => nir::BinaryOp::Le(signedness),
                    pir::BinaryOp::Gt => nir::BinaryOp::Gt(signedness),
                    pir::BinaryOp::Ge => nir::BinaryOp::Ge(signedness),
                    pir::BinaryOp::And => nir::BinaryOp::And,
                    pir::BinaryOp::Or => nir::BinaryOp::Or,
                    pir::BinaryOp::Xor => nir::BinaryOp::Xor,
                    pir::BinaryOp::Shl => nir::BinaryOp::Shl,
                    pir::BinaryOp::Shr => nir::BinaryOp::Shr(if lhs_signed {
                        nir::Signedness::Signed
                    } else {
                        nir::Signedness::Unsigned
                    }),
                };
                let common_width = if shift {
                    expr_width(lhs_shape)
                } else {
                    match expr_common_width(lhs_shape, rhs_shape) {
                        Ok(width) => width,
                        Err(error) => {
                            errors.push(error);
                            0
                        }
                    }
                };
                let operation_width = if comparison {
                    common_width
                } else {
                    common_width.max(width)
                };
                let lhs = self.value(module, *binary.lhs);
                let rhs = self.value(module, *binary.rhs);
                let operands = match (lhs, rhs) {
                    (Ok(lhs), Ok(rhs)) => Ok((lhs, rhs)),
                    (Err(lhs), Err(rhs)) => {
                        self.errors.push(rhs);
                        Err(lhs)
                    }
                    (Err(error), _) | (_, Err(error)) => Err(error),
                };
                let (lhs, rhs) = self.expr_validated(errors, operands)?;
                let lhs = self.resize(lhs, operation_width, lhs_signed);
                let rhs = if shift {
                    rhs
                } else {
                    self.resize(rhs, operation_width, rhs_signed)
                };
                let result = self.emit(
                    module,
                    nir::Binary { op, lhs, rhs }.into(),
                    if comparison { 1 } else { operation_width },
                );
                Ok(self.resize(result, width, expr_signed(shape)))
            }
        }
    }
}
