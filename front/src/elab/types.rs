use std::any::{TypeId, type_name};
use std::ptr;

use air::{
    Argument, BitLiteral, Expr, ExprKind, FieldValue, Interface, InterfaceMember, MethodKind,
    MethodSignature, NamedType, NamedTypeKind, Type,
};

use super::Context;
use crate::{domain::Domain, method::Method, module::Interface as InterfaceShape, value::Shape};

impl<'dsn> Context<'dsn> {
    pub fn ty<T: Shape>(&mut self) -> &'dsn Type<'dsn> {
        let id = TypeId::of::<T>();
        if let Some(&ty) = self.type_cache.get(&id) {
            return ty;
        }
        let ty = T::air_type(self);
        self.type_cache.insert(id, ty);
        ty
    }

    pub fn iface<I: InterfaceShape>(&mut self) -> &'dsn Interface<'dsn> {
        let id = TypeId::of::<I>();
        if let Some(&interface) = self.iface_cache.get(&id) {
            return interface;
        }
        let interface = I::air_interface(self);
        let name = self.type_name::<I>();
        let interface = self.alloc(Interface {
            name,
            members: interface.members,
        });
        self.iface_cache.insert(id, interface);
        interface
    }

    #[must_use]
    pub(crate) fn type_name<T: 'static>(&self) -> &'dsn str {
        let rust_name = type_name::<T>();
        let mut base: String = rust_name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        if base.starts_with(|c: char| c.is_ascii_digit()) {
            base.insert(0, '_');
        }
        let hash = rust_name
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
            });
        let base = format!("{base}_{hash:016x}");
        let mut name = base.clone();
        let mut suffix = 0_usize;
        while self.types.iter().any(|ty| ty.name == name)
            || self.iface_cache.values().any(|iface| iface.name == name)
        {
            suffix += 1;
            name = format!("{base}_{suffix}");
        }
        self.alloc_str(&name)
    }

    pub fn named_type<T: Shape>(&mut self, kind: NamedTypeKind<'dsn>) -> &'dsn Type<'dsn> {
        let name = self.type_name::<T>();
        let named = self.add_type(NamedType { name, kind });
        self.alloc(Type::Named(named))
    }

    pub fn struct_value<T: Shape>(&mut self, values: Vec<&'dsn Expr<'dsn>>) -> &'dsn Expr<'dsn> {
        let ty = self.ty::<T>();
        let Type::Named(named) = ty else {
            panic!("struct literal requires a named struct type");
        };
        let NamedTypeKind::Struct(fields) = &named.kind else {
            panic!("struct literal requires a named struct type");
        };
        assert_eq!(fields.len(), values.len(), "struct field count mismatch");
        let fields = self.alloc_slice(fields.iter().zip(values).map(|(field, value)| {
            assert!(ptr::eq(field.ty, value.ty), "struct field type mismatch");
            FieldValue { field, value }
        }));
        self.alloc(Expr {
            ty,
            kind: ExprKind::Struct(fields),
        })
    }

    pub(crate) fn tuple_value<T: Shape>(
        &mut self,
        values: Vec<&'dsn Expr<'dsn>>,
    ) -> &'dsn Expr<'dsn> {
        let ty = self.ty::<T>();
        let kind = match ty {
            Type::Unit => {
                assert!(values.is_empty(), "unit literal must have no elements");
                ExprKind::Unit
            }
            Type::Tuple(types) => {
                assert_eq!(types.len(), values.len(), "tuple element count mismatch");
                for (expected, value) in types.iter().zip(&values) {
                    assert!(ptr::eq(*expected, value.ty), "tuple element type mismatch");
                }
                let values = self.alloc_slice(values);
                ExprKind::Tuple(values)
            }
            _ => panic!("tuple literal requires a tuple or unit type"),
        };
        self.alloc(Expr { ty, kind })
    }

    pub fn enum_value<T: Shape>(&mut self, variant: &str) -> &'dsn Expr<'dsn> {
        let ty = self.ty::<T>();
        let Type::Named(named) = ty else {
            panic!("enum literal requires a named enum type");
        };
        let NamedTypeKind::Enum(variants) = &named.kind else {
            panic!("enum literal requires a named enum type");
        };
        let variant = variants
            .iter()
            .copied()
            .find(|name| *name == variant)
            .expect("unknown enum variant");
        self.alloc(Expr {
            ty,
            kind: ExprKind::Enum { variant },
        })
    }

    pub(crate) fn bits<T: Shape>(&mut self, words: &[u64]) -> &'dsn Expr<'dsn> {
        let ty = self.ty::<T>();
        let width = match ty {
            Type::Bit { width } | Type::Uint { width } | Type::Sint { width } => *width,
            _ => panic!("bit literal requires a bit, uint, or sint type"),
        };
        assert_eq!(
            words.len(),
            width.div_ceil(64) as usize,
            "bit literal word count mismatch"
        );
        let remainder = width % 64;
        if remainder != 0 {
            assert_eq!(
                words.last().copied().unwrap() >> remainder,
                0,
                "unused high bits must be zero"
            );
        }
        let words_le = self.alloc_slice(words.iter().copied());
        self.alloc(Expr {
            ty,
            kind: ExprKind::Bits(BitLiteral { width, words_le }),
        })
    }

    pub(crate) fn boolean(&mut self, value: bool) -> &'dsn Expr<'dsn> {
        let ty = self.ty::<bool>();
        self.alloc(Expr {
            ty,
            kind: ExprKind::Bool(value),
        })
    }

    pub(crate) fn unit(&mut self) -> &'dsn Expr<'dsn> {
        let ty = self.ty::<()>();
        self.alloc(Expr {
            ty,
            kind: ExprKind::Unit,
        })
    }

    pub(crate) fn method_interface<D: Domain, In: Shape, Out: Shape>(&mut self) -> &'dsn Interface<'dsn> {
        let req = self.action_signature::<In, ()>("req");
        let out = self.action_signature::<(), Out>("out");
        let members =
            self.alloc_slice([InterfaceMember::Method(req), InterfaceMember::Method(out)]);
        let name = self.type_name::<Method<D, In, Out>>();
        self.alloc(Interface { name, members })
    }

    pub fn action_signature<In: Shape, Out: Shape>(
        &mut self,
        name: &str,
    ) -> &'dsn MethodSignature<'dsn> {
        let input = self.ty::<In>();
        let output = self.ty::<Out>();
        let arguments: Vec<_> = Self::input_types(input)
            .into_iter()
            .enumerate()
            .map(|(index, ty)| {
                let name = self.alloc_str(&format!("arg_{index}"));
                Argument { name, ty }
            })
            .collect();
        let inputs = self.alloc_slice(arguments);
        let name = self.alloc_str(name);
        self.alloc(MethodSignature {
            name,
            inputs,
            kind: MethodKind::Action(output),
        })
    }

    #[must_use]
    pub(crate) fn input_types(ty: &'dsn Type<'dsn>) -> Vec<&'dsn Type<'dsn>> {
        match ty {
            Type::Unit => Vec::new(),
            Type::Tuple(types) => types.to_vec(),
            _ => vec![ty],
        }
    }
}
