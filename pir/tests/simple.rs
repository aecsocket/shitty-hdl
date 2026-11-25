use shitty_hdl_pir::*;

#[test]
fn make_pir() {
    let sync = DomainDef {
        name: "sync".into(),
    };

    let bool = TypeDef {
        name: "bool".into(),
        shape: Shape::Bits(1),
    };
    let u32 = TypeDef {
        name: "u32".into(),
        shape: Shape::Unsigned(32),
    };

    let enable: SignalDef = SignalDef {
        name: "enable".into(),
        init: &Bits::from(false),
        ty: Type(&bool),
    };
    let led_on: SignalDef = SignalDef {
        name: "led_on".into(),
        init: &Bits::from(false),
        ty: Type(&bool),
    };
    let count: SignalDef = SignalDef {
        name: "count".into(),
        init: &Bits::from(0u32),
        ty: Type(&u32),
    };

    let count_value = Value {
        ty: Type(&u32),
        kind: ValueKind::Signal(Signal(&count)),
    };
    let threshold = Bits::from(50_000_000u32);
    let threshold_value = Value {
        ty: Type(&u32),
        kind: ValueKind::Const(&threshold),
    };

    let _design = Design {
        top: Module {
            name: "top".into(),
            ports: &[
                Port {
                    name: "enable".into(),
                    flow: Flow::In,
                    signal: Signal(&enable),
                },
                Port {
                    name: "led_on".into(),
                    flow: Flow::Out,
                    signal: Signal(&led_on),
                },
            ],
            signals: &[Signal(&count)],
            uses_domains: &[
                UseDomain {
                    domain: Domain::Comb,
                    stmts: &[Assign {
                        into: Signal(&led_on).into(),
                        from: Value {
                            ty: Type(&bool),
                            kind: ValueKind::Binary {
                                op: BinaryOp::Gt,
                                lhs: &count_value,
                                rhs: &threshold_value,
                            },
                        },
                    }
                    .into()],
                },
                UseDomain {
                    domain: ClockedDomain(&sync).into(),
                    stmts: &[],
                },
            ],
        },
        modules: &[],
    };
}
