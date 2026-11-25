use std::num::NonZero;

use irt::Ident;

#[derive(Debug, Clone, Copy)]
pub struct Design<'d> {
    pub autoidx: Option<i32>,
    pub modules: &'d [Module<'d>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id {
    Public(Ident),
    Internal(Ident),
}

#[derive(Debug, Clone, Copy)]
pub struct Module<'d> {
    pub name: Id,
    pub attributes: &'d [Attribute<'d>],
    pub items: &'d [ModuleItem<'d>],
}

#[derive(Debug, Clone, Copy)]
pub enum ModuleItem<'d> {
    Parameter(ModuleParameter<'d>),
    Wire(Wire<'d>),
    Memory(Memory<'d>),
    Cell(Cell<'d>),
    Process(Process<'d>),
    Connect(Connect<'d>),
}

#[derive(Debug, Clone, Copy)]
pub struct Attribute<'d> {
    pub name: Id,
    pub value: Const<'d>,
}

#[derive(Debug, Clone, Copy)]
pub enum Const<'d> {
    Integer(i32),
    String(&'d str),
    Bits(&'d [State]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    Zero,
    One,
    X,
    Z,
    Marked,
    DontCare,
}

#[derive(Debug, Clone, Copy)]
pub struct ModuleParameter<'d> {
    pub name: Id,
    pub default: Option<Const<'d>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Wire<'d> {
    pub name: Id,
    pub attributes: &'d [Attribute<'d>],
    pub width: u32,
    pub offset: i32,
    pub upto: bool,
    pub signed: bool,
    pub port: Option<WirePort>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WirePort {
    pub id: NonZero<u32>,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone, Copy)]
pub struct Memory<'d> {
    pub name: Id,
    pub attributes: &'d [Attribute<'d>],
    pub width: u32,
    pub size: u32,
    pub offset: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Cell<'d> {
    pub name: Id,
    pub ty: Id,
    pub attributes: &'d [Attribute<'d>],
    pub parameters: &'d [CellParameter<'d>],
    pub connections: &'d [PortConnection<'d>],
}

#[derive(Debug, Clone, Copy)]
pub struct CellParameter<'d> {
    pub name: Id,
    pub kind: ParameterKind,
    pub value: Const<'d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterKind {
    Default,
    Signed,
    Real,
}

#[derive(Debug, Clone, Copy)]
pub struct PortConnection<'d> {
    pub port: Id,
    pub value: SigSpec<'d>,
}

#[derive(Debug, Clone, Copy)]
pub enum SigSpec<'d> {
    Const(Const<'d>),
    Wire(Id),
    Select { value: &'d Self, select: Select },
    Concat(&'d [Self]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Select {
    Bit(i32),
    Range { msb: i32, lsb: i32 },
}

#[derive(Debug, Clone, Copy)]
pub struct Connect<'d> {
    pub into: SigSpec<'d>,
    pub from: SigSpec<'d>,
}

#[derive(Debug, Clone, Copy)]
pub struct Process<'d> {
    pub name: Id,
    pub attributes: &'d [Attribute<'d>],
    pub body: CaseBody<'d>,
    pub syncs: &'d [Sync<'d>],
}

#[derive(Debug, Clone, Copy)]
pub struct CaseBody<'d> {
    pub assigns: &'d [Assign<'d>],
    pub switches: &'d [Switch<'d>],
}

#[derive(Debug, Clone, Copy)]
pub struct Assign<'d> {
    pub into: SigSpec<'d>,
    pub from: SigSpec<'d>,
}

#[derive(Debug, Clone, Copy)]
pub struct Switch<'d> {
    pub attributes: &'d [Attribute<'d>],
    pub on: SigSpec<'d>,
    pub cases: &'d [Case<'d>],
}

#[derive(Debug, Clone, Copy)]
pub struct Case<'d> {
    pub attributes: &'d [Attribute<'d>],
    pub compare: &'d [SigSpec<'d>],
    pub body: CaseBody<'d>,
}

#[derive(Debug, Clone, Copy)]
pub struct Sync<'d> {
    pub kind: SyncKind<'d>,
    pub actions: &'d [SyncAction<'d>],
}

#[derive(Debug, Clone, Copy)]
pub enum SyncKind<'d> {
    Low(SigSpec<'d>),
    High(SigSpec<'d>),
    PosEdge(SigSpec<'d>),
    NegEdge(SigSpec<'d>),
    Edge(SigSpec<'d>),
    Global,
    Init,
    Always,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncAction<'d> {
    Update(Assign<'d>),
    MemoryWrite(MemoryWrite<'d>),
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryWrite<'d> {
    pub attributes: &'d [Attribute<'d>],
    pub memory: Id,
    pub address: SigSpec<'d>,
    pub data: SigSpec<'d>,
    pub enable: SigSpec<'d>,
    pub priority_mask: Const<'d>,
}
