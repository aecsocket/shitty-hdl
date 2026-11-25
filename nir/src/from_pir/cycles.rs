use std::collections::{HashMap, hash_map::Entry};

use crate as nir;

use super::NetLocation;

pub(super) fn check_cycles<'d>(
    cells: &[(nir::CellRef<'d>, u32)],
    connections: &[nir::Connection<'d>],
) -> Vec<super::LowerError> {
    let mut errors = Vec::new();
    let widths: HashMap<_, _> = cells
        .iter()
        .enumerate()
        .map(|(index, &(cell, width))| (cell, (index, width)))
        .collect();
    let mut definitions = HashMap::new();
    for connection in connections {
        match definitions.entry(connection.from) {
            Entry::Occupied(_) => {
                errors.push(super::LowerError::DuplicateLateNetDefinition {
                    name: connection.from.0.name,
                });
            }
            Entry::Vacant(entry) => {
                entry.insert(connection.to);
            }
        }
    }
    let mut colors = HashMap::new();
    let mut stack = Vec::new();
    let roots = cells
        .iter()
        .flat_map(|&(cell, width)| (0..width).map(move |bit| nir::Net::Cell { cell, bit }))
        .chain(
            connections
                .iter()
                .map(|connection| nir::Net::Late(connection.from)),
        );
    for root in roots {
        stack.push((root, false));
        while let Some((net, exit)) = stack.pop() {
            if exit {
                colors.insert(net, 2u8);
                continue;
            }
            match colors.get(&net) {
                Some(2) => continue,
                Some(1) => {
                    let at = match net {
                        nir::Net::Const(value) => NetLocation::Const(value),
                        nir::Net::Late(late) => NetLocation::Late { name: late.0.name },
                        nir::Net::Cell { cell, bit } => NetLocation::Cell {
                            cell: widths[&cell].0,
                            bit,
                        },
                    };
                    errors.push(super::LowerError::CombinationalCycle { at });
                    continue;
                }
                _ => {}
            }
            colors.insert(net, 1);
            stack.push((net, true));
            match net {
                nir::Net::Const(_) => {}
                nir::Net::Late(late) => {
                    if let Some(target) = definitions.get(&late) {
                        stack.push((*target, false));
                    } else {
                        errors.push(super::LowerError::MissingLateNetDefinition {
                            name: late.0.name,
                        });
                    }
                }
                nir::Net::Cell { cell, bit } => {
                    let Some(&(index, width)) = widths.get(&cell) else {
                        errors.push(super::LowerError::UnknownCell);
                        continue;
                    };
                    if bit >= width {
                        errors.push(super::LowerError::CellBitOutOfRange {
                            cell: index,
                            bit,
                            width,
                        });
                        continue;
                    }
                    let mut dependencies = Vec::new();
                    errors.extend(dependencies_for_bit(cell, bit, &mut dependencies).err());
                    stack.extend(dependencies.into_iter().map(|net| (net, false)));
                }
            }
        }
    }
    errors
}

fn value_bit(value: nir::Value<'_>, bit: u32) -> Result<nir::Net<'_>, super::LowerError> {
    value
        .0
        .get(bit as usize)
        .copied()
        .ok_or(super::LowerError::OperandBitOutOfRange {
            bit,
            width: value.0.len(),
        })
}

fn dependencies_for_bit<'d>(
    cell: nir::CellRef<'d>,
    bit: u32,
    dependencies: &mut Vec<nir::Net<'d>>,
) -> Result<(), super::LowerError> {
    let index = bit as usize;
    match *cell.0 {
        nir::Cell::Input(_) | nir::Cell::FlipFlop(_) => {}
        nir::Cell::Unary(unary) => match unary.op {
            nir::UnaryOp::Not => dependencies.push(value_bit(unary.value, bit)?),
            nir::UnaryOp::Neg => {
                dependencies.extend(unary.value.0.iter().take(index + 1).copied());
            }
            _ => dependencies.extend_from_slice(unary.value.0),
        },
        nir::Cell::Binary(binary) => match binary.op {
            nir::BinaryOp::Add | nir::BinaryOp::Sub | nir::BinaryOp::Mul => {
                dependencies.extend(binary.lhs.0.iter().take(index + 1).copied());
                dependencies.extend(binary.rhs.0.iter().take(index + 1).copied());
            }
            nir::BinaryOp::And | nir::BinaryOp::Or | nir::BinaryOp::Xor => {
                dependencies.push(value_bit(binary.lhs, bit)?);
                dependencies.push(value_bit(binary.rhs, bit)?);
            }
            nir::BinaryOp::Shl | nir::BinaryOp::Shr(_)
                if binary
                    .rhs
                    .0
                    .iter()
                    .all(|net| matches!(net, nir::Net::Const(_))) =>
            {
                let amount = binary.rhs.0.iter().rev().fold(0usize, |amount, net| {
                    amount
                        .saturating_mul(2)
                        .saturating_add(usize::from(*net == nir::Net::ONE))
                });
                let source = match binary.op {
                    nir::BinaryOp::Shl => index.checked_sub(amount),
                    _ => index.checked_add(amount),
                };
                if let Some(net) = source.and_then(|source| binary.lhs.0.get(source)) {
                    dependencies.push(*net);
                } else if matches!(binary.op, nir::BinaryOp::Shr(nir::Signedness::Signed))
                    && let Some(sign) = binary.lhs.0.last()
                {
                    dependencies.push(*sign);
                }
            }
            _ => {
                dependencies.extend_from_slice(binary.lhs.0);
                dependencies.extend_from_slice(binary.rhs.0);
            }
        },
        nir::Cell::Mux(mux) => {
            if let nir::Net::Const(select) = mux.select {
                dependencies.push(value_bit(
                    if select {
                        mux.when_true
                    } else {
                        mux.when_false
                    },
                    bit,
                )?);
            } else {
                let when_true = value_bit(mux.when_true, bit)?;
                let when_false = value_bit(mux.when_false, bit)?;
                dependencies.push(when_true);
                if when_true != when_false {
                    dependencies.push(when_false);
                    dependencies.push(mux.select);
                }
            }
        }
        nir::Cell::Match(matcher) => {
            if matcher.enable != nir::Net::ZERO {
                dependencies.push(matcher.enable);
                dependencies.extend_from_slice(matcher.value.0);
            }
        }
        nir::Cell::AssignmentList(list) => {
            let mut needs_default = true;
            for assignment in list.assigns.iter().rev() {
                if assignment.condition == nir::Net::ZERO {
                    continue;
                }
                let Some(offset) = bit.checked_sub(assignment.offset) else {
                    continue;
                };
                let Some(value) = assignment.value.0.get(offset as usize) else {
                    continue;
                };
                dependencies.push(*value);
                if assignment.condition == nir::Net::ONE {
                    needs_default = false;
                    break;
                }
                dependencies.push(assignment.condition);
            }
            if needs_default {
                dependencies.push(value_bit(list.default, bit)?);
            }
        }
    }
    Ok(())
}
