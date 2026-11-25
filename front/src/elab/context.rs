use bumpalo::Bump;

pub struct Context<'dsn> {
    arena: &'dsn Bump,
    name: &'dsn str,
    types: Vec<&'dsn air::NamedType<'dsn>>,
    parameters: Vec<&'dsn air::Parameter<'dsn>>,
    instances: Vec<&'dsn air::Instance<'dsn>>,
    registers: Vec<&'dsn air::Register<'dsn>>,
    methods: Vec<&'dsn air::Method<'dsn>>,
    rules: Vec<&'dsn air::Rule<'dsn>>,
    interface: Option<&'dsn air::InterfaceImpl<'dsn>>,
    scheduling: Vec<air::ScheduleConstraint<'dsn>>,
}

macro_rules! record {
    ($method:ident, $field:ident, $ty:ident) => {
        pub fn $method(&mut self, node: air::$ty<'dsn>) -> &'dsn air::$ty<'dsn> {
            let node = self.alloc(node);
            self.$field.push(node);
            node
        }
    };
}

impl<'dsn> Context<'dsn> {
    pub(super) const fn new(arena: &'dsn Bump) -> Self {
        Self {
            arena,
            name: "mkTop",
            types: Vec::new(),
            parameters: Vec::new(),
            instances: Vec::new(),
            registers: Vec::new(),
            methods: Vec::new(),
            rules: Vec::new(),
            interface: None,
            scheduling: Vec::new(),
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

    pub fn set_name(&mut self, name: &str) {
        self.name = self.alloc_str(name);
    }

    record!(add_type, types, NamedType);
    record!(add_parameter, parameters, Parameter);
    record!(add_instance, instances, Instance);
    record!(add_register, registers, Register);
    record!(add_method, methods, Method);
    record!(add_rule, rules, Rule);

    pub fn set_interface(&mut self, interface: air::InterfaceImpl<'dsn>) {
        assert!(
            self.interface.is_none(),
            "module interface is already defined"
        );
        self.interface = Some(self.alloc(interface));
    }

    pub fn add_schedule_constraint(&mut self, constraint: air::ScheduleConstraint<'dsn>) {
        self.scheduling.push(constraint);
    }

    pub(super) fn finish(self) -> air::Module<'dsn> {
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
