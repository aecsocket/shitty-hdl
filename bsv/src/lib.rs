#[cfg(feature = "from_air")]
mod air;
#[cfg(feature = "from_air")]
pub use air::*;

pub mod fmt;

#[derive(Debug, Clone, Copy)]
pub enum Type<'a> {
    Inferred,
    Void,
    Bool,
    Bit(u32),
    UInt(u32),
    Int(u32),
    Named(&'a str),
    Applied {
        name: &'a str,
        arguments: &'a [Self],
    },
    Vector {
        length: usize,
        element: &'a Self,
    },
    Action,
    ActionValue(&'a Self),
}

#[derive(Debug, Clone, Copy)]
pub struct Package<'a> {
    pub name: &'a str,
    pub imports: &'a [&'a str],
    pub typedefs: &'a [Typedef<'a>],
    pub interfaces: &'a [Interface<'a>],
    pub modules: &'a [Module<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct Field<'a> {
    pub name: &'a str,
    pub ty: Type<'a>,
}
#[derive(Debug, Clone, Copy)]
pub struct Variant<'a> {
    pub name: &'a str,
    pub payload: Option<Type<'a>>,
}
#[derive(Debug, Clone, Copy)]
pub struct Typedef<'a> {
    pub name: &'a str,
    pub kind: TypedefKind<'a>,
    pub deriving: &'a [&'a str],
}
#[derive(Debug, Clone, Copy)]
pub enum TypedefKind<'a> {
    Struct(&'a [Field<'a>]),
    Enum(&'a [&'a str]),
    TaggedUnion(&'a [Variant<'a>]),
    Alias(Type<'a>),
}
#[derive(Debug, Clone, Copy)]
pub struct Interface<'a> {
    pub name: &'a str,
    pub members: &'a [InterfaceMember<'a>],
}
#[derive(Debug, Clone, Copy)]
pub enum InterfaceMember<'a> {
    Method(MethodSignature<'a>),
    Interface { name: &'a str, ty: &'a str },
}
#[derive(Debug, Clone, Copy)]
pub struct MethodSignature<'a> {
    pub name: &'a str,
    pub inputs: &'a [Field<'a>],
    pub output: Type<'a>,
}
#[derive(Debug, Clone, Copy)]
pub struct ClockBinding<'a> {
    pub name: &'a str,
    pub constructor: &'a str,
}
#[derive(Debug, Clone, Copy)]
pub struct Module<'a> {
    pub name: &'a str,
    pub clock_bindings: &'a [ClockBinding<'a>],
    pub parameters: &'a [Parameter<'a>],
    pub interface: &'a str,
    pub registers: &'a [Register<'a>],
    pub instances: &'a [Instance<'a>],
    pub local_interfaces: &'a [LocalInterface<'a>],
    pub rules: &'a [Rule<'a>],
    pub implementation: InterfaceExpr<'a>,
    pub scheduling: &'a [Schedule<'a>],
}
#[derive(Debug, Clone, Copy)]
pub enum Parameter<'a> {
    Value(Field<'a>),
    Interface { name: &'a str, ty: &'a str },
    Clock(&'a str),
}
#[derive(Debug, Clone, Copy)]
pub struct Register<'a> {
    pub name: &'a str,
    pub ty: Type<'a>,
    pub constructor: &'a str,
    pub initial: Option<&'a Expr<'a>>,
    pub clock: Option<&'a str>,
}
#[derive(Debug, Clone, Copy)]
pub struct Instance<'a> {
    pub name: &'a str,
    pub constructor: &'a str,
    pub interface: &'a str,
    pub arguments: &'a [Expr<'a>],
    pub clock: Option<&'a str>,
}
#[derive(Debug, Clone, Copy)]
pub struct LocalInterface<'a> {
    pub name: &'a str,
    pub value: InterfaceExpr<'a>,
}
#[derive(Debug, Clone, Copy)]
pub struct InterfaceExpr<'a> {
    pub ty: &'a str,
    pub members: &'a [Binding<'a>],
}
#[derive(Debug, Clone, Copy)]
pub enum Binding<'a> {
    Method(Method<'a>),
    Interface {
        name: &'a str,
        value: InterfaceExpr<'a>,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct Method<'a> {
    pub signature: MethodSignature<'a>,
    pub guard: Option<&'a Expr<'a>>,
    pub body: MethodBody<'a>,
}
#[derive(Debug, Clone, Copy)]
pub enum MethodBody<'a> {
    Value(&'a Expr<'a>),
    Action(Block<'a>),
    Forward { target: &'a str, action: bool },
}
#[derive(Debug, Clone, Copy)]
pub struct Rule<'a> {
    pub name: &'a str,
    pub guard: Option<&'a Expr<'a>>,
    pub body: Block<'a>,
}
#[derive(Debug, Clone, Copy)]
pub struct Schedule<'a> {
    pub attribute: &'a str,
    pub first: &'a str,
    pub second: &'a str,
}
#[derive(Debug, Clone, Copy)]
pub struct Expr<'a> {
    pub ty: Type<'a>,
    pub kind: ExprKind<'a>,
}
#[derive(Debug, Clone, Copy)]
pub enum ExprKind<'a> {
    Undefined,
    Unit,
    Bool(bool),
    Bits {
        width: u32,
        words_le: &'a [u64],
    },
    Path(&'a str),
    Call {
        function: &'a str,
        arguments: &'a [Expr<'a>],
    },
    Unary {
        operator: &'a str,
        operand: &'a Expr<'a>,
    },
    Binary {
        operator: &'a str,
        lhs: &'a Expr<'a>,
        rhs: &'a Expr<'a>,
    },
    Concat(&'a [Expr<'a>]),
    Annotated(&'a Expr<'a>),
    Conditional {
        condition: &'a Expr<'a>,
        then_value: &'a Expr<'a>,
        else_value: &'a Expr<'a>,
    },
    Struct {
        name: &'a str,
        fields: &'a [FieldValue<'a>],
    },
    Tagged {
        tag: &'a str,
        payload: Option<&'a Expr<'a>>,
    },
    Field {
        value: &'a Expr<'a>,
        field: &'a str,
    },
    Index {
        value: &'a Expr<'a>,
        index: &'a Expr<'a>,
    },
    TagTest {
        value: &'a Expr<'a>,
        tag: &'a str,
        has_payload: bool,
    },
    TagPayload {
        value: &'a Expr<'a>,
        tag: &'a str,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct FieldValue<'a> {
    pub name: &'a str,
    pub value: Expr<'a>,
}
pub type Block<'a> = &'a [Statement<'a>];
#[derive(Debug, Clone, Copy)]
pub enum Statement<'a> {
    Let {
        local: Field<'a>,
        value: Expr<'a>,
    },
    Assign {
        name: &'a str,
        value: Expr<'a>,
    },
    RegisterWrite {
        name: &'a str,
        value: Expr<'a>,
    },
    ActionCall {
        action_value: bool,
        target: &'a str,
        arguments: &'a [Expr<'a>],
        result: Option<Field<'a>>,
    },
    If {
        condition: Expr<'a>,
        then_body: Block<'a>,
        else_body: Option<Block<'a>>,
    },
    Return(Option<Expr<'a>>),
    Assert {
        function: &'a str,
        condition: Expr<'a>,
        message: &'a str,
    },
}
