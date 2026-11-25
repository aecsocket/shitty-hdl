use std::mem::{swap, take};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::ptr;
use std::{any::TypeId, collections::HashMap};

use bumpalo::Bump;

use super::{ActionHandle, BranchHandle, DesignId, ExprHandle, InterfaceHandle, LocalHandle};
use crate::value::Shape;

pub struct Frame<'dsn> {
    pub scope: usize,
    pub statements: Vec<air::Statement<'dsn>>,
}

pub struct Branch<'dsn> {
    pub module: usize,
    pub parent_scope: usize,
    pub block: air::Block<'dsn>,
    pub result: &'dsn air::Expr<'dsn>,
}

/// How the body currently being elaborated reports its completion.
#[derive(Debug, Clone, Copy)]
pub enum ReturnTarget {
    /// Async rule/method bodies: write the result (if any) and jump to `done`.
    State {
        done: usize,
        result: Option<super::RegisterHandle>,
        /// Rule bodies discard a returned value instead of typing it.
        discard: bool,
    },
    /// Combinational action bodies: assign the action's output local.
    Local(LocalHandle),
    /// Bodies with nothing to deliver (unit outputs).
    Unit,
}

pub struct ModuleState<'dsn> {
    pub(crate) root_active: bool,
    pub(crate) root_defined: bool,
    pub(crate) definitions: Vec<&'dsn air::MethodSignature<'dsn>>,
    name: &'dsn str,
    pub(crate) parameters: Vec<&'dsn air::Parameter<'dsn>>,
    instances: Vec<&'dsn air::Instance<'dsn>>,
    pub(crate) registers: Vec<&'dsn air::Register<'dsn>>,
    pub(crate) methods: Vec<&'dsn air::Method<'dsn>>,
    pub(crate) rules: Vec<&'dsn air::Rule<'dsn>>,
    interface: Option<&'dsn air::InterfaceImpl<'dsn>>,
    scheduling: Vec<air::ScheduleConstraint<'dsn>>,
    modules: Vec<&'dsn air::Module<'dsn>>,
    arguments: Vec<air::InstanceArgument<'dsn>>,
    imports: HashMap<(u8, usize), usize>,
    id: usize,
}

pub struct Context<'dsn> {
    modules: Vec<&'dsn air::Module<'dsn>>,
    arguments: Vec<air::InstanceArgument<'dsn>>,
    imports: HashMap<(u8, usize), usize>,
    pub(crate) id: usize,
    next_module: usize,
    parents: Vec<ModuleState<'dsn>>,
    expression_owners: Vec<usize>,
    action_owners: Vec<usize>,
    local_action_interfaces:
        HashMap<usize, (&'dsn air::InterfaceRef<'dsn>, &'dsn air::Interface<'dsn>)>,
    interface_owners: Vec<usize>,
    pub(crate) outputs: Vec<&'dsn air::Output<'dsn>>,
    pub(crate) register_handles: Vec<(&'dsn air::Register<'dsn>, usize)>,
    pub(crate) arena: &'dsn Bump,
    pub(crate) design_id: DesignId,
    pub(crate) root_active: bool,
    pub(crate) root_defined: bool,
    pub(crate) definitions: Vec<&'dsn air::MethodSignature<'dsn>>,
    name: &'dsn str,
    pub(crate) definition_path: Option<&'dsn str>,
    pub(crate) types: Vec<&'dsn air::NamedType<'dsn>>,
    pub(crate) type_cache: HashMap<TypeId, &'dsn air::Type<'dsn>>,
    pub(crate) iface_cache: HashMap<TypeId, &'dsn air::Interface<'dsn>>,
    pub(crate) parameters: Vec<&'dsn air::Parameter<'dsn>>,
    instances: Vec<&'dsn air::Instance<'dsn>>,
    pub(crate) registers: Vec<&'dsn air::Register<'dsn>>,
    pub(crate) methods: Vec<&'dsn air::Method<'dsn>>,
    pub(crate) rules: Vec<&'dsn air::Rule<'dsn>>,
    interface: Option<&'dsn air::InterfaceImpl<'dsn>>,
    scheduling: Vec<air::ScheduleConstraint<'dsn>>,
    pub(crate) expressions: Vec<(&'dsn air::Expr<'dsn>, usize)>,
    pub(crate) actions: Vec<air::MethodRef<'dsn>>,
    pub(crate) interface_refs: Vec<(&'dsn air::InterfaceRef<'dsn>, &'dsn air::Interface<'dsn>)>,
    pub(crate) domains: Vec<&'dsn air::Domain<'dsn>>,
    pub(crate) domain_cache: HashMap<TypeId, (usize, &'dsn air::Domain<'dsn>)>,
    pub(crate) frames: Vec<Frame<'dsn>>,
    scope_parents: Vec<usize>,
    pub(crate) branches: Vec<Branch<'dsn>>,
    pub(crate) async_flow: Option<super::fsm::Flow>,
    pub(crate) return_target: Option<ReturnTarget>,
    pub(crate) locals: Vec<&'dsn air::Local<'dsn>>,
}

macro_rules! record {
    ($method:ident, $field:ident, $ty:ident) => {
        pub(crate) fn $method(&mut self, node: air::$ty<'dsn>) -> &'dsn air::$ty<'dsn> {
            let node = self.alloc(node);
            self.$field.push(node);
            node
        }
    };
}

impl<'dsn> Context<'dsn> {
    pub(crate) fn new(arena: &'dsn Bump, design_id: DesignId) -> Self {
        Self {
            modules: Vec::new(),
            arguments: Vec::new(),
            imports: HashMap::new(),
            id: 0,
            next_module: 1,
            parents: Vec::new(),
            expression_owners: Vec::new(),
            action_owners: Vec::new(),
            local_action_interfaces: HashMap::new(),
            interface_owners: Vec::new(),
            outputs: Vec::new(),
            register_handles: Vec::new(),
            arena,
            design_id,
            root_active: false,
            root_defined: false,
            definitions: Vec::new(),
            name: "mkTop",
            definition_path: None,
            types: Vec::new(),
            type_cache: HashMap::new(),
            iface_cache: HashMap::new(),
            parameters: Vec::new(),
            instances: Vec::new(),
            registers: Vec::new(),
            methods: Vec::new(),
            rules: Vec::new(),
            interface: None,
            scheduling: Vec::new(),
            expressions: Vec::new(),
            actions: Vec::new(),
            interface_refs: Vec::new(),
            domains: Vec::new(),
            domain_cache: HashMap::new(),
            frames: Vec::new(),
            scope_parents: vec![0],
            branches: Vec::new(),
            async_flow: None,
            return_target: None,
            locals: Vec::new(),
        }
    }

    pub fn alloc<T: 'dsn>(&self, value: T) -> &'dsn T {
        self.arena.alloc(value)
    }

    pub fn alloc_slice<T: 'dsn, I>(&self, values: I) -> &'dsn [T]
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.arena.alloc_slice_fill_iter(values)
    }

    #[must_use]
    pub fn alloc_str(&self, value: &str) -> &'dsn str {
        self.arena.alloc_str(value)
    }

    pub(crate) fn intern_domain(
        &mut self,
        identity: TypeId,
        name: &str,
    ) -> (usize, &'dsn air::Domain<'dsn>) {
        if let Some(&(index, domain)) = self.domain_cache.get(&identity) {
            assert_eq!(
                domain.name, name,
                "one domain TypeId cannot have multiple names"
            );
            return (index, domain);
        }
        let domain = self.alloc(air::Domain {
            name: self.alloc_str(name),
        });
        let index = self.domains.len();
        self.domains.push(domain);
        self.domain_cache.insert(identity, (index, domain));
        (index, domain)
    }

    pub(crate) fn set_name(&mut self, name: &str) {
        self.name = self.alloc_str(name);
    }

    record!(add_type, types, NamedType);
    record!(add_parameter, parameters, Parameter);
    record!(add_instance, instances, Instance);
    record!(add_register, registers, Register);
    record!(add_method, methods, Method);
    record!(add_rule, rules, Rule);

    pub(crate) fn set_interface(&mut self, interface: air::InterfaceImpl<'dsn>) {
        assert!(
            self.interface.is_none(),
            "module interface is already defined; elaborate one root module at a time"
        );
        self.interface = Some(self.alloc(interface));
    }

    pub(crate) fn check_design(&self, design_id: DesignId) {
        assert_eq!(
            design_id, self.design_id,
            "HDL handle belongs to a different design"
        );
    }

    pub(crate) fn push_expr(&mut self, expr: &'dsn air::Expr<'dsn>) -> ExprHandle {
        let handle = ExprHandle {
            design_id: self.design_id,
            index: self.expressions.len(),
        };
        self.expression_owners.push(self.id);
        self.expressions.push((expr, self.current_scope()));
        handle
    }

    pub(crate) fn push_local(&mut self, local: &'dsn air::Local<'dsn>) -> LocalHandle {
        let handle = LocalHandle {
            design_id: self.design_id,
            index: self.locals.len(),
        };
        self.locals.push(local);
        handle
    }

    pub(crate) fn local(&self, handle: LocalHandle) -> &'dsn air::Local<'dsn> {
        self.check_design(handle.design_id);
        self.locals[handle.index]
    }

    pub(crate) fn expr(&mut self, handle: ExprHandle) -> &'dsn air::Expr<'dsn> {
        self.check_design(handle.design_id);
        let (expr, mut scope) = self.expressions[handle.index];
        if self.expression_owners[handle.index] != self.id {
            assert_eq!(
                scope, 0,
                "cannot capture an action-scoped expression in another module"
            );
            if let Some(&index) = self.imports.get(&(0, handle.index)) {
                return self.expressions[index].0;
            }
            let value = self.in_parent(|cx| cx.expr(handle));
            let parameter = self.add_parameter(air::Parameter::Value {
                name: self.capture_name(),
                ty: value.ty,
            });
            self.arguments
                .push(air::InstanceArgument::Value { parameter, value });
            let expr = self.alloc(air::Expr {
                ty: value.ty,
                kind: air::ExprKind::Parameter(parameter),
            });
            let imported = self.push_expr(expr);
            self.expressions[imported.index].1 = 0;
            self.imports.insert((0, handle.index), imported.index);
            return expr;
        }
        let target = scope;
        scope = self.current_scope();
        while scope != 0 && scope != target {
            scope = self.scope_parents[scope];
        }
        assert!(
            target == 0 || scope == target,
            "HDL expression escaped its action or branch scope (target={target}, current={})",
            self.current_scope(),
        );
        expr
    }

    pub(crate) fn typed_expr<T: Shape>(&mut self, handle: ExprHandle) -> &'dsn air::Expr<'dsn> {
        let expr = self.expr(handle);
        let ty = self.ty::<T>();
        assert!(ptr::eq(expr.ty, ty), "HDL expression type mismatch");
        expr
    }

    #[must_use]
    pub fn action_ref(&mut self, handle: ActionHandle) -> air::MethodRef<'dsn> {
        self.check_design(handle.design_id);
        if self.action_owners[handle.index] == self.id {
            return self.actions[handle.index];
        }
        if let Some(&index) = self.imports.get(&(1, handle.index)) {
            return self.actions[index];
        }
        let (interface, signature, ty) = self.in_parent(|cx| {
            let method = cx.action_ref(handle);
            let signature = match method {
                air::MethodRef::Local(method) => method.signature,
                air::MethodRef::Interface {
                    interface,
                    signature,
                } => {
                    return (interface, signature, Self::interface_type(interface));
                }
            };
            if let Some(&(interface, ty)) = cx.local_action_interfaces.get(&handle.index) {
                return (interface, signature, ty);
            }
            let ty = cx.alloc(air::Interface {
                name: cx.alloc_str(&format!("CapturedMethod_{}", handle.index)),
                members: cx.alloc_slice([air::InterfaceMember::Method(signature)]),
            });
            let implementation = cx.alloc(air::InterfaceImpl {
                ty,
                members: cx.alloc_slice([air::InterfaceBinding::Method {
                    declaration: signature,
                    method,
                }]),
            });
            let interface = cx.alloc(air::InterfaceRef::Local(implementation));
            cx.local_action_interfaces
                .insert(handle.index, (interface, ty));
            (interface, signature, ty)
        });
        let interface = self.import_interface(interface, ty);
        let method = air::MethodRef::Interface {
            interface,
            signature,
        };
        let imported = self.push_action(method);
        self.imports.insert((1, handle.index), imported.index);
        method
    }

    pub(crate) fn push_action(&mut self, method: air::MethodRef<'dsn>) -> ActionHandle {
        let handle = ActionHandle {
            design_id: self.design_id,
            index: self.actions.len(),
        };
        self.action_owners.push(self.id);
        self.actions.push(method);
        handle
    }

    pub(crate) fn push_interface(
        &mut self,
        interface: &'dsn air::InterfaceRef<'dsn>,
        ty: &'dsn air::Interface<'dsn>,
    ) -> InterfaceHandle {
        let handle = InterfaceHandle {
            design_id: self.design_id,
            index: self.interface_refs.len(),
        };
        self.interface_owners.push(self.id);
        self.interface_refs.push((interface, ty));
        handle
    }

    pub(crate) fn interface_ref(
        &mut self,
        handle: InterfaceHandle,
    ) -> (&'dsn air::InterfaceRef<'dsn>, &'dsn air::Interface<'dsn>) {
        self.check_design(handle.design_id);
        if self.interface_owners[handle.index] == self.id {
            return self.interface_refs[handle.index];
        }
        if let Some(&index) = self.imports.get(&(2, handle.index)) {
            return self.interface_refs[index];
        }
        let (interface, ty) = self.in_parent(|cx| cx.interface_ref(handle));
        let interface = self.import_interface(interface, ty);
        let imported = self.push_interface(interface, ty);
        self.imports.insert((2, handle.index), imported.index);
        (interface, ty)
    }

    pub(crate) fn branch(&self, handle: BranchHandle) -> &Branch<'dsn> {
        self.check_design(handle.design_id);
        let branch = &self.branches[handle.index];
        assert_eq!(
            branch.module, self.id,
            "conditional builder escaped its defining module"
        );
        assert_eq!(
            branch.parent_scope,
            self.current_scope(),
            "conditional builder escaped its defining body"
        );
        branch
    }

    pub(crate) fn current_scope(&self) -> usize {
        self.frames.last().map_or(0, |frame| frame.scope)
    }

    pub(crate) fn begin_body(&mut self) {
        let scope = self.scope_parents.len();
        self.scope_parents.push(self.current_scope());
        self.frames.push(Frame {
            scope,
            statements: Vec::new(),
        });
    }

    pub(crate) fn emit(&mut self, statement: air::Statement<'dsn>) {
        self.frames
            .last_mut()
            .expect("stateful HDL operations require an action or rule body")
            .statements
            .push(statement);
    }

    pub(crate) fn end_body(&mut self) -> air::Block<'dsn> {
        let statements = self.frames.pop().expect("missing action body").statements;
        air::Block {
            statements: self.alloc_slice(statements),
        }
    }

    pub(crate) fn record_register_handle(&mut self, register: &'dsn air::Register<'dsn>) {
        self.register_handles.push((register, self.id));
    }

    pub(crate) fn register_for_write(&self, index: usize) -> &'dsn air::Register<'dsn> {
        let (register, owner) = self.register_handles[index];
        assert_eq!(
            owner, self.id,
            "cannot write a register from another module"
        );
        register
    }

    fn capture_name(&self) -> &'dsn str {
        let mut index = self.parameters.len();
        loop {
            let name = format!("capture_{index}");
            if self.parameters.iter().all(|p| match p {
                air::Parameter::Value { name: existing, .. }
                | air::Parameter::Interface { name: existing, .. } => *existing != name,
                air::Parameter::Clock { domain } => domain.name != name,
            }) {
                return self.alloc_str(&name);
            }
            index += 1;
        }
    }

    fn interface_type(interface: &'dsn air::InterfaceRef<'dsn>) -> &'dsn air::Interface<'dsn> {
        match interface {
            air::InterfaceRef::Parameter(parameter) => match parameter {
                air::Parameter::Interface { ty, .. } => ty,
                _ => panic!("expected interface parameter"),
            },
            air::InterfaceRef::Instance(instance) => instance.module.interface,
            air::InterfaceRef::Local(implementation) => implementation.ty,
            air::InterfaceRef::Subinterface { member, .. } => match member {
                air::InterfaceMember::Interface { ty, .. } => ty,
                air::InterfaceMember::Method(_) => panic!("expected subinterface member"),
            },
        }
    }

    fn import_interface(
        &mut self,
        interface: &'dsn air::InterfaceRef<'dsn>,
        ty: &'dsn air::Interface<'dsn>,
    ) -> &'dsn air::InterfaceRef<'dsn> {
        let key = (3, ptr::from_ref(interface) as usize);
        if let Some(&index) = self.imports.get(&key) {
            return self.interface_refs[index].0;
        }
        let parameter = self.add_parameter(air::Parameter::Interface {
            name: self.capture_name(),
            ty,
        });
        self.arguments.push(air::InstanceArgument::Interface {
            parameter,
            interface,
        });
        let imported = self.alloc(air::InterfaceRef::Parameter(parameter));
        let handle = self.push_interface(imported, ty);
        self.imports.insert(key, handle.index);
        imported
    }

    pub(crate) fn ensure_clock(&mut self, domain: &'dsn air::Domain<'dsn>) {
        if self
            .parameters
            .iter()
            .any(|p| matches!(p, air::Parameter::Clock { domain: d } if ptr::eq(*d, domain)))
        {
            return;
        }
        if !self.parents.is_empty() {
            self.in_parent(|cx| cx.ensure_clock(domain));
        }
        assert!(
            (self.id != 0 || self.outputs.iter().all(|output| output.name != domain.name))
                && self.parameters.iter().all(|parameter| match parameter {
                    air::Parameter::Value { name, .. } | air::Parameter::Interface { name, .. } =>
                        *name != domain.name,
                    air::Parameter::Clock { domain: existing } => existing.name != domain.name,
                }),
            "duplicate module parameter name"
        );
        let parameter = self.add_parameter(air::Parameter::Clock { domain });
        if !self.parents.is_empty() {
            self.arguments
                .push(air::InstanceArgument::Clock { parameter, domain });
        }
    }

    const fn swap_state(&mut self, state: &mut ModuleState<'dsn>) {
        swap(&mut self.root_active, &mut state.root_active);
        swap(&mut self.root_defined, &mut state.root_defined);
        swap(&mut self.definitions, &mut state.definitions);
        swap(&mut self.name, &mut state.name);
        swap(&mut self.parameters, &mut state.parameters);
        swap(&mut self.instances, &mut state.instances);
        swap(&mut self.registers, &mut state.registers);
        swap(&mut self.methods, &mut state.methods);
        swap(&mut self.rules, &mut state.rules);
        swap(&mut self.interface, &mut state.interface);
        swap(&mut self.scheduling, &mut state.scheduling);
        swap(&mut self.modules, &mut state.modules);
        swap(&mut self.arguments, &mut state.arguments);
        swap(&mut self.imports, &mut state.imports);
        swap(&mut self.id, &mut state.id);
    }

    pub(crate) fn in_root<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        assert!(
            self.frames.is_empty(),
            "cannot construct physical IO in action bodies"
        );
        if self.id == 0 {
            f(self)
        } else {
            self.in_parent(|cx| cx.in_root(f))
        }
    }

    fn in_parent<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let mut parent = self
            .parents
            .pop()
            .expect("HDL handle escaped its defining module");
        self.swap_state(&mut parent);
        let frames = take(&mut self.frames);
        let result = catch_unwind(AssertUnwindSafe(|| f(self)));
        self.frames = frames;
        self.swap_state(&mut parent);
        self.parents.push(parent);
        match result {
            Ok(value) => value,
            Err(error) => resume_unwind(error),
        }
    }

    pub(crate) fn begin_module(&mut self, name: &str) {
        assert!(
            self.frames.is_empty(),
            "cannot construct module instances in action bodies"
        );
        let id = self.next_module;
        self.next_module += 1;
        let mut state = ModuleState {
            root_active: true,
            root_defined: false,
            definitions: Vec::new(),
            name: self.alloc_str(&format!(
                "{}_child_{name}_{id}",
                self.parents.first().map_or(self.name, |parent| parent.name)
            )),
            parameters: Vec::new(),
            instances: Vec::new(),
            registers: Vec::new(),
            methods: Vec::new(),
            rules: Vec::new(),
            interface: None,
            scheduling: Vec::new(),
            modules: Vec::new(),
            arguments: Vec::new(),
            imports: HashMap::new(),
            id,
        };
        self.swap_state(&mut state);
        self.parents.push(state);
    }

    pub(crate) fn abort_module(&mut self) {
        let mut parent = self.parents.pop().expect("missing parent module");
        self.swap_state(&mut parent);
    }

    pub(crate) fn end_module(&mut self) -> InterfaceHandle {
        let domain = self.module_clock();
        self.ensure_clock(domain);
        let module = self.alloc(self.finish());
        let arguments = self.alloc_slice(self.arguments.iter().copied());
        self.abort_module();
        self.modules.push(module);
        let instance = self.add_instance(air::Instance {
            name: self.alloc_str(&format!("instance_{}", self.instances.len())),
            module: module.signature,
            arguments,
            domain,
        });
        let interface = self.alloc(air::InterfaceRef::Instance(instance));
        self.push_interface(interface, module.signature.interface)
    }

    pub(crate) fn primitive_name(&mut self) -> &'dsn str {
        let id = self.next_module;
        self.next_module += 1;
        self.alloc_str(&format!("mk_bram_{id}"))
    }

    pub(crate) fn attach_primitive(
        &mut self,
        module: &'dsn air::Module<'dsn>,
        arguments: &'dsn [air::InstanceArgument<'dsn>],
        domain: &'dsn air::Domain<'dsn>,
    ) -> InterfaceHandle {
        self.modules.push(module);
        let instance = self.add_instance(air::Instance {
            name: self.alloc_str(&format!("instance_{}", self.instances.len())),
            module: module.signature,
            arguments,
            domain,
        });
        let interface = self.alloc(air::InterfaceRef::Instance(instance));
        self.push_interface(interface, module.signature.interface)
    }

    pub(crate) fn module_clock(&self) -> &'dsn air::Domain<'dsn> {
        self.parameters
            .iter()
            .find_map(|parameter| match parameter {
                air::Parameter::Clock { domain } => Some(*domain),
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!(
                    "module {:?} has no clock: create one before finishing the module, e.g. with \
                     `platform.clock::<D>()`, a register, or physical IO",
                    self.name
                )
            })
    }

    pub(crate) fn finish(&self) -> air::Module<'dsn> {
        assert!(self.frames.is_empty(), "unfinished action body");
        let clock = self.module_clock();
        let interface = self.interface.unwrap_or_else(|| {
            let ty = self.alloc(air::Interface {
                name: "Empty",
                members: &[],
            });
            self.alloc(air::InterfaceImpl { ty, members: &[] })
        });
        let signature = self.alloc(air::ModuleSignature {
            name: self.name,
            parameters: self.arena.alloc_slice_copy(&self.parameters),
            interface: interface.ty,
        });
        let mut domains: Vec<&'dsn air::Domain<'dsn>> = Vec::new();
        for domain in self
            .parameters
            .iter()
            .filter_map(|parameter| match parameter {
                air::Parameter::Clock { domain } => Some(*domain),
                _ => None,
            })
            .chain(self.registers.iter().map(|register| register.domain))
            .chain(self.instances.iter().map(|instance| instance.domain))
            .chain(self.instances.iter().flat_map(|instance| {
                instance
                    .arguments
                    .iter()
                    .filter_map(|argument| match argument {
                        air::InstanceArgument::Clock { domain, .. } => Some(*domain),
                        _ => None,
                    })
            }))
        {
            if !domains.iter().any(|existing| ptr::eq(*existing, domain)) {
                domains.push(domain);
            }
        }
        air::Module {
            definition_path: if self.id == 0 {
                self.definition_path
            } else {
                None
            },
            clock,
            signature,
            primitive: None,
            modules: self.arena.alloc_slice_copy(&self.modules),
            domains: self.arena.alloc_slice_copy(&domains),
            types: self.arena.alloc_slice_copy(&self.types),
            instances: self.arena.alloc_slice_copy(&self.instances),
            registers: self.arena.alloc_slice_copy(&self.registers),
            outputs: if self.id == 0 {
                self.arena.alloc_slice_copy(&self.outputs)
            } else {
                &[]
            },
            methods: self.arena.alloc_slice_copy(&self.methods),
            rules: self.arena.alloc_slice_copy(&self.rules),
            interface,
            scheduling: self.arena.alloc_slice_copy(&self.scheduling),
        }
    }
}
