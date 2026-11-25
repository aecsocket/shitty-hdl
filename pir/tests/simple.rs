use shitty_hdl_pir::*;

#[test]
fn make_pir() {
    let sync = DomainDef {
        name: "sync".parse().unwrap(),
    };

    let bool = TypeDef {
        name: "bool".parse().unwrap(),
        shape: Shape::Bits(1),
    };
    let u32 = TypeDef {
        name: "u32".parse().unwrap(),
        shape: Shape::Unsigned(32),
    };

    let enable: SignalDef = SignalDef {
        name: "enable".parse().unwrap(),
        init: &Bits::from(false),
        ty: Type(&bool),
    };
    let led_on: SignalDef = SignalDef {
        name: "led_on".parse().unwrap(),
        init: &Bits::from(false),
        ty: Type(&bool),
    };
    let count: SignalDef = SignalDef {
        name: "count".parse().unwrap(),
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

    let _top = Module {
        name: "top".parse().unwrap(),
        ports: &[
            Port {
                name: "enable".parse().unwrap(),
                flow: Flow::In,
                signal: Signal(&enable),
            },
            Port {
                name: "led_on".parse().unwrap(),
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
                        kind: Binary {
                            op: BinaryOp::Gt,
                            lhs: &count_value,
                            rhs: &threshold_value,
                        }
                        .into(),
                    },
                }
                .into()],
            },
            UseDomain {
                domain: ClockedDomain(&sync).into(),
                stmts: &[],
            },
        ],
        children: &[],
    };
}
