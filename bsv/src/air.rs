use crate::{
    Binding, Block, ClockBinding, Expr, ExprKind, Field, FieldValue, Instance, Interface,
    InterfaceExpr, InterfaceMember, LocalInterface, Method, MethodBody, MethodSignature, Module,
    Package, Parameter, Register, Rule, Schedule, Statement, Type, Typedef, TypedefKind, Variant,
};
use ::air as source;
use bumpalo::Bump;
use std::{collections::HashSet, error::Error, fmt, ptr, result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerError {
    pub message: String,
}
impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl Error for LowerError {}
type Result<T> = result::Result<T, LowerError>;
fn error<T>(message: impl Into<String>) -> Result<T> {
    Err(LowerError {
        message: message.into(),
    })
}
fn same<T>(a: &T, b: &T) -> bool {
    ptr::eq(a, b)
}
fn unique<'a>(names: impl IntoIterator<Item = &'a str>) -> Result<()> {
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(name) {
            return error(format!("duplicate declaration name {name}"));
        }
    }
    Ok(())
}
fn returns(b: source::Block<'_>) -> bool {
    b.statements.iter().any(|s| match s {
        source::Statement::Return(_) => true,
        source::Statement::If {
            then_body,
            else_body: Some(other),
            ..
        } => returns(*then_body) && returns(*other),
        _ => false,
    })
}

pub fn from_air<'out>(storage: &'out Bump, module: &source::Module<'_>) -> Result<Package<'out>> {
    Lower::new(storage, module).lower()
}

struct Lower<'o, 'm, 's> {
    arena: &'o Bump,
    module: &'m source::Module<'s>,
    typedefs: Vec<Typedef<'o>>,
    interfaces: Vec<Interface<'o>>,
    named: Vec<(&'s source::NamedType<'s>, bool)>,
    ifaces: Vec<(&'s source::Interface<'s>, bool)>,
    tuples: Vec<(&'s source::Type<'s>, &'o str)>,
    imports: Vec<&'o str>,
    implementations: Vec<&'s source::InterfaceImpl<'s>>,
    args: Vec<(&'s source::Argument<'s>, &'o str)>,
    locals: Vec<(&'s source::Local<'s>, &'o str, bool)>,
    declared_locals: Vec<&'s source::Local<'s>>,
    reserved: HashSet<String>,
    next_name: usize,
    visited_exprs: Vec<&'s source::Expr<'s>>,
    visited_refs: Vec<&'s source::InterfaceRef<'s>>,
}
impl<'o, 'm, 's> Lower<'o, 'm, 's> {
    fn new(arena: &'o Bump, module: &'m source::Module<'s>) -> Self {
        Self {
            arena,
            module,
            typedefs: vec![],
            interfaces: vec![],
            named: vec![],
            ifaces: vec![],
            tuples: vec![],
            imports: vec![],
            implementations: vec![],
            args: vec![],
            locals: vec![],
            declared_locals: vec![],
            reserved: HashSet::new(),
            next_name: 0,
            visited_exprs: vec![],
            visited_refs: vec![],
        }
    }
    fn fresh(&mut self) -> &'o str {
        loop {
            let name = format!("bsv_bound_{}", self.next_name);
            self.next_name += 1;
            if self.reserved.insert(name.clone()) {
                return self.text(&name);
            }
        }
    }
    fn bind_local(&mut self, local: &'s source::Local<'s>, unit: bool) -> Result<&'o str> {
        if self.declared_locals.iter().any(|p| same(*p, local)) {
            return error("repeated declaration of local node");
        }
        let name = self.fresh();
        self.declared_locals.push(local);
        self.locals.push((local, name, unit));
        Ok(name)
    }
    fn text(&self, s: &str) -> &'o str {
        self.arena.alloc_str(s)
    }
    fn type_name(&self, name: &str) -> &'o str {
        self.text(&format!("T_{name}"))
    }
    fn interface_name(&self, name: &str) -> &'o str {
        self.text(&format!("I_{name}"))
    }
    fn slice<T: Copy>(&self, v: &[T]) -> &'o [T] {
        self.arena.alloc_slice_copy(v)
    }
    fn alloc<T>(&self, v: T) -> &'o T {
        self.arena.alloc(v)
    }
    fn import(&mut self, s: &'static str) {
        if !self.imports.contains(&s) {
            self.imports.push(s);
        }
    }
    fn ty(&mut self, t: &'s source::Type<'s>) -> Result<Type<'o>> {
        Ok(match t {
            source::Type::Unit => Type::Void,
            source::Type::Bool => Type::Bool,
            source::Type::Bit { width } => Type::Bit(*width),
            source::Type::Uint { width } => Type::UInt(*width),
            source::Type::Sint { width } => Type::Int(*width),
            source::Type::Vector { element, length } => {
                self.import("Vector");
                let element = self.ty(element)?;
                Type::Vector {
                    length: *length,
                    element: self.alloc(element),
                }
            }
            source::Type::Named(n) => {
                self.named(n)?;
                Type::Named(self.type_name(n.name))
            }
            source::Type::Tuple(ts) => match ts.len() {
                0 => Type::Void,
                2..=8 => {
                    let args = ts.iter().map(|t| self.ty(t)).collect::<Result<Vec<_>>>()?;
                    Type::Applied {
                        name: self.text(&format!("Tuple{}", ts.len())),
                        arguments: self.slice(&args),
                    }
                }
                _ => {
                    if let Some((_, name)) = self.tuples.iter().find(|(p, _)| same(*p, t)) {
                        return Ok(Type::Named(name));
                    }
                    let name = self.text(&format!("BsvTuple{}", self.tuples.len()));
                    self.tuples.push((t, name));
                    let fields = ts
                        .iter()
                        .enumerate()
                        .map(|(i, t)| {
                            Ok(Field {
                                name: self.text(&format!("element{i}")),
                                ty: self.ty(t)?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    self.add_typedef(Typedef {
                        name,
                        kind: TypedefKind::Struct(self.slice(&fields)),
                        deriving: &["Bits", "Eq"],
                    })?;
                    Type::Named(name)
                }
            },
        })
    }
    fn add_typedef(&mut self, d: Typedef<'o>) -> Result<()> {
        if self.typedefs.iter().any(|v| v.name == d.name)
            || self.interfaces.iter().any(|v| v.name == d.name)
        {
            return error(format!("duplicate type name {}", d.name));
        }
        self.typedefs.push(d);
        Ok(())
    }
    fn named(&mut self, n: &'s source::NamedType<'s>) -> Result<()> {
        if let Some((_, done)) = self.named.iter().find(|(p, _)| same(*p, n)) {
            return if *done {
                Ok(())
            } else {
                error(format!("cyclic typedef {}", n.name))
            };
        }
        let name = self.type_name(n.name);
        if self
            .named
            .iter()
            .any(|(p, _)| self.type_name(p.name) == name)
        {
            return error(format!("duplicate typedef {}", n.name));
        }
        self.reserved.insert(name.to_owned());
        match n.kind {
            source::NamedTypeKind::Struct(fields) => {
                self.reserved
                    .extend(fields.iter().map(|f| f.name.to_owned()));
            }
            source::NamedTypeKind::Enum(variants) => {
                self.reserved
                    .extend(variants.iter().map(|v| (*v).to_owned()));
            }
            source::NamedTypeKind::TaggedUnion(variants) => {
                self.reserved
                    .extend(variants.iter().map(|v| v.name.to_owned()));
            }
            source::NamedTypeKind::Alias(_) => {}
        }
        let index = self.named.len();
        self.named.push((n, false));
        match n.kind {
            source::NamedTypeKind::Struct(fs) => unique(fs.iter().map(|f| f.name))?,
            source::NamedTypeKind::Enum(vs) => {
                if vs.is_empty() {
                    return error(format!("empty enum {}", n.name));
                }
                unique(vs.iter().copied())?;
            }
            source::NamedTypeKind::TaggedUnion(vs) => {
                if vs.is_empty() {
                    return error(format!("empty tagged union {}", n.name));
                }
                unique(vs.iter().map(|v| v.name))?;
            }
            source::NamedTypeKind::Alias(_) => {}
        }
        let kind = match n.kind {
            source::NamedTypeKind::Struct(fs) => {
                let fs = fs
                    .iter()
                    .map(|f| {
                        Ok(Field {
                            name: self.text(f.name),
                            ty: self.ty(f.ty)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                TypedefKind::Struct(self.slice(&fs))
            }
            source::NamedTypeKind::Enum(vs) => {
                let vs = vs.iter().map(|v| self.text(v)).collect::<Vec<_>>();
                TypedefKind::Enum(self.slice(&vs))
            }
            source::NamedTypeKind::TaggedUnion(vs) => {
                let vs = vs
                    .iter()
                    .map(|v| {
                        Ok(Variant {
                            name: self.text(v.name),
                            payload: v.payload.map(|t| self.ty(t)).transpose()?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                TypedefKind::TaggedUnion(self.slice(&vs))
            }
            source::NamedTypeKind::Alias(t) => TypedefKind::Alias(self.ty(t)?),
        };
        self.add_typedef(Typedef {
            name,
            kind,
            deriving: if matches!(kind, TypedefKind::Alias(_)) {
                &[]
            } else {
                &["Bits", "Eq"]
            },
        })?;
        self.named[index].1 = true;
        Ok(())
    }
    fn resolve_alias(mut ty: &'s source::Type<'s>) -> Result<&'s source::Type<'s>> {
        let mut seen = Vec::new();
        while let source::Type::Named(named) = ty
            && let source::NamedTypeKind::Alias(inner) = named.kind
        {
            if seen.iter().any(|previous| same(*previous, *named)) {
                return error(format!("cyclic alias {}", named.name));
            }
            seen.push(*named);
            ty = inner;
        }
        Ok(ty)
    }
    fn is_unit(&mut self, t: &'s source::Type<'s>) -> Result<bool> {
        self.ty(t)?;
        let ty = self.ty(Self::resolve_alias(t)?)?;
        Ok(matches!(ty, Type::Void))
    }
    fn signature(
        &mut self,
        s: &'s source::MethodSignature<'s>,
        name: Option<&'o str>,
    ) -> Result<MethodSignature<'o>> {
        if name.is_none() {
            unique(s.inputs.iter().map(|a| a.name))?;
        }
        let inputs = s
            .inputs
            .iter()
            .map(|a| {
                Ok(Field {
                    name: if name.is_some() {
                        self.args
                            .iter()
                            .find(|(p, _)| same(*p, a))
                            .map(|(_, name)| *name)
                            .ok_or_else(|| LowerError {
                                message: "undeclared private method argument".into(),
                            })?
                    } else {
                        self.text(a.name)
                    },
                    ty: self.ty(a.ty)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let output = match s.kind {
            source::MethodKind::Value(t) => self.ty(t)?,
            source::MethodKind::Action(t) => {
                if self.is_unit(t)? {
                    Type::Action
                } else {
                    let ty = self.ty(t)?;
                    Type::ActionValue(self.alloc(ty))
                }
            }
        };
        Ok(MethodSignature {
            name: name.unwrap_or_else(|| self.text(s.name)),
            inputs: self.slice(&inputs),
            output,
        })
    }
    fn iface(&mut self, i: &'s source::Interface<'s>) -> Result<&'o str> {
        let name = self.interface_name(i.name);
        if let Some((_, done)) = self.ifaces.iter().find(|(p, _)| same(*p, i)) {
            return if *done {
                Ok(name)
            } else {
                error("cyclic interface")
            };
        }
        if self
            .ifaces
            .iter()
            .any(|(p, _)| self.interface_name(p.name) == name)
        {
            return error(format!("duplicate interface {}", i.name));
        }
        unique(i.members.iter().map(|m| match m {
            source::InterfaceMember::Method(s) => s.name,
            source::InterfaceMember::Interface { name, .. } => name,
        }))?;
        self.reserved.insert(name.to_owned());
        self.reserved.extend(i.members.iter().map(|m| match m {
            source::InterfaceMember::Method(s) => s.name.to_owned(),
            source::InterfaceMember::Interface { name, .. } => (*name).to_owned(),
        }));
        let index = self.ifaces.len();
        self.ifaces.push((i, false));
        let members = i
            .members
            .iter()
            .map(|m| {
                Ok(match m {
                    source::InterfaceMember::Method(s) => {
                        InterfaceMember::Method(self.signature(s, None)?)
                    }
                    source::InterfaceMember::Interface { name, ty } => InterfaceMember::Interface {
                        name: self.text(name),
                        ty: self.iface(ty)?,
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if self.typedefs.iter().any(|v| v.name == name)
            || self.interfaces.iter().any(|v| v.name == name)
        {
            return error("interface/type name collision");
        }
        self.interfaces.push(Interface {
            name,
            members: self.slice(&members),
        });
        self.ifaces[index].1 = true;
        Ok(name)
    }
    fn parameter(&self, p: &source::Parameter<'s>) -> Result<&'o str> {
        if !self.module.signature.parameters.iter().any(|q| same(*q, p)) {
            return error("undeclared parameter");
        }
        Ok(self.text(match p {
            source::Parameter::Value { name, .. } | source::Parameter::Interface { name, .. } => {
                name
            }
            source::Parameter::Clock { domain } => domain.name,
        }))
    }
    fn clock(&self, d: &source::Domain<'s>) -> Result<Option<&'o str>> {
        if !self.module.domains.iter().any(|p| same(*p, d)) {
            return error(format!("undeclared clock domain {}", d.name));
        }
        for p in self.module.signature.parameters {
            if let source::Parameter::Clock { domain } = p
                && same(*domain, d)
            {
                return Ok(Some(self.text(domain.name)));
            }
        }
        let implicit =
            self.module
                .domains
                .iter()
                .filter(|d| {
                    !self.module.signature.parameters.iter().any(
                        |p| matches!(p, source::Parameter::Clock { domain } if same(*domain, **d)),
                    )
                })
                .count();
        if implicit == 1 {
            Ok(None)
        } else {
            error("multiple implicit clock domain identities")
        }
    }
    fn local_name(&self, m: &source::Method<'s>) -> Result<&'o str> {
        self.module
            .methods
            .iter()
            .position(|p| same(*p, m))
            .map(|i| self.text(&format!("bsv_local_{i}")))
            .ok_or_else(|| LowerError {
                message: "undeclared local method".into(),
            })
    }
    fn reference(
        &mut self,
        r: &source::InterfaceRef<'s>,
    ) -> Result<(&'o str, &'s source::Interface<'s>)> {
        match r {
            source::InterfaceRef::Parameter(p) => {
                let name = self.parameter(p)?;
                if let source::Parameter::Interface { ty, .. } = p {
                    Ok((name, ty))
                } else {
                    error("non-interface parameter reference")
                }
            }
            source::InterfaceRef::Instance(i) => {
                if !self.module.instances.iter().any(|p| same(*p, *i)) {
                    return error("undeclared instance");
                }
                Ok((self.text(i.name), i.module.interface))
            }
            source::InterfaceRef::Local(i) => {
                let index = self
                    .implementations
                    .iter()
                    .position(|p| same(*p, *i))
                    .ok_or_else(|| LowerError {
                        message: "undeclared local interface".into(),
                    })?;
                Ok((self.text(&format!("bsv_interface_{index}")), i.ty))
            }
            source::InterfaceRef::Subinterface { parent, member } => {
                let (path, ty) = self.reference(parent)?;
                if !ty.members.iter().any(|m| same(m, *member)) {
                    return error("undeclared subinterface member");
                }
                if let source::InterfaceMember::Interface { name, ty } = member {
                    Ok((
                        self.text(format!("{path}.{name}").trim_start_matches('.')),
                        ty,
                    ))
                } else {
                    error("method used as subinterface")
                }
            }
        }
    }
    fn collect_impl(
        &mut self,
        i: &'s source::InterfaceImpl<'s>,
        stack: &mut Vec<&'s source::InterfaceImpl<'s>>,
    ) -> Result<()> {
        if stack.iter().any(|p| same(*p, i)) {
            return error("cyclic interface implementation");
        }
        if self.implementations.iter().any(|p| same(*p, i)) {
            return Ok(());
        }
        stack.push(i);
        self.implementations.push(i);
        for b in i.members {
            if let source::InterfaceBinding::Interface { implementation, .. } = b {
                self.collect_impl(implementation, stack)?;
            }
        }
        stack.pop();
        for binding in i.members {
            if let source::InterfaceBinding::Method { method, .. } = binding {
                self.discover_method(*method)?;
            }
        }
        Ok(())
    }
    fn discover_signature(&mut self, signature: &'s source::MethodSignature<'s>) -> Result<()> {
        self.reserved.insert(signature.name.to_owned());
        for argument in signature.inputs {
            self.ty(argument.ty)?;
        }
        match signature.kind {
            source::MethodKind::Value(t) | source::MethodKind::Action(t) => {
                self.ty(t)?;
            }
        }
        Ok(())
    }
    fn discover_reference(&mut self, reference: &'s source::InterfaceRef<'s>) -> Result<()> {
        if self.visited_refs.iter().any(|p| same(*p, reference)) {
            return Ok(());
        }
        self.visited_refs.push(reference);
        match reference {
            source::InterfaceRef::Local(i) => self.collect_impl(i, &mut Vec::new())?,
            source::InterfaceRef::Subinterface { parent, .. } => self.discover_reference(parent)?,
            source::InterfaceRef::Parameter(_) | source::InterfaceRef::Instance(_) => {}
        }
        Ok(())
    }
    fn discover_method(&mut self, method: source::MethodRef<'s>) -> Result<()> {
        match method {
            source::MethodRef::Local(m) => self.discover_signature(m.signature)?,
            source::MethodRef::Interface {
                interface,
                signature,
            } => {
                self.discover_signature(signature)?;
                self.discover_reference(interface)?;
            }
        }
        Ok(())
    }
    fn discover_expr(&mut self, expr: &'s source::Expr<'s>) -> Result<()> {
        use source::ExprKind as E;
        if self.visited_exprs.iter().any(|p| same(*p, expr)) {
            return Ok(());
        }
        self.visited_exprs.push(expr);
        self.ty(expr.ty)?;
        match expr.kind {
            E::MethodCall { method, args } => {
                self.discover_method(method)?;
                for expr in args {
                    self.discover_expr(expr)?;
                }
            }
            E::Unary { operand, .. } => self.discover_expr(operand)?,
            E::Cast { value, .. }
            | E::Field { value, .. }
            | E::TupleElement { value, .. }
            | E::IsVariant { value, .. }
            | E::UnionPayload { value, .. } => self.discover_expr(value)?,
            E::Binary { lhs, rhs, .. } => {
                self.discover_expr(lhs)?;
                self.discover_expr(rhs)?;
            }
            E::Mux {
                condition,
                then_value,
                else_value,
            } => {
                self.discover_expr(condition)?;
                self.discover_expr(then_value)?;
                self.discover_expr(else_value)?;
            }
            E::Tuple(values) | E::Vector(values) => {
                for expr in values {
                    self.discover_expr(expr)?;
                }
            }
            E::Struct(fields) => {
                for field in fields {
                    self.discover_expr(field.value)?;
                }
            }
            E::TaggedUnion { payload, .. } => {
                if let Some(expr) = payload {
                    self.discover_expr(expr)?;
                }
            }
            E::Index { value, index } => {
                self.discover_expr(value)?;
                self.discover_expr(index)?;
            }
            E::Slice { value, offset, .. } => {
                self.discover_expr(value)?;
                self.discover_expr(offset)?;
            }
            E::Undefined
            | E::Unit
            | E::Bool(_)
            | E::Bits(_)
            | E::Parameter(_)
            | E::Argument(_)
            | E::Local(_)
            | E::Register(_)
            | E::Enum { .. } => {}
        }
        Ok(())
    }
    fn discover_block(&mut self, block: source::Block<'s>) -> Result<()> {
        for statement in block.statements {
            match *statement {
                source::Statement::Let { local, value }
                | source::Statement::Assign { local, value } => {
                    self.ty(local.ty)?;
                    self.discover_expr(value)?;
                }
                source::Statement::RegisterWrite { value, .. } => self.discover_expr(value)?,
                source::Statement::ActionCall {
                    method,
                    args,
                    result,
                } => {
                    self.discover_method(method)?;
                    for expr in args {
                        self.discover_expr(expr)?;
                    }
                    if let Some(local) = result {
                        self.ty(local.ty)?;
                    }
                }
                source::Statement::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    self.discover_expr(condition)?;
                    self.discover_block(then_body)?;
                    if let Some(block) = else_body {
                        self.discover_block(block)?;
                    }
                }
                source::Statement::Return(value) => {
                    if let Some(expr) = value {
                        self.discover_expr(expr)?;
                    }
                }
                source::Statement::Assert { condition, .. } => self.discover_expr(condition)?,
            }
        }
        Ok(())
    }
    fn discover(&mut self) -> Result<()> {
        let m = self.module;
        self.reserved.insert(m.signature.name.to_owned());
        for ty in m.types {
            self.named(ty)?;
        }
        self.iface(m.signature.interface)?;
        for parameter in m.signature.parameters {
            match parameter {
                source::Parameter::Value { ty, .. } => {
                    self.ty(ty)?;
                }
                source::Parameter::Interface { ty, .. } => {
                    self.iface(ty)?;
                }
                source::Parameter::Clock { .. } => {}
            }
        }
        self.collect_impl(m.interface, &mut Vec::new())?;
        for instance in m.instances {
            self.reserved.insert(instance.module.name.to_owned());
            self.iface(instance.module.interface)?;
            for parameter in instance.module.parameters {
                match parameter {
                    source::Parameter::Value { ty, .. } => {
                        self.ty(ty)?;
                    }
                    source::Parameter::Interface { ty, .. } => {
                        self.iface(ty)?;
                    }
                    source::Parameter::Clock { .. } => {}
                }
            }
            for argument in instance.arguments {
                match argument {
                    source::InstanceArgument::Value { value, .. } => self.discover_expr(value)?,
                    source::InstanceArgument::Interface { interface, .. } => {
                        self.discover_reference(interface)?;
                    }
                    source::InstanceArgument::Clock { .. } => {}
                }
            }
        }
        for register in m.registers {
            self.ty(register.ty)?;
            match register.reset {
                source::RegisterReset::Synchronous(expr)
                | source::RegisterReset::Asynchronous(expr) => self.discover_expr(expr)?,
                source::RegisterReset::Uninitialized => {}
            }
        }
        for method in m.methods {
            self.discover_signature(method.signature)?;
            if let Some(guard) = method.guard {
                self.discover_expr(guard)?;
            }
            match method.body {
                source::MethodBody::Value(expr) => self.discover_expr(expr)?,
                source::MethodBody::Action(block) => self.discover_block(block)?,
            }
        }
        for rule in m.rules {
            if let Some(guard) = rule.guard {
                self.discover_expr(guard)?;
            }
            self.discover_block(rule.body)?;
        }
        for index in 0..self.implementations.len() {
            self.iface(self.implementations[index].ty)?;
        }
        Ok(())
    }
    fn method_ref(
        &mut self,
        m: source::MethodRef<'s>,
    ) -> Result<(&'o str, &'s source::MethodSignature<'s>)> {
        match m {
            source::MethodRef::Local(m) => Ok((
                self.text(&format!("{}.invoke", self.local_name(m)?)),
                m.signature,
            )),
            source::MethodRef::Interface {
                interface,
                signature,
            } => {
                let (path, ty) = self.reference(interface)?;
                if !ty
                    .members
                    .iter()
                    .any(|m| matches!(m, source::InterfaceMember::Method(s) if same(*s, signature)))
                {
                    return error("undeclared interface method");
                }
                Ok((self.text(&format!("{path}.{}", signature.name)), signature))
            }
        }
    }
    fn compatible(&mut self, a: &'s source::Type<'s>, b: &'s source::Type<'s>) -> Result<()> {
        self.ty(a)?;
        self.ty(b)?;
        if same(Self::resolve_alias(a)?, Self::resolve_alias(b)?) {
            Ok(())
        } else {
            error("type mismatch")
        }
    }
    fn call(
        &mut self,
        m: source::MethodRef<'s>,
        args: &'s [&'s source::Expr<'s>],
        action: bool,
    ) -> Result<(&'o str, &'s source::Type<'s>, &'o [Expr<'o>])> {
        let (name, sig) = self.method_ref(m)?;
        let ((source::MethodKind::Action(t), true) | (source::MethodKind::Value(t), false)) =
            (sig.kind, action)
        else {
            return error("incompatible method call kind");
        };
        if sig.inputs.len() != args.len() {
            return error("method argument count mismatch");
        }
        let mut values = Vec::new();
        for (p, v) in sig.inputs.iter().zip(args) {
            self.compatible(p.ty, v.ty)?;
            values.push(self.expr(v)?);
        }
        Ok((name, t, self.slice(&values)))
    }
    fn expr_ref(&mut self, e: &'s source::Expr<'s>) -> Result<&'o Expr<'o>> {
        let v = self.expr(e)?;
        Ok(self.alloc(v))
    }
    fn expr(&mut self, e: &'s source::Expr<'s>) -> Result<Expr<'o>> {
        use source::ExprKind as E;
        let ty = self.ty(e.ty)?;
        let shape = Self::resolve_alias(e.ty)?;
        let kind = match e.kind {
            E::Undefined => ExprKind::Undefined,
            E::Unit => {
                if !self.is_unit(e.ty)? {
                    return error("unit literal type mismatch");
                }
                ExprKind::Unit
            }
            E::Bool(b) => {
                Self::boolean(e)?;
                ExprKind::Bool(b)
            }
            E::Bits(b) => ExprKind::Bits {
                width: b.width,
                words_le: self.slice(b.words_le),
            },
            E::Parameter(p) => {
                let name = self.parameter(p)?;
                if let source::Parameter::Value { ty, .. } = p {
                    self.compatible(e.ty, ty)?;
                } else {
                    return error("non-value parameter expression");
                }
                ExprKind::Path(name)
            }
            E::Argument(a) => {
                let name = self
                    .args
                    .iter()
                    .find(|(p, _)| same(*p, a))
                    .map(|(_, name)| *name)
                    .ok_or_else(|| LowerError {
                        message: "undeclared argument".into(),
                    })?;
                self.compatible(e.ty, a.ty)?;
                ExprKind::Path(name)
            }
            E::Local(l) => {
                let (_, name, unit) = self
                    .locals
                    .iter()
                    .find(|(p, _, _)| same(*p, l))
                    .copied()
                    .ok_or_else(|| LowerError {
                        message: "undeclared local".into(),
                    })?;
                self.compatible(e.ty, l.ty)?;
                if unit {
                    return Ok(Expr {
                        ty: Type::Void,
                        kind: ExprKind::Unit,
                    });
                }
                ExprKind::Path(name)
            }
            E::Register(r) => {
                self.register_ref(r)?;
                self.compatible(e.ty, r.ty)?;
                ExprKind::Path(self.text(r.name))
            }
            E::MethodCall { method, args } => {
                let (function, t, arguments) = self.call(method, args, false)?;
                self.compatible(e.ty, t)?;
                ExprKind::Call {
                    function,
                    arguments,
                }
            }
            E::Unary { op, operand } => ExprKind::Unary {
                operator: match op {
                    source::UnaryOp::Not => "!",
                    source::UnaryOp::Negate => "-",
                    source::UnaryOp::BitNot => "~",
                },
                operand: self.expr_ref(operand)?,
            },
            E::Binary { op, lhs, rhs } => {
                let lhs = self.expr_ref(lhs)?;
                let rhs = self.expr_ref(rhs)?;
                if matches!(op, source::BinaryOp::Concat) {
                    ExprKind::Concat(self.slice(&[*lhs, *rhs]))
                } else {
                    ExprKind::Binary {
                        operator: match op {
                            source::BinaryOp::Add => "+",
                            source::BinaryOp::Subtract => "-",
                            source::BinaryOp::Multiply => "*",
                            source::BinaryOp::Divide => "/",
                            source::BinaryOp::Remainder => "%",
                            source::BinaryOp::BitAnd => "&",
                            source::BinaryOp::BitOr => "|",
                            source::BinaryOp::BitXor => "^",
                            source::BinaryOp::ShiftLeft => "<<",
                            source::BinaryOp::ShiftRight => ">>",
                            source::BinaryOp::Equal => "==",
                            source::BinaryOp::NotEqual => "!=",
                            source::BinaryOp::Less => "<",
                            source::BinaryOp::LessEqual => "<=",
                            source::BinaryOp::Greater => ">",
                            source::BinaryOp::GreaterEqual => ">=",
                            source::BinaryOp::And => "&&",
                            source::BinaryOp::Or => "||",
                            source::BinaryOp::Concat => return error("invalid concat"),
                        },
                        lhs,
                        rhs,
                    }
                }
            }
            E::Cast { kind, value } => {
                let mut value = self.expr(value)?;
                if matches!(kind, source::CastKind::Reinterpret) {
                    value = Expr {
                        ty: Type::Inferred,
                        kind: ExprKind::Call {
                            function: "pack",
                            arguments: self.slice(&[value]),
                        },
                    };
                }
                let function = match kind {
                    source::CastKind::Pack => "pack",
                    source::CastKind::Unpack | source::CastKind::Reinterpret => "unpack",
                    source::CastKind::ZeroExtend => "zeroExtend",
                    source::CastKind::SignExtend => "signExtend",
                    source::CastKind::Truncate => "truncate",
                };
                ExprKind::Annotated(self.alloc(Expr {
                    ty,
                    kind: ExprKind::Call {
                        function,
                        arguments: self.slice(&[value]),
                    },
                }))
            }
            E::Mux {
                condition,
                then_value,
                else_value,
            } => {
                Self::boolean(condition)?;
                self.compatible(e.ty, then_value.ty)?;
                self.compatible(e.ty, else_value.ty)?;
                ExprKind::Conditional {
                    condition: self.expr_ref(condition)?,
                    then_value: self.expr_ref(then_value)?,
                    else_value: self.expr_ref(else_value)?,
                }
            }
            E::Tuple(vs) => {
                let source::Type::Tuple(ts) = shape else {
                    return error("tuple with non-tuple type");
                };
                if ts.len() != vs.len() {
                    return error("tuple length mismatch");
                }
                let values = vs
                    .iter()
                    .zip(*ts)
                    .map(|(v, t)| {
                        self.compatible(v.ty, t)?;
                        self.expr(v)
                    })
                    .collect::<Result<Vec<_>>>()?;
                match values.len() {
                    0 => ExprKind::Unit,
                    2..=8 => ExprKind::Call {
                        function: self.text(&format!("tuple{}", values.len())),
                        arguments: self.slice(&values),
                    },
                    _ => {
                        let Type::Named(name) = ty else {
                            return error("invalid tuple representation");
                        };
                        let fs = values
                            .into_iter()
                            .enumerate()
                            .map(|(i, value)| FieldValue {
                                name: self.text(&format!("element{i}")),
                                value,
                            })
                            .collect::<Vec<_>>();
                        ExprKind::Struct {
                            name,
                            fields: self.slice(&fs),
                        }
                    }
                }
            }
            E::Vector(vs) => {
                self.import("Vector");
                let source::Type::Vector { element, length } = shape else {
                    return error("non-vector construction");
                };
                if *length != vs.len() {
                    return error("vector length mismatch");
                }
                let element_type = self.ty(element)?;
                let element_type = self.alloc(element_type);
                let mut v = Expr {
                    ty: Type::Vector {
                        length: 0,
                        element: element_type,
                    },
                    kind: ExprKind::Call {
                        function: "nil",
                        arguments: &[],
                    },
                };
                for (index, item) in vs.iter().rev().enumerate() {
                    self.compatible(item.ty, element)?;
                    let item = self.expr(item)?;
                    v = Expr {
                        ty: Type::Vector {
                            length: index + 1,
                            element: element_type,
                        },
                        kind: ExprKind::Call {
                            function: "cons",
                            arguments: self.slice(&[item, v]),
                        },
                    };
                }
                v.kind
            }
            E::Struct(fs) => {
                let source::Type::Named(n) = shape else {
                    return error("non-named struct");
                };
                let source::NamedTypeKind::Struct(fields) = n.kind else {
                    return error("non-struct literal");
                };
                if fields.len() != fs.len() {
                    return error("struct field count mismatch");
                }
                let mut values = Vec::new();
                for f in fields {
                    let found = fs.iter().filter(|v| same(v.field, f)).collect::<Vec<_>>();
                    if found.len() != 1 {
                        return error("missing/duplicate struct field");
                    }
                    self.compatible(f.ty, found[0].value.ty)?;
                    values.push(FieldValue {
                        name: self.text(f.name),
                        value: self.expr(found[0].value)?,
                    });
                }
                let Type::Named(name) = ty else {
                    return error("non-named struct representation");
                };
                ExprKind::Struct {
                    name,
                    fields: self.slice(&values),
                }
            }
            E::Enum { variant } => {
                let source::Type::Named(n) = shape else {
                    return error("non-enum literal");
                };
                if !matches!(n.kind, source::NamedTypeKind::Enum(vs) if vs.contains(&variant)) {
                    return error("unknown enum variant");
                }
                ExprKind::Path(self.text(variant))
            }
            E::TaggedUnion { variant, payload } => {
                Self::variant(e.ty, variant)?;
                match (variant.payload, payload) {
                    (Some(t), Some(v)) => self.compatible(t, v.ty)?,
                    (None, None) => {}
                    _ => return error("union payload mismatch"),
                }
                ExprKind::Tagged {
                    tag: self.text(variant.name),
                    payload: payload.map(|v| self.expr_ref(v)).transpose()?,
                }
            }
            E::Field { value, field } => {
                let source::Type::Named(n) = Self::resolve_alias(value.ty)? else {
                    return error("field of non-struct");
                };
                if !matches!(n.kind, source::NamedTypeKind::Struct(fs) if fs.iter().any(|f| same(f,field)))
                {
                    return error("unknown field");
                }
                self.compatible(e.ty, field.ty)?;
                ExprKind::Field {
                    value: self.expr_ref(value)?,
                    field: self.text(field.name),
                }
            }
            E::TupleElement { value, index } => {
                let source::Type::Tuple(ts) = Self::resolve_alias(value.ty)? else {
                    return error("tuple selection on non-tuple");
                };
                let t = ts.get(index).ok_or_else(|| LowerError {
                    message: "tuple index out of bounds".into(),
                })?;
                self.compatible(e.ty, t)?;
                let value = self.expr(value)?;
                if (2..=8).contains(&ts.len()) {
                    ExprKind::Call {
                        function: self.text(&format!("tpl_{}", index + 1)),
                        arguments: self.slice(&[value]),
                    }
                } else {
                    ExprKind::Field {
                        value: self.alloc(value),
                        field: self.text(&format!("element{index}")),
                    }
                }
            }
            E::Index { value, index } => ExprKind::Index {
                value: self.expr_ref(value)?,
                index: self.expr_ref(index)?,
            },
            E::Slice {
                value,
                offset,
                width,
            } => {
                if !matches!(shape, source::Type::Bit { width: w } if *w == width) {
                    return error("slice result type mismatch");
                }
                let shifted = Expr {
                    ty: self.ty(value.ty)?,
                    kind: ExprKind::Binary {
                        operator: ">>",
                        lhs: self.expr_ref(value)?,
                        rhs: self.expr_ref(offset)?,
                    },
                };
                ExprKind::Annotated(self.alloc(Expr {
                    ty,
                    kind: ExprKind::Call {
                        function: "truncate",
                        arguments: self.slice(&[shifted]),
                    },
                }))
            }
            E::IsVariant { value, variant } => {
                Self::boolean(e)?;
                Self::variant(value.ty, variant)?;
                ExprKind::TagTest {
                    value: self.expr_ref(value)?,
                    tag: self.text(variant.name),
                    has_payload: variant.payload.is_some(),
                }
            }
            E::UnionPayload { value, variant } => {
                Self::variant(value.ty, variant)?;
                let t = variant.payload.ok_or_else(|| LowerError {
                    message: "payloadless union variant".into(),
                })?;
                self.compatible(e.ty, t)?;
                ExprKind::TagPayload {
                    value: self.expr_ref(value)?,
                    tag: self.text(variant.name),
                }
            }
        };
        Ok(Expr { ty, kind })
    }
    fn variant(t: &'s source::Type<'s>, v: &source::UnionVariant<'s>) -> Result<()> {
        if matches!(Self::resolve_alias(t)?, source::Type::Named(n) if matches!(n.kind, source::NamedTypeKind::TaggedUnion(vs) if vs.iter().any(|p| same(p,v))))
        {
            Ok(())
        } else {
            error("undeclared union variant")
        }
    }
    fn boolean(e: &source::Expr<'s>) -> Result<()> {
        if matches!(Self::resolve_alias(e.ty)?, source::Type::Bool) {
            Ok(())
        } else {
            error("non-boolean condition")
        }
    }
    fn register_ref(&self, r: &source::Register<'s>) -> Result<()> {
        if self.module.registers.iter().any(|p| same(*p, r)) {
            Ok(())
        } else {
            error("undeclared register")
        }
    }
    fn block(&mut self, b: source::Block<'s>, result: &'s source::Type<'s>) -> Result<Block<'o>> {
        let scope = self.locals.len();
        let mut out = Vec::new();
        for s in b.statements {
            let statement = match *s {
                source::Statement::Let { local, value } => {
                    self.compatible(local.ty, value.ty)?;
                    let value = self.expr(value)?;
                    let name = self.bind_local(local, false)?;
                    Statement::Let {
                        local: Field {
                            name,
                            ty: self.ty(local.ty)?,
                        },
                        value,
                    }
                }
                source::Statement::Assign { local, value } => {
                    let (_, name, unit) = self
                        .locals
                        .iter()
                        .find(|(p, _, _)| same(*p, local))
                        .copied()
                        .ok_or_else(|| LowerError {
                            message: "assignment to undeclared local".into(),
                        })?;
                    self.compatible(local.ty, value.ty)?;
                    let value = self.expr(value)?;
                    if unit {
                        Statement::Let {
                            local: Field {
                                name: "_",
                                ty: Type::Inferred,
                            },
                            value,
                        }
                    } else {
                        Statement::Assign { name, value }
                    }
                }
                source::Statement::RegisterWrite { register, value } => {
                    self.register_ref(register)?;
                    self.compatible(register.ty, value.ty)?;
                    Statement::RegisterWrite {
                        name: self.text(register.name),
                        value: self.expr(value)?,
                    }
                }
                source::Statement::ActionCall {
                    method,
                    args,
                    result,
                } => {
                    let (target, t, arguments) = self.call(method, args, true)?;
                    let result = if let Some(l) = result {
                        self.compatible(l.ty, t)?;
                        let unit = self.is_unit(t)?;
                        let name = self.bind_local(l, unit)?;
                        if unit {
                            None
                        } else {
                            Some(Field {
                                name,
                                ty: self.ty(l.ty)?,
                            })
                        }
                    } else {
                        None
                    };
                    Statement::ActionCall {
                        action_value: !self.is_unit(t)?,
                        target,
                        arguments,
                        result,
                    }
                }
                source::Statement::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    Self::boolean(condition)?;
                    Statement::If {
                        condition: self.expr(condition)?,
                        then_body: self.block(then_body, result)?,
                        else_body: else_body.map(|b| self.block(b, result)).transpose()?,
                    }
                }
                source::Statement::Return(value) => {
                    match value {
                        Some(v) => self.compatible(result, v.ty)?,
                        None if !self.is_unit(result)? => {
                            return error("missing return value");
                        }
                        None => {}
                    }
                    let value = value.map(|v| self.expr(v)).transpose()?;
                    if self.is_unit(result)? {
                        if let Some(value) = value
                            && !matches!(value.kind, ExprKind::Unit | ExprKind::Undefined)
                        {
                            out.push(Statement::Let {
                                local: Field {
                                    name: "_",
                                    ty: Type::Inferred,
                                },
                                value,
                            });
                        }
                        Statement::Return(None)
                    } else {
                        Statement::Return(value)
                    }
                }
                source::Statement::Assert { condition, message } => {
                    Self::boolean(condition)?;
                    self.import("Assert");
                    Statement::Assert {
                        function: "dynamicAssert",
                        condition: self.expr(condition)?,
                        message: self.text(message),
                    }
                }
            };
            out.push(statement);
        }
        self.locals.truncate(scope);
        Ok(self.slice(&out))
    }
    fn implementation(
        &mut self,
        i: &'s source::InterfaceImpl<'s>,
        stack: &mut Vec<&'s source::InterfaceImpl<'s>>,
    ) -> Result<InterfaceExpr<'o>> {
        if stack.iter().any(|p| same(*p, i)) {
            return error("cyclic interface implementation");
        }
        stack.push(i);
        let ty = self.iface(i.ty)?;
        let mut members = Vec::new();
        if i.members.len() != i.ty.members.len() {
            return error("interface binding count mismatch");
        }
        for declaration in i.ty.members {
            match declaration {
                source::InterfaceMember::Method(s) => {
                    let bs = i
                        .members
                        .iter()
                        .filter_map(|b| match b {
                            source::InterfaceBinding::Method {
                                declaration,
                                method,
                            } if same(*declaration, *s) => Some(*method),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    if bs.len() != 1 {
                        return error("missing/duplicate method binding");
                    }
                    let (target, actual) = self.method_ref(bs[0])?;
                    let mut signature = self.signature(s, None)?;
                    let inputs = signature
                        .inputs
                        .iter()
                        .map(|input| Field {
                            name: self.fresh(),
                            ty: input.ty,
                        })
                        .collect::<Vec<_>>();
                    signature.inputs = self.slice(&inputs);
                    match (s.kind, actual.kind) {
                        (source::MethodKind::Value(expected), source::MethodKind::Value(found))
                        | (
                            source::MethodKind::Action(expected),
                            source::MethodKind::Action(found),
                        ) => {
                            self.compatible(expected, found)?;
                        }
                        _ => return error("incompatible forwarded method kind"),
                    }
                    if s.inputs.len() != actual.inputs.len() {
                        return error("incompatible forwarded method argument count");
                    }
                    for (expected, found) in s.inputs.iter().zip(actual.inputs) {
                        self.compatible(expected.ty, found.ty)?;
                    }
                    members.push(Binding::Method(Method {
                        signature,
                        guard: None,
                        body: MethodBody::Forward {
                            target,
                            action: matches!(s.kind, source::MethodKind::Action(_)),
                        },
                    }));
                }
                source::InterfaceMember::Interface { name, ty } => {
                    let bs = i
                        .members
                        .iter()
                        .filter_map(|b| match b {
                            source::InterfaceBinding::Interface {
                                declaration: d,
                                implementation,
                            } if same(*d, declaration) => Some(*implementation),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    if bs.len() != 1 || !same(bs[0].ty, *ty) {
                        return error("incompatible nested interface binding");
                    }
                    let value = self.implementation(bs[0], stack)?;
                    members.push(Binding::Interface {
                        name: self.text(name),
                        value,
                    });
                }
            }
        }
        stack.pop();
        Ok(InterfaceExpr {
            ty,
            members: self.slice(&members),
        })
    }
    fn schedule_target(&self, t: source::ScheduleTarget<'s>) -> Result<&'o str> {
        match t {
            source::ScheduleTarget::Method(m) => {
                Ok(self.text(&format!("{}.invoke", self.local_name(m)?)))
            }
            source::ScheduleTarget::Rule(r) => {
                if self.module.rules.iter().any(|p| same(*p, r)) {
                    Ok(self.text(r.name))
                } else {
                    error("undeclared scheduling rule")
                }
            }
        }
    }
    fn lower(mut self) -> Result<Package<'o>> {
        let m = self.module;
        for (index, domain) in m.domains.iter().enumerate() {
            if m.domains[..index].iter().any(|p| same(*p, *domain)) {
                return error("duplicate clock domain identity");
            }
            self.clock(domain)?;
        }
        for p in m.signature.parameters {
            if let source::Parameter::Clock { domain } = p {
                self.clock(domain)?;
            }
        }
        self.discover()?;
        let mut names = vec!["bsv_default_clock"];
        for p in m.signature.parameters {
            names.push(match p {
                source::Parameter::Value { name, .. }
                | source::Parameter::Interface { name, .. } => name,
                source::Parameter::Clock { domain } => domain.name,
            });
        }
        names.extend(m.registers.iter().map(|r| r.name));
        names.extend(m.instances.iter().map(|i| i.name));
        names.extend(m.rules.iter().map(|r| r.name));
        let generated = (0..m.methods.len())
            .map(|i| format!("bsv_local_{i}"))
            .chain((0..self.implementations.len()).map(|i| format!("bsv_interface_{i}")))
            .collect::<Vec<_>>();
        names.extend(generated.iter().map(String::as_str));
        unique(names.iter().copied())?;
        self.reserved.extend(names.into_iter().map(str::to_owned));
        unique(m.methods.iter().map(|m| m.signature.name))?;
        for t in m.types {
            self.named(t)?;
        }
        let interface = self.iface(m.signature.interface)?;
        if !same(m.interface.ty, m.signature.interface) {
            return error("module interface mismatch");
        }
        let mut parameters = Vec::new();
        for p in m.signature.parameters {
            parameters.push(match p {
                source::Parameter::Value { name, ty } => Parameter::Value(Field {
                    name: self.text(name),
                    ty: self.ty(ty)?,
                }),
                source::Parameter::Interface { name, ty } => Parameter::Interface {
                    name: self.text(name),
                    ty: self.iface(ty)?,
                },
                source::Parameter::Clock { domain } => Parameter::Clock(self.text(domain.name)),
            });
        }
        let mut registers = Vec::new();
        for r in m.registers {
            let (constructor, initial) = match r.reset {
                source::RegisterReset::Uninitialized => ("mkRegU", None),
                source::RegisterReset::Synchronous(e) => {
                    self.compatible(r.ty, e.ty)?;
                    ("mkReg", Some(self.expr_ref(e)?))
                }
                source::RegisterReset::Asynchronous(e) => {
                    self.compatible(r.ty, e.ty)?;
                    ("mkRegA", Some(self.expr_ref(e)?))
                }
            };
            registers.push(Register {
                name: self.text(r.name),
                ty: self.ty(r.ty)?,
                constructor,
                initial,
                clock: self.clock(r.domain)?,
            });
        }
        let mut instances = Vec::new();
        for i in m.instances {
            let interface = self.iface(i.module.interface)?;
            let mut arguments = Vec::new();
            if i.arguments.len() != i.module.parameters.len() {
                return error("instance argument count mismatch");
            }
            for p in i.module.parameters {
                let matches = i
                    .arguments
                    .iter()
                    .filter(|a| {
                        same(
                            match a {
                                source::InstanceArgument::Value { parameter, .. }
                                | source::InstanceArgument::Interface { parameter, .. }
                                | source::InstanceArgument::Clock { parameter, .. } => *parameter,
                            },
                            *p,
                        )
                    })
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    return error("missing/duplicate instance argument");
                }
                arguments.push(match (p, matches[0]) {
                    (
                        source::Parameter::Value { ty, .. },
                        source::InstanceArgument::Value { value, .. },
                    ) => {
                        self.compatible(ty, value.ty)?;
                        self.expr(value)?
                    }
                    (
                        source::Parameter::Interface { ty, .. },
                        source::InstanceArgument::Interface { interface, .. },
                    ) => {
                        let (path, actual) = self.reference(interface)?;
                        if !same(*ty, actual) {
                            return error("instance interface type mismatch");
                        }
                        Expr {
                            ty: Type::Named(self.iface(ty)?),
                            kind: ExprKind::Path(path),
                        }
                    }
                    (
                        source::Parameter::Clock { .. },
                        source::InstanceArgument::Clock { domain, .. },
                    ) => {
                        let path = self.clock(domain)?.unwrap_or("bsv_default_clock");
                        Expr {
                            ty: Type::Named("Clock"),
                            kind: ExprKind::Path(path),
                        }
                    }
                    _ => return error("instance argument kind mismatch"),
                });
            }
            instances.push(Instance {
                name: self.text(i.name),
                constructor: self.text(i.module.name),
                interface,
                arguments: self.slice(&arguments),
                clock: self.clock(i.domain)?,
            });
        }
        let mut local_interfaces = Vec::new();
        for (index, method) in m.methods.iter().enumerate() {
            self.args.clear();
            self.locals.clear();
            self.declared_locals.clear();
            for argument in method.signature.inputs {
                if self.args.iter().any(|(p, _)| same(*p, argument)) {
                    return error("repeated argument node");
                }
                let name = self.fresh();
                self.args.push((argument, name));
            }
            let signature = self.signature(method.signature, Some("invoke"))?;
            let name = self.text(&format!("BsvLocalMethod{index}"));
            if self.interfaces.iter().any(|i| i.name == name)
                || self.typedefs.iter().any(|t| t.name == name)
            {
                return error("generated method interface name collision");
            }
            self.interfaces.push(Interface {
                name,
                members: self.slice(&[InterfaceMember::Method(signature)]),
            });
            let guard = if let Some(g) = method.guard {
                Self::boolean(g)?;
                Some(self.expr_ref(g)?)
            } else {
                None
            };
            let body = match (method.signature.kind, method.body) {
                (source::MethodKind::Value(t), source::MethodBody::Value(e)) => {
                    self.compatible(t, e.ty)?;
                    MethodBody::Value(self.expr_ref(e)?)
                }
                (source::MethodKind::Action(t), source::MethodBody::Action(b)) => {
                    if !self.is_unit(t)? && !returns(b) {
                        return error("action-value method does not return on every path");
                    }
                    MethodBody::Action(self.block(b, t)?)
                }
                _ => return error("method kind/body mismatch"),
            };
            local_interfaces.push(LocalInterface {
                name: self.local_name(method)?,
                value: InterfaceExpr {
                    ty: name,
                    members: self.slice(&[Binding::Method(Method {
                        signature,
                        guard,
                        body,
                    })]),
                },
            });
        }
        self.args.clear();
        self.locals.clear();
        let mut rules = Vec::new();
        for r in m.rules {
            self.locals.clear();
            self.declared_locals.clear();
            let guard = if let Some(g) = r.guard {
                Self::boolean(g)?;
                Some(self.expr_ref(g)?)
            } else {
                None
            };
            let body = self.block(r.body, &source::Type::Unit)?;
            rules.push(Rule {
                name: self.text(r.name),
                guard,
                body,
            });
        }
        let mut index = 0;
        while index < self.implementations.len() {
            let i = self.implementations[index];
            let value = self.implementation(i, &mut Vec::new())?;
            local_interfaces.push(LocalInterface {
                name: self.text(&format!("bsv_interface_{index}")),
                value,
            });
            index += 1;
        }
        let implementation = self.implementation(m.interface, &mut Vec::new())?;
        let mut scheduling = Vec::new();
        for s in m.scheduling {
            let (attribute, a, b) = match *s {
                source::ScheduleConstraint::Urgency { higher, lower } => {
                    ("descending_urgency", higher, lower)
                }
                source::ScheduleConstraint::ExecutionOrder { before, after } => {
                    ("execution_order", before, after)
                }
                source::ScheduleConstraint::MutualExclusion { first, second } => {
                    ("mutually_exclusive", first, second)
                }
                source::ScheduleConstraint::ConflictFree { first, second } => {
                    ("conflict_free", first, second)
                }
                source::ScheduleConstraint::Preemption {
                    preemptor,
                    preempted,
                } => ("preempts", preemptor, preempted),
            };
            scheduling.push(Schedule {
                attribute,
                first: self.schedule_target(a)?,
                second: self.schedule_target(b)?,
            });
        }
        let root = Module {
            name: self.text(m.signature.name),
            clock_bindings: self.slice(&[ClockBinding {
                name: "bsv_default_clock",
                constructor: "exposeCurrentClock",
            }]),
            parameters: self.slice(&parameters),
            interface,
            registers: self.slice(&registers),
            instances: self.slice(&instances),
            local_interfaces: self.slice(&local_interfaces),
            rules: self.slice(&rules),
            implementation,
            scheduling: self.slice(&scheduling),
        };
        Ok(Package {
            name: self.text(m.signature.name),
            imports: self.slice(&self.imports),
            typedefs: self.slice(&self.typedefs),
            interfaces: self.slice(&self.interfaces),
            modules: self.slice(&[root]),
        })
    }
}
