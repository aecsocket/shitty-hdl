#[derive(Debug, Clone)]
pub struct Interface<'dsn> {
    pub name: &'dsn str,
    pub members: &'dsn [InterfaceMember<'dsn>],
}

#[derive(Debug, Clone)]
pub enum InterfaceMember<'dsn> {
    Method(&'dsn MethodSignature<'dsn>),
    Interface {
        name: &'dsn str,
        ty: &'dsn Interface<'dsn>,
    },
}

#[derive(Debug, Clone)]
pub struct MethodSignature<'dsn> {
    pub name: &'dsn str,
    pub inputs: &'dsn [Argument<'dsn>],
    pub kind: MethodKind<'dsn>,
}

#[derive(Debug, Clone)]
pub struct Argument<'dsn> {
    pub name: &'dsn str,
    pub ty: &'dsn Type<'dsn>,
}

#[derive(Debug, Clone, Copy)]
pub enum MethodKind<'dsn> {
    Value(&'dsn Type<'dsn>),
    Action(&'dsn Type<'dsn>),
}

#[derive(Debug, Clone)]
pub enum Type<'dsn> {
    Unit,
    Bool,
    Bit { width: u32 },
    Uint { width: u32 },
    Sint { width: u32 },
    Tuple(&'dsn [&'dsn Self]),
    Vector { element: &'dsn Self, length: usize },
    Named(&'dsn NamedType<'dsn>),
}

#[derive(Debug, Clone)]
pub struct NamedType<'dsn> {
    pub name: &'dsn str,
    pub kind: NamedTypeKind<'dsn>,
}

#[derive(Debug, Clone)]
pub enum NamedTypeKind<'dsn> {
    Struct(&'dsn [Field<'dsn>]),
    Enum(&'dsn [&'dsn str]),
    TaggedUnion(&'dsn [UnionVariant<'dsn>]),
    Alias(&'dsn Type<'dsn>),
}

#[derive(Debug, Clone)]
pub struct Field<'dsn> {
    pub name: &'dsn str,
    pub ty: &'dsn Type<'dsn>,
}

#[derive(Debug, Clone)]
pub struct UnionVariant<'dsn> {
    pub name: &'dsn str,
    pub payload: Option<&'dsn Type<'dsn>>,
}
