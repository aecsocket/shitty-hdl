use shitty_hdl_pir::*;

#[test]
fn make_pir() {
    let sync = DomainDef {
        name: "sync".into(),
    };

    let bool: TypeDef = TypeDef {
        name: "bool".into(),
        shape: Shape::Bits(1),
    };

    let design = Design {
        top: Module {
            name: "top".into(),
            ports: vec![
                Port {
                    name: "enable".into(),
                    flow: Flow::In,
                    ty: Type(&bool),
                },
                Port {
                    name: "led_on".into(),
                    flow: Flow::Out,
                    ty: Type(&bool),
                },
            ],
            uses_domains: vec![
                UseDomain {
                    domain: Domain::Comb,
                    stmts: vec![],
                },
                UseDomain {
                    domain: ClockedDomain(&sync).into(),
                    stmts: vec![],
                },
            ],
        },
        modules: vec![],
    };
}
