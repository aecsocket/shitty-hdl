use std::error::Error;

use derive_more::Display;

use crate::{Ident, ShapeSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum SignalAccess {
    #[display("read")]
    Read,
    #[display("drive")]
    Drive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum PatternPart {
    #[display("pattern")]
    Value,
    #[display("pattern mask")]
    Mask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum DriverDomain {
    #[display("combinational")]
    Comb,
    #[display("clocked domain {name}")]
    Clocked { name: Ident },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetLocation {
    Const(bool),
    Late { name: Ident },
    Cell { cell: usize, bit: ShapeSize },
}

#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum LowerError {
    #[display("duplicate port {port} in module {module}")]
    DuplicatePortName { module: Ident, port: Ident },
    #[display(
        "ports {first_port} and {second_port} in module {module} expose the same signal {signal}"
    )]
    DuplicatePortSignal {
        module: Ident,
        signal: Ident,
        first_port: Ident,
        second_port: Ident,
    },
    #[display("duplicate child module {child} in module {module}")]
    DuplicateChildModule { module: Ident, child: Ident },
    #[display("signal {signal} is declared in both {first_module} and {second_module}")]
    DuplicateSignalDeclaration {
        signal: Ident,
        first_module: Ident,
        second_module: Ident,
    },
    #[display("signal {signal} initial value has width {actual}, expected {expected}")]
    InitialWidthMismatch {
        signal: Ident,
        expected: ShapeSize,
        actual: ShapeSize,
    },
    #[display("module {module} cannot {access} undeclared signal {signal}")]
    UndeclaredSignal {
        module: Ident,
        signal: Ident,
        access: SignalAccess,
    },
    #[display("module {module} cannot {access} signal {signal} declared in {declared_in}")]
    SignalAccessViolation {
        module: Ident,
        signal: Ident,
        declared_in: Ident,
        access: SignalAccess,
    },
    #[display("signal {signal} has type {expected}, but its value declares {actual}")]
    SignalTypeMismatch {
        signal: Ident,
        expected: Ident,
        actual: Ident,
    },
    #[display(
        "assignment to {signal} at offset {offset} with width {width} exceeds signal width {signal_width}"
    )]
    AssignmentOutOfBounds {
        signal: Ident,
        offset: ShapeSize,
        width: ShapeSize,
        signal_width: ShapeSize,
    },
    #[display("assignment to {signal} has width {actual}, expected {expected}")]
    AssignmentWidthMismatch {
        signal: Ident,
        expected: ShapeSize,
        actual: ShapeSize,
    },
    #[display("assignment to {signal} has type {actual}, expected {expected}")]
    AssignmentTypeMismatch {
        signal: Ident,
        expected: Ident,
        actual: Ident,
    },
    #[display(
        "signal {signal} bit {bit} is driven by both {first_module} ({first_domain}) and {second_module} ({second_domain})"
    )]
    ConflictingDrivers {
        signal: Ident,
        bit: ShapeSize,
        first_module: Ident,
        first_domain: DriverDomain,
        second_module: Ident,
        second_domain: DriverDomain,
    },
    #[display("{part} has width {actual}, expected switch width {expected}")]
    PatternWidthMismatch {
        part: PatternPart,
        expected: ShapeSize,
        actual: ShapeSize,
    },
    #[display("switch guard has width {actual}, expected one bit")]
    GuardWidthMismatch { actual: ShapeSize },
    #[display("constant has width {actual}, expected {expected}")]
    ConstantWidthMismatch {
        expected: ShapeSize,
        actual: ShapeSize,
    },
    #[display("normalizing operand shape {shape:?} would overflow the width limit")]
    OperandWidthOverflow { shape: pir::Shape },
    #[display("cast {op:?} produces shape {actual:?}, expected {expected:?}")]
    CastShapeMismatch {
        op: pir::UnaryOp,
        expected: pir::Shape,
        actual: pir::Shape,
    },
    #[display("unary operation {op:?} has result width {actual}, expected {expected}")]
    UnaryResultWidthMismatch {
        op: pir::UnaryOp,
        expected: ShapeSize,
        actual: ShapeSize,
    },
    #[display("binary operation {op:?} has result width {actual}, expected {expected}")]
    BinaryResultWidthMismatch {
        op: pir::BinaryOp,
        expected: ShapeSize,
        actual: ShapeSize,
    },
    #[display("shift operation {op:?} requires an unsigned shift amount")]
    SignedShiftAmount { op: pir::BinaryOp },
    #[display("late net {name} has multiple definitions")]
    DuplicateLateNetDefinition { name: Ident },
    #[display("late net {name} has no definition")]
    MissingLateNetDefinition { name: Ident },
    #[display("reference to an unknown cell")]
    UnknownCell,
    #[display("cell {cell} bit {bit} is out of range for width {width}")]
    CellBitOutOfRange {
        cell: usize,
        bit: ShapeSize,
        width: ShapeSize,
    },
    #[display("operand bit {bit} is out of range for width {width}")]
    OperandBitOutOfRange { bit: ShapeSize, width: usize },
    #[display("combinational cycle at {at:?}")]
    CombinationalCycle { at: NetLocation },
}

impl Error for LowerError {}
