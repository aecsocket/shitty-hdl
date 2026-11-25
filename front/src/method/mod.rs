use crate::{
    air, elab,
    module::{Action, Interface},
    value::Shape,
};

pub struct Method<In: Shape, Out: Shape> {
    pub req: Action<In, ()>,
    pub out: Action<(), Out>,
}

impl<In: Shape, Out: Shape> Interface for Method<In, Out> {
    fn air_interface<'dsn>(cx: &mut elab::Context<'dsn>) -> &'dsn air::Interface<'dsn> {
        cx.method_interface::<In, Out>()
    }

    fn from_handle(handle: elab::InterfaceHandle) -> Self {
        Self::new(
            Action::from_interface(handle, "req"),
            Action::from_interface(handle, "out"),
        )
    }

    fn air_bind<'dsn>(&self, cx: &mut elab::Context<'dsn>) -> &'dsn air::InterfaceImpl<'dsn> {
        let ty = cx.iface::<Self>();
        let handles = [self.req.handle(), self.out.handle()];
        let mut members = Vec::new();
        for (declaration, handle) in ty.members.iter().zip(handles) {
            let air::InterfaceMember::Method(declaration) = declaration else {
                panic!("expected method declaration");
            };
            members.push(air::InterfaceBinding::Method {
                declaration,
                method: cx.action_ref(handle),
            });
        }
        let members = cx.alloc_slice(members);
        cx.alloc(air::InterfaceImpl { ty, members })
    }
}

impl<In: Shape, Out: Shape> Method<In, Out> {
    #[must_use]
    pub const fn new(req: Action<In, ()>, out: Action<(), Out>) -> Self {
        Self { req, out }
    }

    pub fn define_as(f: impl AsyncFnOnce(In::Values) -> Out::Values) -> Self {
        drop(f);
        todo!();
    }
}
