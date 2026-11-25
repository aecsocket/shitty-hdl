use derive_more::{Display, Error};
use irt::Ident;

#[derive(Debug, Clone, PartialEq, Eq, Display, Error)]
pub enum LowerError {
    #[display("invalid RTLIL identifier {name:?}")]
    InvalidIdentifier { name: Ident },
    #[display("duplicate child {child} in module {parent}")]
    DuplicateModule { parent: Ident, child: Ident },
    #[display("duplicate port {port} in module {module}")]
    DuplicatePort { module: Ident, port: Ident },
    #[display("cell {cell} in {module} is already declared in {first_module}")]
    DuplicateCell {
        module: Ident,
        first_module: Ident,
        cell: usize,
    },
    #[display("width {width} in {module} exceeds RTLIL's signed 32-bit range")]
    WidthOverflow { module: Ident, width: usize },
    #[display("late net {net} has duplicate definitions")]
    DuplicateConnection { net: Ident },
    #[display("late net {net} has no definition")]
    MissingConnection { net: Ident },
    #[display("late-net connection cycle at {net}")]
    ConnectionCycle { net: Ident },
    #[display("module {module} references an unknown cell")]
    UnknownCell { module: Ident },
    #[display("cell {cell} bit {bit} in {module} exceeds its width {width}")]
    CellBitOutOfBounds {
        module: Ident,
        cell: usize,
        bit: u32,
        width: u32,
    },
    #[display(
        "module {module} cannot access cell {cell} bit {bit} from {source_module} through its ports"
    )]
    UnroutedNet {
        module: Ident,
        source_module: Ident,
        cell: usize,
        bit: u32,
    },
    #[display("input cell {cell} in {module} is not in the top module")]
    InputCellInChild { module: Ident, cell: usize },
    #[display("input port {port} bit {bit} in {module} is not an external input source")]
    InvalidInputPort {
        module: Ident,
        port: Ident,
        bit: u32,
    },
    #[display("input port {port} bit {bit} in {module} is already driven by another input")]
    DuplicateInputBit {
        module: Ident,
        port: Ident,
        bit: u32,
    },
    #[display("input port {port} in {module} is driven inside the module")]
    InputPortDrivenInternally { module: Ident, port: Ident },
    #[display("input cell {cell} bit {bit} in {module} has no top-level input port")]
    UnboundInput {
        module: Ident,
        cell: usize,
        bit: u32,
    },
    #[display("cell {cell} in {module} has operand width {actual}, expected {expected}")]
    OperandWidthMismatch {
        module: Ident,
        cell: usize,
        expected: usize,
        actual: usize,
    },
    #[display("flip-flop cell {cell} in {module} has initial width {actual}, expected {expected}")]
    InitialWidthMismatch {
        module: Ident,
        cell: usize,
        expected: u32,
        actual: u32,
    },
    #[display("match cell {cell} in {module} has pattern width {actual}, expected {expected}")]
    PatternWidthMismatch {
        module: Ident,
        cell: usize,
        expected: u32,
        actual: u32,
    },
    #[display(
        "assignment in cell {cell} in {module}, offset {offset}, width {width}, exceeds output width {output_width}"
    )]
    AssignmentOutOfBounds {
        module: Ident,
        cell: usize,
        offset: u32,
        width: u32,
        output_width: u32,
    },
}
