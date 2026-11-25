use std::ptr;

use crate::{data::Bram, domain::Clock, module::Interface, ops::Bits};

use super::with_context;

pub(crate) fn bram<Addr: Bits, Data: Bits>(clock: Clock, depth: u64) -> Bram<Addr, Data> {
    assert!(depth != 0, "BRAM depth must be nonzero");
    let address_width = u32::try_from(Addr::WIDTH).expect("BRAM address width exceeds u32");
    let data_width = u32::try_from(Data::WIDTH).expect("BRAM data width exceeds u32");
    assert!(address_width != 0, "BRAM address width must be nonzero");
    assert!(data_width != 0, "BRAM data width must be nonzero");
    assert!(
        address_width >= 64 || depth <= (1u64 << address_width),
        "BRAM depth exceeds address space"
    );
    let handle = with_context(|cx| {
        assert!(
            cx.frames.is_empty(),
            "cannot construct module instances in action bodies"
        );
        let domain = match clock.handle {
            None => match cx.domain_cache.get(&clock.identity) {
                Some(&(_, domain)) => domain,
                None => cx.intern_domain(clock.identity, "clk").1,
            },
            Some(handle) => {
                cx.check_design(handle.design_id);
                let domain = cx.domains[handle.index];
                let canonical = cx
                    .domain_cache
                    .get(&clock.identity)
                    .expect("unregistered clock domain")
                    .1;
                assert!(ptr::eq(domain, canonical), "clock domain identity mismatch");
                domain
            }
        };
        cx.ensure_clock(domain);
        let address = cx.ty::<Addr>();
        let data = cx.ty::<Data>();
        let ty = cx.iface::<Bram<Addr, Data>>();
        let parameter = cx.alloc(air::Parameter::Clock { domain });
        let name = cx.primitive_name();
        let signature = cx.alloc(air::ModuleSignature {
            name,
            parameters: cx.alloc_slice([parameter]),
            interface: ty,
        });
        let module = cx.alloc(air::Module {
            signature,
            primitive: Some(air::Primitive::Bram {
                address,
                data,
                address_width,
                data_width,
                depth,
            }),
            modules: &[],
            domains: cx.alloc_slice([domain]),
            types: &[],
            instances: &[],
            registers: &[],
            methods: &[],
            rules: &[],
            interface: cx.alloc(air::InterfaceImpl { ty, members: &[] }),
            scheduling: &[],
        });
        let arguments = cx.alloc_slice([air::InstanceArgument::Clock { parameter, domain }]);
        cx.attach_primitive(module, arguments, domain)
    });
    Bram::from_handle(handle)
}
