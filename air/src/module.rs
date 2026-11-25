use crate::iface::{self, Argument, Field, MethodSignature, NamedType, Type, UnionVariant};

#[derive(Debug, Clone)]
pub struct ModuleSignature<'dsn> {
    pub name: &'dsn str,
    pub parameters: &'dsn [&'dsn Parameter<'dsn>],
    pub interface: &'dsn iface::Interface<'dsn>,
}

#[derive(Debug, Clone, Copy)]
pub enum Parameter<'dsn> {
    Value {
        name: &'dsn str,
        ty: &'dsn Type<'dsn>,
    },
    Interface {
        name: &'dsn str,
        ty: &'dsn iface::Interface<'dsn>,
    },
    Clock {
        name: &'dsn str,
    },
}

#[derive(Debug, Clone)]
pub struct Module<'dsn> {
    pub signature: &'dsn ModuleSignature<'dsn>,
    pub types: &'dsn [&'dsn NamedType<'dsn>],
    pub instances: &'dsn [&'dsn Instance<'dsn>],
    pub registers: &'dsn [&'dsn Register<'dsn>],
    pub methods: &'dsn [&'dsn Method<'dsn>],
    pub rules: &'dsn [&'dsn Rule<'dsn>],
    pub interface: &'dsn InterfaceImpl<'dsn>,
    pub scheduling: &'dsn [ScheduleConstraint<'dsn>],
}

#[derive(Debug, Clone)]
pub struct Instance<'dsn> {
    pub name: &'dsn str,
    pub module: &'dsn ModuleSignature<'dsn>,
    pub arguments: &'dsn [InstanceArgument<'dsn>],
    pub clock: ClockRef<'dsn>,
}

#[derive(Debug, Clone, Copy)]
pub enum InstanceArgument<'dsn> {
    Value {
        parameter: &'dsn Parameter<'dsn>,
        value: &'dsn Expr<'dsn>,
    },
    Interface {
        parameter: &'dsn Parameter<'dsn>,
        interface: &'dsn InterfaceRef<'dsn>,
    },
    Clock {
        parameter: &'dsn Parameter<'dsn>,
        clock: ClockRef<'dsn>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum InterfaceRef<'dsn> {
    Parameter(&'dsn Parameter<'dsn>),
    Instance(&'dsn Instance<'dsn>),
    Local(&'dsn InterfaceImpl<'dsn>),
    Subinterface {
        parent: &'dsn Self,
        member: &'dsn iface::InterfaceMember<'dsn>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum MethodRef<'dsn> {
    Local(&'dsn Method<'dsn>),
    Interface {
        interface: &'dsn InterfaceRef<'dsn>,
        signature: &'dsn MethodSignature<'dsn>,
    },
}

#[derive(Debug, Clone)]
pub struct InterfaceImpl<'dsn> {
    pub ty: &'dsn iface::Interface<'dsn>,
    pub members: &'dsn [InterfaceBinding<'dsn>],
}

#[derive(Debug, Clone, Copy)]
pub enum InterfaceBinding<'dsn> {
    Method {
        declaration: &'dsn MethodSignature<'dsn>,
        method: MethodRef<'dsn>,
    },
    Interface {
        declaration: &'dsn iface::InterfaceMember<'dsn>,
        implementation: &'dsn InterfaceImpl<'dsn>,
    },
}

#[derive(Debug, Clone)]
pub struct Method<'dsn> {
    pub signature: &'dsn MethodSignature<'dsn>,
    pub guard: Option<&'dsn Expr<'dsn>>,
    pub body: MethodBody<'dsn>,
}

#[derive(Debug, Clone, Copy)]
pub enum MethodBody<'dsn> {
    Value(&'dsn Expr<'dsn>),
    Action(Block<'dsn>),
}

#[derive(Debug, Clone)]
pub struct Rule<'dsn> {
    pub name: &'dsn str,
    pub guard: Option<&'dsn Expr<'dsn>>,
    pub body: Block<'dsn>,
}

#[derive(Debug, Clone)]
pub struct Register<'dsn> {
    pub name: &'dsn str,
    pub ty: &'dsn Type<'dsn>,
    pub clock: ClockRef<'dsn>,
    pub reset: RegisterReset<'dsn>,
}

#[derive(Debug, Clone, Copy)]
pub enum RegisterReset<'dsn> {
    Uninitialized,
    Synchronous(&'dsn Expr<'dsn>),
    Asynchronous(&'dsn Expr<'dsn>),
}

#[derive(Debug, Clone, Copy)]
pub enum ClockRef<'dsn> {
    Default,
    None,
    Parameter(&'dsn Parameter<'dsn>),
}

#[derive(Debug, Clone)]
pub struct Expr<'dsn> {
    pub ty: &'dsn Type<'dsn>,
    pub kind: ExprKind<'dsn>,
}

#[derive(Debug, Clone, Copy)]
pub struct BitLiteral<'dsn> {
    pub width: u32,
    pub words_le: &'dsn [u64],
}

#[derive(Debug, Clone, Copy)]
pub enum ExprKind<'dsn> {
    Unit,
    Bool(bool),
    Bits(BitLiteral<'dsn>),
    Parameter(&'dsn Parameter<'dsn>),
    Argument(&'dsn Argument<'dsn>),
    Local(&'dsn Local<'dsn>),
    Register(&'dsn Register<'dsn>),
    MethodCall {
        method: MethodRef<'dsn>,
        args: &'dsn [&'dsn Expr<'dsn>],
    },
    Unary {
        op: UnaryOp,
        operand: &'dsn Expr<'dsn>,
    },
    Binary {
        op: BinaryOp,
        lhs: &'dsn Expr<'dsn>,
        rhs: &'dsn Expr<'dsn>,
    },
    Cast {
        kind: CastKind,
        value: &'dsn Expr<'dsn>,
    },
    Mux {
        condition: &'dsn Expr<'dsn>,
        then_value: &'dsn Expr<'dsn>,
        else_value: &'dsn Expr<'dsn>,
    },
    Tuple(&'dsn [&'dsn Expr<'dsn>]),
    Vector(&'dsn [&'dsn Expr<'dsn>]),
    Struct(&'dsn [FieldValue<'dsn>]),
    Enum {
        variant: &'dsn str,
    },
    TaggedUnion {
        variant: &'dsn UnionVariant<'dsn>,
        payload: Option<&'dsn Expr<'dsn>>,
    },
    Field {
        value: &'dsn Expr<'dsn>,
        field: &'dsn Field<'dsn>,
    },
    TupleElement {
        value: &'dsn Expr<'dsn>,
        index: usize,
    },
    Index {
        value: &'dsn Expr<'dsn>,
        index: &'dsn Expr<'dsn>,
    },
    Slice {
        value: &'dsn Expr<'dsn>,
        offset: &'dsn Expr<'dsn>,
        width: u32,
    },
    IsVariant {
        value: &'dsn Expr<'dsn>,
        variant: &'dsn UnionVariant<'dsn>,
    },
    UnionPayload {
        value: &'dsn Expr<'dsn>,
        variant: &'dsn UnionVariant<'dsn>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct FieldValue<'dsn> {
    pub field: &'dsn Field<'dsn>,
    pub value: &'dsn Expr<'dsn>,
}

#[derive(Debug, Clone, Copy)]
pub enum CastKind {
    Pack,
    Unpack,
    ZeroExtend,
    SignExtend,
    Truncate,
    Reinterpret,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Not,
    Negate,
    BitNot,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Concat,
}

#[derive(Debug, Clone)]
pub struct Local<'dsn> {
    pub name: &'dsn str,
    pub ty: &'dsn Type<'dsn>,
}

#[derive(Debug, Clone, Copy)]
pub struct Block<'dsn> {
    pub statements: &'dsn [Statement<'dsn>],
}

#[derive(Debug, Clone, Copy)]
pub enum Statement<'dsn> {
    Let {
        local: &'dsn Local<'dsn>,
        value: &'dsn Expr<'dsn>,
    },
    RegisterWrite {
        register: &'dsn Register<'dsn>,
        value: &'dsn Expr<'dsn>,
    },
    ActionCall {
        method: MethodRef<'dsn>,
        args: &'dsn [&'dsn Expr<'dsn>],
        result: Option<&'dsn Local<'dsn>>,
    },
    If {
        condition: &'dsn Expr<'dsn>,
        then_body: Block<'dsn>,
        else_body: Option<Block<'dsn>>,
    },
    Return(Option<&'dsn Expr<'dsn>>),
    Assert {
        condition: &'dsn Expr<'dsn>,
        message: &'dsn str,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum ScheduleTarget<'dsn> {
    Rule(&'dsn Rule<'dsn>),
    Method(&'dsn Method<'dsn>),
}

#[derive(Debug, Clone, Copy)]
pub enum ScheduleConstraint<'dsn> {
    Urgency {
        higher: ScheduleTarget<'dsn>,
        lower: ScheduleTarget<'dsn>,
    },
    ExecutionOrder {
        before: ScheduleTarget<'dsn>,
        after: ScheduleTarget<'dsn>,
    },
    MutualExclusion {
        first: ScheduleTarget<'dsn>,
        second: ScheduleTarget<'dsn>,
    },
    ConflictFree {
        first: ScheduleTarget<'dsn>,
        second: ScheduleTarget<'dsn>,
    },
    Preemption {
        preemptor: ScheduleTarget<'dsn>,
        preempted: ScheduleTarget<'dsn>,
    },
}
