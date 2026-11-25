use std::ptr;
use std::{any::TypeId, collections::HashMap};

use bumpalo::Bump;

use super::{ActionHandle, BranchHandle, DesignId, ExprHandle, InterfaceHandle};
use crate::value::Shape;

pub(crate) struct Frame<'dsn> {
    pub scope: usize,
    pub statements: Vec<air::Statement<'dsn>>,
}

pub(crate) struct Branch<'dsn> {
    pub parent_scope: usize,
    pub block: air::Block<'dsn>,
    pub result: &'dsn air::Expr<'dsn>,
}

pub struct Context<'dsn> {
    pub(crate) arena: &'dsn Bump,
    pub(crate) design_id: DesignId,
    pub(crate) root_active: bool,
    pub(crate) root_defined: bool,
    pub(crate) definitions: Vec<&'dsn air::MethodSignature<'dsn>>,
    name: &'dsn str,
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
            arena,
            design_id,
            root_active: false,
            root_defined: false,
            definitions: Vec::new(),
            name: "mkTop",
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

    pub(crate) fn domain<D: 'static>(&mut self, name: &str) -> &'dsn air::Domain<'dsn> {
        self.intern_domain(TypeId::of::<D>(), name).1
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

    pub(crate) fn add_schedule_constraint(&mut self, constraint: air::ScheduleConstraint<'dsn>) {
        self.scheduling.push(constraint);
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
        self.expressions.push((expr, self.current_scope()));
        handle
    }

    pub(crate) fn expr(&self, handle: ExprHandle) -> &'dsn air::Expr<'dsn> {
        self.check_design(handle.design_id);
        let (expr, mut scope) = self.expressions[handle.index];
        let target = scope;
        scope = self.current_scope();
        while scope != 0 && scope != target {
            scope = self.scope_parents[scope];
        }
        assert!(
            target == 0 || scope == target,
            "HDL expression escaped its action or branch scope"
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
    pub fn action_ref(&self, handle: ActionHandle) -> air::MethodRef<'dsn> {
        self.check_design(handle.design_id);
        self.actions[handle.index]
    }

    pub(crate) fn push_action(&mut self, method: air::MethodRef<'dsn>) -> ActionHandle {
        let handle = ActionHandle {
            design_id: self.design_id,
            index: self.actions.len(),
        };
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
        self.interface_refs.push((interface, ty));
        handle
    }

    pub(crate) fn interface_ref(
        &self,
        handle: InterfaceHandle,
    ) -> (&'dsn air::InterfaceRef<'dsn>, &'dsn air::Interface<'dsn>) {
        self.check_design(handle.design_id);
        self.interface_refs[handle.index]
    }

    pub(crate) fn branch(&self, handle: BranchHandle) -> &Branch<'dsn> {
        self.check_design(handle.design_id);
        let branch = &self.branches[handle.index];
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

    pub(crate) fn finish(self) -> air::Module<'dsn> {
        assert!(self.frames.is_empty(), "unfinished action body");
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
        air::Module {
            signature,
            domains: self.arena.alloc_slice_copy(&self.domains),
            types: self.arena.alloc_slice_copy(&self.types),
            instances: self.arena.alloc_slice_copy(&self.instances),
            registers: self.arena.alloc_slice_copy(&self.registers),
            methods: self.arena.alloc_slice_copy(&self.methods),
            rules: self.arena.alloc_slice_copy(&self.rules),
            interface,
            scheduling: self.arena.alloc_slice_fill_iter(self.scheduling),
        }
    }
}
