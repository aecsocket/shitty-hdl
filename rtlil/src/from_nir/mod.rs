mod cells;
mod errors;

pub use errors::LowerError;

use crate::ast;
use irt::{Ident, bumpalo::Bump};
use std::collections::{HashMap, HashSet};

type Net<'d> = nir::Net<'d>;

struct Context<'d> {
    modules: Vec<nir::Module<'d>>,
    parents: Vec<Option<usize>>,
    names: Vec<ast::Id>,
    owners: HashMap<nir::CellRef<'d>, (usize, usize, u32)>,
    resolved: HashMap<nir::LateNet<'d>, Option<Net<'d>>>,
}

fn valid(name: Ident, errors: &mut Vec<LowerError>) {
    let text = name.to_string();
    if text.is_empty() || text.chars().any(|c| c.is_whitespace() || c.is_control()) {
        errors.push(LowerError::InvalidIdentifier { name });
    }
}

fn width(cell: nir::CellRef<'_>) -> usize {
    match *cell.0 {
        nir::Cell::Input(x) => x.width as usize,
        nir::Cell::Unary(x) => match x.op {
            nir::UnaryOp::Neg | nir::UnaryOp::Not => x.value.0.len(),
            _ => 1,
        },
        nir::Cell::Binary(x) => match x.op {
            nir::BinaryOp::Eq
            | nir::BinaryOp::Ne
            | nir::BinaryOp::Lt(_)
            | nir::BinaryOp::Le(_)
            | nir::BinaryOp::Gt(_)
            | nir::BinaryOp::Ge(_) => 1,
            _ => x.lhs.0.len(),
        },
        nir::Cell::Mux(x) => x.when_true.0.len(),
        nir::Cell::Match(x) => x.arms.len(),
        nir::Cell::AssignmentList(x) => x.default.0.len(),
        nir::Cell::FlipFlop(x) => x.data.0.len(),
    }
}

fn collect<'d>(
    module: nir::Module<'d>,
    parent: Option<usize>,
    ctx: &mut Context<'d>,
    errors: &mut Vec<LowerError>,
) {
    let index = ctx.modules.len();
    valid(module.name, errors);
    ctx.modules.push(module);
    ctx.parents.push(parent);
    ctx.names.push(if parent.is_none() {
        ast::Id::Public(module.name)
    } else {
        ast::Id::Internal(Ident::new(format!("module{index}")))
    });
    let mut ports = HashSet::new();
    for port in module.ports {
        valid(port.name, errors);
        if !ports.insert(port.name.to_string()) {
            errors.push(LowerError::DuplicatePort {
                module: module.name,
                port: port.name,
            });
        }
        if port.value.0.len() > i32::MAX as usize {
            errors.push(LowerError::WidthOverflow {
                module: module.name,
                width: port.value.0.len(),
            });
        }
    }
    for (cell_index, &cell) in module.cells.iter().enumerate() {
        let w = width(cell);
        if w > i32::MAX as usize {
            errors.push(LowerError::WidthOverflow {
                module: module.name,
                width: w,
            });
        }
        if let Some(&(first, _, _)) = ctx.owners.get(&cell) {
            errors.push(LowerError::DuplicateCell {
                module: module.name,
                first_module: ctx.modules[first].name,
                cell: cell_index,
            });
        } else {
            ctx.owners
                .insert(cell, (index, cell_index, u32::try_from(w).unwrap_or(0)));
        }
    }
    let mut children = HashSet::new();
    for &child in module.children {
        if !children.insert(child.name.to_string()) {
            errors.push(LowerError::DuplicateModule {
                parent: module.name,
                child: child.name,
            });
        }
        collect(child, Some(index), ctx, errors);
    }
}

fn resolve<'d>(
    net: nir::LateNet<'d>,
    definitions: &HashMap<nir::LateNet<'d>, Net<'d>>,
    resolved: &mut HashMap<nir::LateNet<'d>, Option<Net<'d>>>,
    errors: &mut Vec<LowerError>,
) {
    if resolved.contains_key(&net) {
        return;
    }
    let mut path = Vec::new();
    let mut seen = HashSet::new();
    let mut current = net;
    let result = loop {
        if let Some(&value) = resolved.get(&current) {
            break value;
        }
        if !seen.insert(current) {
            errors.push(LowerError::ConnectionCycle {
                net: current.0.name,
            });
            break None;
        }
        path.push(current);
        match definitions.get(&current).copied() {
            Some(Net::Late(next)) => current = next,
            Some(value) => break Some(value),
            None => {
                errors.push(LowerError::MissingConnection {
                    net: current.0.name,
                });
                break None;
            }
        }
    };
    for item in path {
        resolved.insert(item, result);
    }
}

impl<'d> Context<'d> {
    fn canonical(&self, net: Net<'d>) -> Option<Net<'d>> {
        match net {
            Net::Late(late) => self.resolved.get(&late).copied().flatten(),
            _ => Some(net),
        }
    }
    fn inside(&self, mut module: usize, ancestor: usize) -> bool {
        loop {
            if module == ancestor {
                return true;
            }
            match self.parents[module] {
                Some(parent) => module = parent,
                None => return false,
            }
        }
    }
}

pub fn from_nir<'d>(
    netlist: nir::Netlist<'d>,
    bump: &'d Bump,
) -> (Option<ast::Design<'d>>, Vec<LowerError>) {
    let mut errors = Vec::new();
    let mut ctx = Context {
        modules: Vec::new(),
        parents: Vec::new(),
        names: Vec::new(),
        owners: HashMap::new(),
        resolved: HashMap::new(),
    };
    collect(netlist.top, None, &mut ctx, &mut errors);
    let mut definitions = HashMap::new();
    for connection in netlist.connections {
        if definitions.insert(connection.from, connection.to).is_some() {
            errors.push(LowerError::DuplicateConnection {
                net: connection.from.0.name,
            });
        }
    }
    let mut late = Vec::new();
    late.extend(netlist.connections.iter().map(|c| c.from));
    let mut visit = |net| {
        if let Net::Late(n) = net {
            late.push(n);
        }
    };
    for module in &ctx.modules {
        for port in module.ports {
            for &net in port.value.0 {
                visit(net);
            }
        }
        for cell in module.cells {
            match *cell.0 {
                nir::Cell::Input(_) => {}
                nir::Cell::Unary(x) => {
                    for &n in x.value.0 {
                        visit(n);
                    }
                }
                nir::Cell::Binary(x) => {
                    for &n in x.lhs.0.iter().chain(x.rhs.0) {
                        visit(n);
                    }
                }
                nir::Cell::Mux(x) => {
                    visit(x.select);
                    for &n in x.when_true.0.iter().chain(x.when_false.0) {
                        visit(n);
                    }
                }
                nir::Cell::Match(x) => {
                    visit(x.enable);
                    for &n in x.value.0 {
                        visit(n);
                    }
                }
                nir::Cell::AssignmentList(x) => {
                    for &n in x.default.0 {
                        visit(n);
                    }
                    for a in x.assigns {
                        visit(a.condition);
                        for &n in a.value.0 {
                            visit(n);
                        }
                    }
                }
                nir::Cell::FlipFlop(x) => {
                    visit(x.clock);
                    visit(x.async_reset);
                    for &n in x.data.0 {
                        visit(n);
                    }
                }
            }
        }
    }
    for binding in netlist.signals {
        for &n in binding.value.0 {
            visit(n);
        }
    }
    for n in late {
        resolve(n, &definitions, &mut ctx.resolved, &mut errors);
    }
    if !errors.is_empty() {
        return (None, errors);
    }
    let mut modules = Vec::new();
    for index in 0..ctx.modules.len() {
        let mut emitter = Emitter {
            bump,
            ctx: &ctx,
            module: ctx.modules[index].name,
            index,
            items: Vec::new(),
            nets: HashMap::new(),
            outputs: HashMap::new(),
            serial: 0,
        };
        emitter.lower(&mut errors);
        modules.push(ast::Module {
            name: ctx.names[index],
            attributes: if index == 0 {
                bump.alloc_slice_copy(&[ast::Attribute {
                    name: ast::Id::Public(Ident::new("top")),
                    value: ast::Const::Integer(1),
                }])
            } else {
                &[]
            },
            items: bump.alloc_slice_copy(&emitter.items),
        });
    }
    let design = if errors.is_empty() {
        Some(ast::Design {
            autoidx: None,
            modules: bump.alloc_slice_copy(&modules),
        })
    } else {
        None
    };
    (design, errors)
}

struct Emitter<'a, 'd> {
    bump: &'d Bump,
    ctx: &'a Context<'d>,
    module: Ident,
    index: usize,
    items: Vec<ast::ModuleItem<'d>>,
    nets: HashMap<Net<'d>, ast::SigSpec<'d>>,
    outputs: HashMap<nir::CellRef<'d>, ast::SigSpec<'d>>,
    serial: usize,
}

impl<'a, 'd> Emitter<'a, 'd> {
    fn fresh(&mut self) -> ast::Id {
        let id = ast::Id::Internal(Ident::new(format!("n{}", self.serial)));
        self.serial += 1;
        id
    }
    fn width(&self, cell: nir::CellRef<'d>) -> u32 {
        self.ctx.owners[&cell].2
    }
    fn cell_index(&self, cell: nir::CellRef<'d>) -> usize {
        self.ctx.owners[&cell].1
    }
    fn output(&self, cell: nir::CellRef<'d>) -> ast::SigSpec<'d> {
        self.outputs[&cell]
    }
    fn wire_named(
        &mut self,
        name: ast::Id,
        width: u32,
        port: Option<ast::WirePort>,
    ) -> ast::SigSpec<'d> {
        if width == 0 && port.is_none() {
            return self.concat(&[]);
        }
        self.items.push(ast::ModuleItem::Wire(ast::Wire {
            name,
            attributes: &[],
            width,
            offset: 0,
            upto: false,
            signed: false,
            port,
        }));
        ast::SigSpec::Wire(name)
    }
    fn wire(&mut self, width: u32) -> ast::SigSpec<'d> {
        let name = self.fresh();
        self.wire_named(name, width, None)
    }
    fn concat(&self, parts: &[ast::SigSpec<'d>]) -> ast::SigSpec<'d> {
        if parts.len() == 1 {
            parts[0]
        } else {
            ast::SigSpec::Concat(self.bump.alloc_slice_copy(parts))
        }
    }
    fn select(&self, value: ast::SigSpec<'d>, offset: u32, width: u32) -> ast::SigSpec<'d> {
        if width == 0 {
            return self.concat(&[]);
        }
        ast::SigSpec::Select {
            value: self.bump.alloc(value),
            select: if width == 1 {
                ast::Select::Bit(offset as i32)
            } else {
                ast::Select::Range {
                    msb: (offset + width - 1) as i32,
                    lsb: offset as i32,
                }
            },
        }
    }
    fn constant(&self, value: bool) -> ast::SigSpec<'d> {
        ast::SigSpec::Const(ast::Const::Bits(self.bump.alloc_slice_copy(&[if value {
            ast::State::One
        } else {
            ast::State::Zero
        }])))
    }
    fn bits(&self, bits: &irt::Bits) -> ast::SigSpec<'d> {
        let states: Vec<_> = (0..bits.width())
            .map(|bit| {
                if bits.words()[bit as usize / 64] & (1u64 << (bit % 64)) == 0 {
                    ast::State::Zero
                } else {
                    ast::State::One
                }
            })
            .collect();
        ast::SigSpec::Const(ast::Const::Bits(self.bump.alloc_slice_copy(&states)))
    }
    fn init(&mut self, cell: nir::CellRef<'d>, bits: &irt::Bits) {
        let ast::SigSpec::Const(value) = self.bits(bits) else {
            unreachable!()
        };
        let ast::SigSpec::Wire(name) = self.output(cell) else {
            return;
        };
        for item in &mut self.items {
            if let ast::ModuleItem::Wire(wire) = item {
                if wire.name == name {
                    wire.attributes = self.bump.alloc_slice_copy(&[ast::Attribute {
                        name: ast::Id::Public(Ident::new("init")),
                        value,
                    }]);
                }
            }
        }
    }
    fn connect(&mut self, into: ast::SigSpec<'d>, from: ast::SigSpec<'d>) {
        self.items
            .push(ast::ModuleItem::Connect(ast::Connect { into, from }));
    }
    fn builtin(
        &mut self,
        ty: &str,
        parameters: &[(&str, i32)],
        connections: &[(&str, ast::SigSpec<'d>)],
    ) {
        let parameters: Vec<_> = parameters
            .iter()
            .map(|&(n, v)| (n, ast::Const::Integer(v)))
            .collect();
        self.builtin_values(ty, &parameters, connections);
    }
    fn builtin_values(
        &mut self,
        ty: &str,
        parameters: &[(&str, ast::Const<'d>)],
        connections: &[(&str, ast::SigSpec<'d>)],
    ) {
        let name = self.fresh();
        let parameters: Vec<_> = parameters
            .iter()
            .map(|&(n, value)| ast::CellParameter {
                name: ast::Id::Public(Ident::new(n)),
                kind: ast::ParameterKind::Default,
                value,
            })
            .collect();
        let connections: Vec<_> = connections
            .iter()
            .map(|&(n, value)| ast::PortConnection {
                port: ast::Id::Public(Ident::new(n)),
                value,
            })
            .collect();
        self.items.push(ast::ModuleItem::Cell(ast::Cell {
            name,
            ty: ast::Id::Internal(Ident::new(ty.trim_start_matches('$'))),
            attributes: &[],
            parameters: self.bump.alloc_slice_copy(&parameters),
            connections: self.bump.alloc_slice_copy(&connections),
        }));
    }
    fn canonical(&self, net: Net<'d>) -> Result<Net<'d>, LowerError> {
        let net = self.ctx.canonical(net).ok_or_else(|| match net {
            Net::Late(n) => LowerError::MissingConnection { net: n.0.name },
            _ => unreachable!(),
        })?;
        if let Net::Cell { cell, bit } = net {
            let &(_, index, width) = self.ctx.owners.get(&cell).ok_or(LowerError::UnknownCell {
                module: self.module,
            })?;
            if bit >= width {
                return Err(LowerError::CellBitOutOfBounds {
                    module: self.module,
                    cell: index,
                    bit,
                    width,
                });
            }
        }
        Ok(net)
    }
    fn net(&self, net: Net<'d>) -> Result<ast::SigSpec<'d>, LowerError> {
        let net = self.canonical(net)?;
        if let Net::Const(value) = net {
            return Ok(self.constant(value));
        }
        if let Some(&value) = self.nets.get(&net) {
            return Ok(value);
        }
        let Net::Cell { cell, bit } = net else {
            unreachable!()
        };
        let &(owner, index, _) = &self.ctx.owners[&cell];
        Err(LowerError::UnroutedNet {
            module: self.module,
            source_module: self.ctx.modules[owner].name,
            cell: index,
            bit,
        })
    }
    fn sig(&self, value: nir::Value<'d>) -> Result<ast::SigSpec<'d>, LowerError> {
        if value.0.len() > i32::MAX as usize {
            return Err(LowerError::WidthOverflow {
                module: self.module,
                width: value.0.len(),
            });
        }
        let parts = value
            .0
            .iter()
            .rev()
            .map(|&n| self.net(n))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.concat(&parts))
    }
    fn lower(&mut self, errors: &mut Vec<LowerError>) {
        let module = self.ctx.modules[self.index];
        for &cell in module.cells {
            if matches!(*cell.0, nir::Cell::Input(_)) {
                if self.index != 0 {
                    errors.push(LowerError::InputCellInChild {
                        module: self.module,
                        cell: self.cell_index(cell),
                    });
                }
                continue;
            }
            let output = self.wire(self.width(cell));
            self.outputs.insert(cell, output);
            for bit in 0..self.width(cell) {
                self.nets
                    .insert(Net::Cell { cell, bit }, self.select(output, bit, 1));
            }
        }
        let mut port_wires = Vec::new();
        for (index, port) in module.ports.iter().enumerate() {
            let Ok(width) = i32::try_from(port.value.0.len()) else {
                continue;
            };
            let Ok(id) = u32::try_from(index + 1) else {
                errors.push(LowerError::WidthOverflow {
                    module: self.module,
                    width: index + 1,
                });
                continue;
            };
            let wire = self.wire_named(
                ast::Id::Public(port.name),
                width as u32,
                Some(ast::WirePort {
                    id: id.try_into().unwrap(),
                    direction: match port.flow {
                        nir::Flow::In => ast::Direction::In,
                        nir::Flow::Out => ast::Direction::Out,
                    },
                }),
            );
            port_wires.push((port, wire));
            if port.flow != nir::Flow::In {
                continue;
            }
            for (bit, &net) in port.value.0.iter().enumerate() {
                let net = match self.canonical(net) {
                    Ok(n) => n,
                    Err(e) => {
                        errors.push(e);
                        continue;
                    }
                };
                let Net::Cell { cell, .. } = net else {
                    errors.push(LowerError::InvalidInputPort {
                        module: self.module,
                        port: port.name,
                        bit: bit as u32,
                    });
                    continue;
                };
                let owner = self.ctx.owners[&cell].0;
                if self.index == 0 && !(owner == 0 && matches!(*cell.0, nir::Cell::Input(_))) {
                    errors.push(LowerError::InvalidInputPort {
                        module: self.module,
                        port: port.name,
                        bit: bit as u32,
                    });
                    continue;
                }
                if self.index != 0 && self.ctx.inside(owner, self.index) {
                    errors.push(LowerError::InputPortDrivenInternally {
                        module: self.module,
                        port: port.name,
                    });
                    continue;
                }
                if self.nets.contains_key(&net) {
                    errors.push(LowerError::DuplicateInputBit {
                        module: self.module,
                        port: port.name,
                        bit: bit as u32,
                    });
                } else {
                    self.nets.insert(net, self.select(wire, bit as u32, 1));
                }
            }
        }
        if self.index == 0 {
            for &cell in module.cells {
                if matches!(*cell.0, nir::Cell::Input(_)) {
                    for bit in 0..self.width(cell) {
                        if !self.nets.contains_key(&Net::Cell { cell, bit }) {
                            errors.push(LowerError::UnboundInput {
                                module: self.module,
                                cell: self.cell_index(cell),
                                bit,
                            });
                        }
                    }
                }
            }
        }
        let children: Vec<_> = self
            .ctx
            .parents
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| (p == Some(self.index)).then_some(i))
            .collect();
        let mut child_ports = Vec::new();
        for child in children {
            let mut connections = Vec::new();
            for port in self.ctx.modules[child].ports {
                if port.flow != nir::Flow::Out {
                    continue;
                }
                let Ok(width) = i32::try_from(port.value.0.len()) else {
                    continue;
                };
                let wire = self.wire(width as u32);
                connections.push(ast::PortConnection {
                    port: ast::Id::Public(port.name),
                    value: wire,
                });
                for (bit, &net) in port.value.0.iter().enumerate() {
                    match self.canonical(net) {
                        Ok(net @ Net::Cell { cell, .. })
                            if self.ctx.inside(self.ctx.owners[&cell].0, child) =>
                        {
                            let value = self.select(wire, bit as u32, 1);
                            self.nets.entry(net).or_insert(value);
                        }
                        Ok(_) => {}
                        Err(e) => errors.push(e),
                    }
                }
            }
            child_ports.push((child, connections));
        }
        for (child, mut connections) in child_ports {
            for port in self.ctx.modules[child].ports {
                if port.flow == nir::Flow::In {
                    match self.sig(port.value) {
                        Ok(value) => connections.push(ast::PortConnection {
                            port: ast::Id::Public(port.name),
                            value,
                        }),
                        Err(e) => errors.push(e),
                    }
                }
            }
            let name = ast::Id::Public(self.ctx.modules[child].name);
            self.items.push(ast::ModuleItem::Cell(ast::Cell {
                name,
                ty: self.ctx.names[child],
                attributes: &[],
                parameters: &[],
                connections: self.bump.alloc_slice_copy(&connections),
            }));
        }
        for &cell in module.cells {
            let mut operands = Vec::new();
            match *cell.0 {
                nir::Cell::Input(_) => {}
                nir::Cell::Unary(x) => operands.extend_from_slice(x.value.0),
                nir::Cell::Binary(x) => {
                    operands.extend_from_slice(x.lhs.0);
                    operands.extend_from_slice(x.rhs.0);
                }
                nir::Cell::Mux(x) => {
                    operands.push(x.select);
                    operands.extend_from_slice(x.when_true.0);
                    operands.extend_from_slice(x.when_false.0);
                }
                nir::Cell::Match(x) => {
                    operands.push(x.enable);
                    operands.extend_from_slice(x.value.0);
                }
                nir::Cell::AssignmentList(x) => {
                    operands.extend_from_slice(x.default.0);
                    for assign in x.assigns {
                        operands.push(assign.condition);
                        operands.extend_from_slice(assign.value.0);
                    }
                }
                nir::Cell::FlipFlop(x) => {
                    operands.push(x.clock);
                    operands.push(x.async_reset);
                    operands.extend_from_slice(x.data.0);
                }
            }
            let mut invalid = false;
            for operand in operands {
                if let Err(e) = self.net(operand) {
                    errors.push(e);
                    invalid = true;
                }
            }
            if !invalid {
                if let Err(e) = self.lower_cell(cell) {
                    errors.push(e);
                }
            }
        }
        for (port, wire) in port_wires {
            if port.flow == nir::Flow::Out {
                match self.sig(port.value) {
                    Ok(value) => self.connect(wire, value),
                    Err(e) => errors.push(e),
                }
            }
        }
    }
}
