use crate::{module::Interface, value::Shape};
use std::ptr;

use super::{ActionHandle, InterfaceHandle, with_context};

#[must_use]
pub(crate) fn input_interface<I: Interface>(name: &str) -> I {
    let handle = with_context(|cx| {
        assert!(
            cx.parameters.iter().all(|parameter| match parameter {
                air::Parameter::Clock { domain } => domain.name != name,
                air::Parameter::Value { name: existing, .. }
                | air::Parameter::Interface { name: existing, .. } => *existing != name,
            }),
            "duplicate module parameter name"
        );
        let ty = cx.iface::<I>();
        let parameter = cx.add_parameter(air::Parameter::Interface {
            name: cx.alloc_str(name),
            ty,
        });
        let interface = cx.alloc(air::InterfaceRef::Parameter(parameter));
        cx.push_interface(interface, ty)
    });
    I::from_handle(handle)
}

#[must_use]
pub(crate) fn subinterface(handle: InterfaceHandle, name: &str) -> InterfaceHandle {
    with_context(|cx| {
        let (parent, ty) = cx.interface_ref(handle);
        let member = ty.members.iter().find(|member| matches!(member, air::InterfaceMember::Interface { name: label, .. } if *label == name)).expect("unknown subinterface");
        let air::InterfaceMember::Interface { ty, .. } = member else {
            unreachable!()
        };
        let interface = cx.alloc(air::InterfaceRef::Subinterface { parent, member });
        cx.push_interface(interface, ty)
    })
}

#[must_use]
pub(crate) fn interface_action<In: Shape, Out: Shape>(
    handle: InterfaceHandle,
    name: &str,
) -> ActionHandle {
    with_context(|cx| {
        let (interface, ty) = cx.interface_ref(handle);
        let signature = ty
            .members
            .iter()
            .find_map(|member| match member {
                air::InterfaceMember::Method(signature) if signature.name == name => {
                    Some(*signature)
                }
                _ => None,
            })
            .expect("unknown interface method");
        let expected = cx.action_signature::<In, Out>(name);
        assert!(
            matches!(signature.kind, air::MethodKind::Action(ty) if ptr::eq(ty, cx.ty::<Out>())),
            "interface action result type mismatch"
        );
        assert_eq!(
            signature.inputs.len(),
            expected.inputs.len(),
            "interface action argument count mismatch"
        );
        for (actual, expected) in signature.inputs.iter().zip(expected.inputs) {
            assert!(
                ptr::eq(actual.ty, expected.ty),
                "interface action argument type mismatch"
            );
        }
        cx.push_action(air::MethodRef::Interface {
            interface,
            signature,
        })
    })
}

pub(crate) fn module<I: Interface>(name: &str, f: impl FnOnce() -> I) -> I {
    with_context(|cx| {
        assert!(
            !cx.root_active && !cx.root_defined,
            "elaborate one root module at a time; nested module definitions are not supported yet"
        );
        cx.root_active = true;
        cx.set_name(&format!("mk_{name}"));
    });
    let interface = f();
    with_context(|cx| {
        let implementation = interface.air_bind(cx);
        cx.set_interface(implementation.clone());
        cx.root_active = false;
        cx.root_defined = true;
    });
    interface
}
