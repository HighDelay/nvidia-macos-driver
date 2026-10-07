use super::*;

impl Emitter {
    pub(in crate::native::emitter) fn emit_branch(
        &mut self,
        term: &crate::native::tir::TirTerminator,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        match term {
            crate::native::tir::TirTerminator::Br(label) => {
                if let Some(loop_merge) = self.current_loop_merge() {
                    self.emit_loop_merge(&loop_merge, instructions)?;
                }
                let label_id = self.label_id(label.trim())?;
                instructions.push(Self::inst(
                    Op::Branch,
                    None,
                    None,
                    vec![Operand::IdRef(label_id)],
                ));
                Ok(())
            }
            crate::native::tir::TirTerminator::BrCond { cond, t, f } => {
                let cond = parse_value(cond)?;
                let true_label = t.trim();
                let false_label = f.trim();
                if let Some(loop_merge) = self.current_loop_merge() {
                    self.emit_loop_merge(&loop_merge, instructions)?;
                } else {
                    let header_merge = self
                        .current_block
                        .as_ref()
                        .and_then(|block| self.branch_merges_by_header.get(block))
                        .cloned();
                    let merge = if self.branch_merges_header_only {
                        header_merge
                    } else {
                        header_merge.or_else(|| {
                            self.branch_merges
                                .get(&(true_label.to_string(), false_label.to_string()))
                                .cloned()
                        })
                    };
                    if let Some(merge) = merge {
                        let merge_id = self.label_id(&merge)?;
                        instructions.push(Self::inst(
                            Op::SelectionMerge,
                            None,
                            None,
                            vec![
                                Operand::IdRef(merge_id),
                                Operand::SelectionControl(SelectionControl::NONE),
                            ],
                        ));
                    }
                }
                let cond_id = self.value_id(&cond, &LlType::Bool)?;
                instructions.push(Self::inst(
                    Op::BranchConditional,
                    None,
                    None,
                    vec![
                        Operand::IdRef(cond_id),
                        Operand::IdRef(self.label_id(true_label)?),
                        Operand::IdRef(self.label_id(false_label)?),
                    ],
                ));
                Ok(())
            }
            _ => Err(format!(
                "native emitter: emit_branch on non-branch terminator: {term:?}"
            )),
        }
    }

    pub(in crate::native::emitter) fn emit_terminator(
        &mut self,
        term: &crate::native::tir::TirTerminator,
        ret: &crate::native::tir::RetEmit,
        switch: &Option<crate::native::parse::LlSwitch>,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        use crate::native::tir::{RetEmit, TirTerminator};
        match term {
            TirTerminator::Br(_) | TirTerminator::BrCond { .. } => {
                self.emit_branch(term, instructions)
            }
            TirTerminator::Unreachable => {
                instructions.push(Self::inst(Op::Unreachable, None, None, vec![]));
                Ok(())
            }
            TirTerminator::Ret(v) => match ret {
                RetEmit::Void => {
                    instructions.push(Self::inst(Op::Return, None, None, vec![]));
                    Ok(())
                }
                RetEmit::Value(tv) => {
                    let id = self.value_id_in(&tv.value, &tv.ty, instructions)?;
                    instructions.push(Self::inst(
                        Op::ReturnValue,
                        None,
                        None,
                        vec![Operand::IdRef(id)],
                    ));
                    Ok(())
                }
                RetEmit::FromText => Err(format!(
                    "native emitter: ret operand did not parse into a typed value \
                     (reason=ret_operand_unparsed, value={v:?})"
                )),
            },
            TirTerminator::Switch {
                selector, default, ..
            } => match switch {
                Some(sw) => self.emit_switch_resolved(sw, instructions),
                None => Err(format!(
                    "native emitter: switch operands did not parse \
                     (reason=switch_operands_unparsed, selector={selector}, default={default})"
                )),
            },
        }
    }

    pub(in crate::native::emitter) fn current_loop_merge(&self) -> Option<LoopMergeInfo> {
        let block = self.current_block.as_ref()?;
        self.loop_merges.get(block).cloned()
    }

    pub(in crate::native::emitter) fn emit_loop_merge(
        &mut self,
        loop_merge: &LoopMergeInfo,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let (control, partial_count) = self
            .current_block
            .as_ref()
            .and_then(|header| self.loop_controls.get(header).copied())
            .unwrap_or((LoopControl::NONE, None));
        let mut operands = vec![
            Operand::IdRef(self.label_id(&loop_merge.merge)?),
            Operand::IdRef(self.label_id(&loop_merge.continue_target)?),
            Operand::LoopControl(control),
        ];
        if let Some(count) = partial_count {
            operands.push(Operand::LiteralBit32(count));
        }
        instructions.push(Self::inst(Op::LoopMerge, None, None, operands));
        Ok(())
    }

    pub(in crate::native::emitter) fn emit_switch_resolved(
        &mut self,
        switch: &crate::native::parse::LlSwitch,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let selector_ty = self.resolve_type(&switch.selector.ty)?;
        let selector_id = self.value_id_in(&switch.selector.value, &selector_ty, instructions)?;
        let block = self
            .current_block
            .as_ref()
            .ok_or_else(|| "native emitter: switch outside a block".to_string())?;
        let merge = self.switch_merges.get(block).cloned().ok_or_else(|| {
            format!("native emitter: could not infer structured merge for switch in {block}")
        })?;
        if let Some(loop_merge) = self.current_loop_merge() {
            self.emit_loop_merge(&loop_merge, instructions)?;
        }
        instructions.push(Self::inst(
            Op::SelectionMerge,
            None,
            None,
            vec![
                Operand::IdRef(self.label_id(&merge)?),
                Operand::SelectionControl(SelectionControl::NONE),
            ],
        ));
        let mut ops = vec![
            Operand::IdRef(selector_id),
            Operand::IdRef(self.label_id(&switch.default_label)?),
        ];
        for (value, label) in &switch.cases {
            ops.push(switch_literal_operand(value, &selector_ty)?);
            ops.push(Operand::IdRef(self.label_id(label)?));
        }
        instructions.push(Self::inst(Op::Switch, None, None, ops));
        Ok(())
    }
}
