use super::{Emitter, LowerError};
use crate::ast;

impl<'a, 'd> Emitter<'a, 'd> {
    pub(super) fn lower_cell(&mut self, cell: nir::CellRef<'d>) -> Result<(), LowerError> {
        let width = self.width(cell);
        match *cell.0 {
            nir::Cell::Input(_) => {}
            nir::Cell::Unary(unary) => {
                let a_width = self.cells_width(unary.value.0.len())?;
                let ty = match unary.op {
                    nir::UnaryOp::Neg => "$neg",
                    nir::UnaryOp::Not => "$not",
                    nir::UnaryOp::Bool => "$reduce_bool",
                    nir::UnaryOp::ReduceAnd => "$reduce_and",
                    nir::UnaryOp::ReduceOr => "$reduce_or",
                    nir::UnaryOp::ReduceXor => "$reduce_xor",
                };
                let reduction = !matches!(unary.op, nir::UnaryOp::Neg | nir::UnaryOp::Not);
                self.cells_operand_width(
                    cell,
                    if reduction { 1 } else { width },
                    if reduction { width } else { a_width },
                )?;
                if width == 0 {
                    return Ok(());
                }
                let y = self.output(cell);
                if a_width == 0 {
                    let value = self.constant(matches!(unary.op, nir::UnaryOp::ReduceAnd));
                    self.connect(y, value);
                } else {
                    let a = self.sig(unary.value)?;
                    let aw = self.cells_parameter_width(a_width)?;
                    let yw = self.cells_parameter_width(width)?;
                    self.builtin(
                        ty,
                        &[("A_SIGNED", 0), ("A_WIDTH", aw), ("Y_WIDTH", yw)],
                        &[("A", a), ("Y", y)],
                    );
                }
            }
            nir::Cell::Binary(binary) => {
                use nir::{BinaryOp as Op, Signedness::Signed};
                let a_width = self.cells_width(binary.lhs.0.len())?;
                let b_width = self.cells_width(binary.rhs.0.len())?;
                let comparison = matches!(
                    binary.op,
                    Op::Eq | Op::Ne | Op::Lt(_) | Op::Le(_) | Op::Gt(_) | Op::Ge(_)
                );
                let shift = matches!(binary.op, Op::Shl | Op::Shr(_));
                if !shift {
                    self.cells_operand_width(cell, a_width, b_width)?;
                }
                self.cells_operand_width(cell, if comparison { 1 } else { a_width }, width)?;
                if width == 0 {
                    return Ok(());
                }
                let y = self.output(cell);
                if comparison && a_width == 0 {
                    let value = self.constant(matches!(binary.op, Op::Eq | Op::Le(_) | Op::Ge(_)));
                    self.connect(y, value);
                    return Ok(());
                }
                let ty = match binary.op {
                    Op::Add => "$add",
                    Op::Sub => "$sub",
                    Op::Mul => "$mul",
                    Op::FloorDiv(_) => "$divfloor",
                    Op::Mod(_) => "$modfloor",
                    Op::Eq => "$eq",
                    Op::Ne => "$ne",
                    Op::Lt(_) => "$lt",
                    Op::Le(_) => "$le",
                    Op::Gt(_) => "$gt",
                    Op::Ge(_) => "$ge",
                    Op::And => "$and",
                    Op::Or => "$or",
                    Op::Xor => "$xor",
                    Op::Shl => "$shl",
                    Op::Shr(nir::Signedness::Unsigned) => "$shr",
                    Op::Shr(Signed) => "$sshr",
                };
                let signed = matches!(
                    binary.op,
                    Op::FloorDiv(Signed)
                        | Op::Mod(Signed)
                        | Op::Lt(Signed)
                        | Op::Le(Signed)
                        | Op::Gt(Signed)
                        | Op::Ge(Signed)
                        | Op::Shr(Signed)
                );
                let a = self.sig(binary.lhs)?;
                let b = self.sig(binary.rhs)?;
                let aw = self.cells_parameter_width(a_width)?;
                let bw = self.cells_parameter_width(b_width)?;
                let yw = self.cells_parameter_width(width)?;
                self.builtin(
                    ty,
                    &[
                        ("A_SIGNED", i32::from(signed)),
                        ("B_SIGNED", i32::from(signed && !shift)),
                        ("A_WIDTH", aw),
                        ("B_WIDTH", bw),
                        ("Y_WIDTH", yw),
                    ],
                    &[("A", a), ("B", b), ("Y", y)],
                );
            }
            nir::Cell::Mux(mux) => {
                let true_width = self.cells_width(mux.when_true.0.len())?;
                let false_width = self.cells_width(mux.when_false.0.len())?;
                self.cells_operand_width(cell, width, true_width)?;
                self.cells_operand_width(cell, width, false_width)?;
                if width == 0 {
                    return Ok(());
                }
                let a = self.sig(mux.when_false)?;
                let b = self.sig(mux.when_true)?;
                let s = self.net(mux.select)?;
                let y = self.output(cell);
                let parameter_width = self.cells_parameter_width(width)?;
                self.builtin(
                    "$mux",
                    &[("WIDTH", parameter_width)],
                    &[("A", a), ("B", b), ("S", s), ("Y", y)],
                );
            }
            nir::Cell::Match(matcher) => {
                let value_width = self.cells_width(matcher.value.0.len())?;
                let arm_width = self.cells_width(matcher.arms.len())?;
                self.cells_operand_width(cell, width, arm_width)?;
                for arm in matcher.arms {
                    for pattern in arm.patterns {
                        match *pattern {
                            nir::Pattern::Any => {}
                            nir::Pattern::Exact(bits) => {
                                self.cells_pattern_width(cell, value_width, bits.width())?
                            }
                            nir::Pattern::Masked { value, mask } => {
                                self.cells_pattern_width(cell, value_width, value.width())?;
                                self.cells_pattern_width(cell, value_width, mask.width())?;
                            }
                        }
                    }
                }
                if width == 0 {
                    return Ok(());
                }
                let value = self.sig(matcher.value)?;
                let mut remaining = self.net(matcher.enable)?;
                let output = self.output(cell);
                for (index, arm) in matcher.arms.iter().enumerate() {
                    let mut matched = self.constant(false);
                    for pattern in arm.patterns {
                        let test = match *pattern {
                            nir::Pattern::Any => self.constant(true),
                            nir::Pattern::Exact(bits) => {
                                if value_width == 0 {
                                    self.constant(true)
                                } else {
                                    let b = self.bits(bits);
                                    self.cells_binary("$eq", value, b, value_width, 1)?
                                }
                            }
                            nir::Pattern::Masked {
                                value: pattern,
                                mask,
                            } => {
                                if value_width == 0 {
                                    self.constant(true)
                                } else {
                                    let mask = self.bits(mask);
                                    let pattern = self.bits(pattern);
                                    let a = self.cells_binary(
                                        "$and",
                                        value,
                                        mask,
                                        value_width,
                                        value_width,
                                    )?;
                                    let b = self.cells_binary(
                                        "$and",
                                        pattern,
                                        mask,
                                        value_width,
                                        value_width,
                                    )?;
                                    self.cells_binary("$eq", a, b, value_width, 1)?
                                }
                            }
                        };
                        matched = self.cells_binary("$or", matched, test, 1, 1)?;
                    }
                    let active = self.cells_binary("$and", remaining, matched, 1, 1)?;
                    let index = self.cells_width(index)?;
                    let into = self.select(output, index, 1);
                    self.connect(into, active);
                    let not_matched = self.wire(1);
                    self.builtin(
                        "$not",
                        &[("A_SIGNED", 0), ("A_WIDTH", 1), ("Y_WIDTH", 1)],
                        &[("A", matched), ("Y", not_matched)],
                    );
                    remaining = self.cells_binary("$and", remaining, not_matched, 1, 1)?;
                }
            }
            nir::Cell::AssignmentList(assignments) => {
                let default_width = self.cells_width(assignments.default.0.len())?;
                self.cells_operand_width(cell, width, default_width)?;
                for assign in assignments.assigns {
                    let assign_width = self.cells_width(assign.value.0.len())?;
                    if assign
                        .offset
                        .checked_add(assign_width)
                        .is_none_or(|end| end > width)
                    {
                        return Err(LowerError::AssignmentOutOfBounds {
                            module: self.module,
                            cell: self.cell_index(cell),
                            offset: assign.offset,
                            width: assign_width,
                            output_width: width,
                        });
                    }
                }
                if width == 0 {
                    return Ok(());
                }
                let mut current = self.sig(assignments.default)?;
                for assign in assignments.assigns {
                    let assign_width = self.cells_width(assign.value.0.len())?;
                    if assign_width == 0 {
                        continue;
                    }
                    let a = self.select(current, assign.offset, assign_width);
                    let b = self.sig(assign.value)?;
                    let s = self.net(assign.condition)?;
                    let y = self.wire(assign_width);
                    let parameter_width = self.cells_parameter_width(assign_width)?;
                    self.builtin(
                        "$mux",
                        &[("WIDTH", parameter_width)],
                        &[("A", a), ("B", b), ("S", s), ("Y", y)],
                    );
                    let end = assign.offset + assign_width;
                    let upper = self.select(current, end, width - end);
                    let lower = self.select(current, 0, assign.offset);
                    current = self.concat(&[upper, y, lower]);
                }
                let output = self.output(cell);
                self.connect(output, current);
            }
            nir::Cell::FlipFlop(flip_flop) => {
                let data_width = self.cells_width(flip_flop.data.0.len())?;
                self.cells_operand_width(cell, width, data_width)?;
                if flip_flop.init.width() != width {
                    return Err(LowerError::InitialWidthMismatch {
                        module: self.module,
                        cell: self.cell_index(cell),
                        expected: width,
                        actual: flip_flop.init.width(),
                    });
                }
                if width == 0 {
                    return Ok(());
                }
                self.init(cell, flip_flop.init);
                let d = self.sig(flip_flop.data)?;
                let clk = self.net(flip_flop.clock)?;
                let q = self.output(cell);
                let parameter_width = self.cells_parameter_width(width)?;
                let polarity = i32::from(matches!(flip_flop.edge, nir::Edge::Pos));
                if matches!(flip_flop.async_reset, nir::Net::Const(false)) {
                    self.builtin(
                        "$dff",
                        &[("WIDTH", parameter_width), ("CLK_POLARITY", polarity)],
                        &[("D", d), ("CLK", clk), ("Q", q)],
                    );
                } else {
                    let arst = self.net(flip_flop.async_reset)?;
                    let states: Vec<_> = (0..width)
                        .map(|bit| {
                            if flip_flop.init.words()[bit as usize / 64] & (1u64 << (bit % 64)) == 0
                            {
                                ast::State::Zero
                            } else {
                                ast::State::One
                            }
                        })
                        .collect();
                    let reset = ast::Const::Bits(self.bump.alloc_slice_copy(&states));
                    self.builtin_values(
                        "$adff",
                        &[
                            ("WIDTH", ast::Const::Integer(parameter_width)),
                            ("CLK_POLARITY", ast::Const::Integer(polarity)),
                            ("ARST_POLARITY", ast::Const::Integer(1)),
                            ("ARST_VALUE", reset),
                        ],
                        &[("D", d), ("CLK", clk), ("ARST", arst), ("Q", q)],
                    );
                }
            }
        }
        Ok(())
    }

    fn cells_width(&self, width: usize) -> Result<u32, LowerError> {
        i32::try_from(width).map_err(|_| LowerError::WidthOverflow {
            module: self.module,
            width,
        })?;
        u32::try_from(width).map_err(|_| LowerError::WidthOverflow {
            module: self.module,
            width,
        })
    }

    fn cells_parameter_width(&self, width: u32) -> Result<i32, LowerError> {
        i32::try_from(width).map_err(|_| LowerError::WidthOverflow {
            module: self.module,
            width: width as usize,
        })
    }

    fn cells_operand_width(
        &self,
        cell: nir::CellRef<'d>,
        expected: u32,
        actual: u32,
    ) -> Result<(), LowerError> {
        if expected != actual {
            return Err(LowerError::OperandWidthMismatch {
                module: self.module,
                cell: self.cell_index(cell),
                expected: expected as usize,
                actual: actual as usize,
            });
        }
        Ok(())
    }

    fn cells_pattern_width(
        &self,
        cell: nir::CellRef<'d>,
        expected: u32,
        actual: u32,
    ) -> Result<(), LowerError> {
        if expected != actual {
            return Err(LowerError::PatternWidthMismatch {
                module: self.module,
                cell: self.cell_index(cell),
                expected,
                actual,
            });
        }
        Ok(())
    }

    fn cells_binary(
        &mut self,
        ty: &str,
        a: ast::SigSpec<'d>,
        b: ast::SigSpec<'d>,
        width: u32,
        output_width: u32,
    ) -> Result<ast::SigSpec<'d>, LowerError> {
        let parameter_width = self.cells_parameter_width(width)?;
        let parameter_output_width = self.cells_parameter_width(output_width)?;
        let y = self.wire(output_width);
        self.builtin(
            ty,
            &[
                ("A_SIGNED", 0),
                ("B_SIGNED", 0),
                ("A_WIDTH", parameter_width),
                ("B_WIDTH", parameter_width),
                ("Y_WIDTH", parameter_output_width),
            ],
            &[("A", a), ("B", b), ("Y", y)],
        );
        Ok(y)
    }
}
