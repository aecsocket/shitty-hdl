use super::{Lower, Result, error, same, source};
use crate::{
    Binding, ClockBinding, Declaration, Expr, ExprKind, Field, FieldValue, Instance, InterfaceExpr,
    Method, MethodBody, Module, Parameter, Register, Statement, Type,
};

impl<'o, 'm, 's> Lower<'o, 'm, 's> {
    pub(super) fn primitive(&mut self, primitive: source::Primitive<'s>) -> Result<Module<'o>> {
        match primitive {
            source::Primitive::Bram {
                address,
                data,
                address_width,
                data_width,
                depth,
            } => self.bram(address, data, address_width, data_width, depth),
        }
    }

    fn bram(
        &mut self,
        address: &'s source::Type<'s>,
        data: &'s source::Type<'s>,
        address_width: u32,
        data_width: u32,
        depth: u64,
    ) -> Result<Module<'o>> {
        if depth == 0 || address_width == 0 || data_width == 0 {
            return error("BRAM depth and packed widths must be nonzero");
        }
        if address_width < 64 && depth > (1_u64 << address_width) {
            return error("BRAM depth exceeds address capacity");
        }
        let m = self.module;
        let [source::Parameter::Clock { domain }] = m.signature.parameters else {
            return error("BRAM primitive requires exactly one clock parameter");
        };
        if domain.name == "bsv_result" {
            return error("duplicate name: bsv_result");
        }
        let clock = self.clock(domain)?;
        let reset = None;
        let interface = self.iface(m.signature.interface)?;
        let address_ty = self.ty(address)?;
        let data_ty = self.ty(data)?;
        let mut bindings = Vec::new();
        for (name, write) in [("read", false), ("write", true)] {
            let Some(source::InterfaceMember::Interface { ty, .. }) = m
                .signature
                .interface
                .members
                .iter()
                .find(|member| matches!(member, source::InterfaceMember::Interface { name: actual, .. } if *actual == name))
            else {
                return error("BRAM primitive requires read and write interfaces");
            };
            let mut methods = Vec::new();
            for request in [true, false] {
                let method_name = if request { "req" } else { "out" };
                let Some(signature) = ty.members.iter().find_map(|member| match member {
                    source::InterfaceMember::Method(signature) if signature.name == method_name => {
                        Some(*signature)
                    }
                    _ => None,
                }) else {
                    return error("BRAM channel requires req and out actions");
                };
                let source::MethodKind::Action(output) = signature.kind else {
                    return error("BRAM methods must be actions");
                };
                let valid_inputs = if request {
                    signature.inputs.len() == if write { 2 } else { 1 }
                        && same(signature.inputs[0].ty, address)
                        && (!write || same(signature.inputs[1].ty, data))
                } else {
                    signature.inputs.is_empty()
                };
                let valid_output = if !request && !write {
                    same(output, data)
                } else {
                    self.is_unit(output)?
                };
                if !valid_inputs || !valid_output {
                    return error("BRAM primitive interface type mismatch");
                }
                let signature = self.signature(signature, None)?;
                let busy = if write { "write_busy" } else { "read_busy" };
                let mut guard = Expr {
                    ty: Type::Bool,
                    kind: ExprKind::Path(busy),
                };
                let mut body = Vec::new();
                if request {
                    guard = Expr {
                        ty: Type::Bool,
                        kind: ExprKind::Unary {
                            operator: "!",
                            operand: self.alloc(guard),
                        },
                    };
                    let addr = self.bram_call(
                        "pack",
                        Type::Bit(address_width),
                        &[Expr {
                            ty: address_ty,
                            kind: ExprKind::Path(signature.inputs[0].name),
                        }],
                    );
                    if address_width >= 64 || depth != (1_u64 << address_width) {
                        let valid = Expr {
                            ty: Type::Bool,
                            kind: ExprKind::Binary {
                                operator: "<",
                                lhs: self.alloc(addr),
                                rhs: self.alloc(Expr {
                                    ty: Type::Bit(address_width),
                                    kind: ExprKind::Integer(depth),
                                }),
                            },
                        };
                        self.import("Assert");
                        body.push(Statement::Assert {
                            function: "dynamicAssert",
                            condition: valid,
                            message: "BRAM address out of range",
                        });
                    }
                    let datain = if write {
                        self.bram_call(
                            "pack",
                            Type::Bit(data_width),
                            &[Expr {
                                ty: data_ty,
                                kind: ExprKind::Path(signature.inputs[1].name),
                            }],
                        )
                    } else {
                        Expr {
                            ty: Type::Bit(data_width),
                            kind: ExprKind::Undefined,
                        }
                    };
                    let fields = [
                        FieldValue {
                            name: "write",
                            value: Self::bram_bool(write),
                        },
                        FieldValue {
                            name: "responseOnWrite",
                            value: Self::bram_bool(write),
                        },
                        FieldValue {
                            name: "address",
                            value: addr,
                        },
                        FieldValue {
                            name: "datain",
                            value: datain,
                        },
                    ];
                    body.push(Statement::ActionCall {
                        action_value: false,
                        target: if write {
                            "memory.portB.request.put"
                        } else {
                            "memory.portA.request.put"
                        },
                        arguments: self.slice(&[Expr {
                            ty: Type::Inferred,
                            kind: ExprKind::Struct {
                                name: "BRAMRequest",
                                fields: self.slice(&fields),
                            },
                        }]),
                        result: None,
                    });
                } else {
                    body.push(Statement::ActionCall {
                        action_value: true,
                        target: if write {
                            "memory.portB.response.get"
                        } else {
                            "memory.portA.response.get"
                        },
                        arguments: &[],
                        result: Some(Field {
                            name: "response",
                            ty: Type::Bit(data_width),
                        }),
                    });
                }
                body.push(Statement::RegisterWrite {
                    name: busy,
                    value: Self::bram_bool(request),
                });
                if !request && !write {
                    body.push(Statement::Return(Some(self.bram_call(
                        "unpack",
                        data_ty,
                        &[Expr {
                            ty: Type::Bit(data_width),
                            kind: ExprKind::Path("response"),
                        }],
                    ))));
                }
                methods.push(Binding::Method(Method {
                    signature,
                    guard: Some(self.alloc(guard)),
                    body: MethodBody::Action(self.slice(&body)),
                }));
            }
            bindings.push(Binding::Interface {
                name,
                value: InterfaceExpr {
                    ty: self.iface(ty)?,
                    members: self.slice(&methods),
                },
            });
        }
        self.import("BRAM");
        let config = Expr {
            ty: Type::Named("BRAM_Configure"),
            kind: ExprKind::Struct {
                name: "BRAM_Configure",
                fields: self.slice(&[
                    FieldValue {
                        name: "memorySize",
                        value: Expr {
                            ty: Type::Named("Integer"),
                            kind: ExprKind::Integer(depth),
                        },
                    },
                    FieldValue {
                        name: "latency",
                        value: Expr {
                            ty: Type::Named("Integer"),
                            kind: ExprKind::Integer(1),
                        },
                    },
                    FieldValue {
                        name: "loadFormat",
                        value: Expr {
                            ty: Type::Named("LoadFormat"),
                            kind: ExprKind::Tagged {
                                tag: "None",
                                payload: None,
                            },
                        },
                    },
                    FieldValue {
                        name: "outFIFODepth",
                        value: Expr {
                            ty: Type::Named("Integer"),
                            kind: ExprKind::Integer(1),
                        },
                    },
                    FieldValue {
                        name: "allowWriteResponseBypass",
                        value: Self::bram_bool(false),
                    },
                ]),
            },
        };
        let declarations = self.slice(&[Declaration {
            field: Field {
                name: "bram_config",
                ty: config.ty,
            },
            value: config,
        }]);
        let initial = self.alloc(Self::bram_bool(false));
        let registers = self.slice(&["read_busy", "write_busy"].map(|name| Register {
            name,
            ty: Type::Bool,
            constructor: "mkReg",
            initial: Some(initial),
            clock,
            reset,
        }));
        let memory = Instance {
            name: "memory",
            constructor: "mkBRAM2Server",
            interface: Type::Applied {
                name: "BRAM2Port",
                arguments: self.slice(&[Type::Bit(address_width), Type::Bit(data_width)]),
            },
            arguments: self.slice(&[Expr {
                ty: config.ty,
                kind: ExprKind::Path("bram_config"),
            }]),
            clock,
            reset,
        };
        Ok(Module {
            name: self.module_name(m.signature.name),
            clock_bindings: self.slice(&[ClockBinding {
                name: "bsv_default_clock",
                constructor: "exposeCurrentClock",
            }]),
            parameters: self.slice(&[Parameter::Clock {
                name: self.text(domain.name),
                port: self.text(&format!("CLK_{}", domain.name)),
            }]),
            interface,
            reset_bindings: &[],
            declarations,
            registers,
            instances: self.slice(&[memory]),
            local_interfaces: &[],
            rules: &[],
            implementation: InterfaceExpr {
                ty: interface,
                members: self.slice(&bindings),
            },
            scheduling: &[],
        })
    }

    fn bram_call(&self, function: &'o str, ty: Type<'o>, arguments: &[Expr<'o>]) -> Expr<'o> {
        Expr {
            ty,
            kind: ExprKind::Call {
                function,
                arguments: self.slice(arguments),
            },
        }
    }

    fn bram_bool(value: bool) -> Expr<'o> {
        Expr {
            ty: Type::Bool,
            kind: ExprKind::Bool(value),
        }
    }
}
