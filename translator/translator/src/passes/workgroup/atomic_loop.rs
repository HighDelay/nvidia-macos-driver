use super::*;
use crate::passes::access::const_u32;

const MAX_UNROLL_TRIP_COUNT: u32 = 256;

#[derive(Clone, Copy)]
struct PhiSpec {
    result: Word,
    init: Word,
    recur: Word,
}

struct LoopPlan {
    header_idx: usize,
    header_id: Word,
    preheader_idx: Option<usize>,
    merge_id: Word,
    trip_count: u32,
    phis: Vec<PhiSpec>,
    body_start: usize,
    body_end: usize,
}

pub(in crate::passes) fn unroll_small_workgroup_atomic_loops(ctx: &mut Ctx, entry_idx: usize) {
    while let Some(plan) = find_unroll_plan(ctx, entry_idx) {
        apply_unroll_plan(ctx, entry_idx, plan);
    }
}

fn find_unroll_plan(ctx: &Ctx, entry_idx: usize) -> Option<LoopPlan> {
    let func = &ctx.module.functions[entry_idx];
    for (header_idx, block) in func.blocks.iter().enumerate() {
        let header_id = block_label_id(block)?;
        if block.instructions.len() < 3 {
            continue;
        }
        let branch_idx = block.instructions.len() - 1;
        let merge_idx = block.instructions.len() - 2;
        let branch = &block.instructions[branch_idx];
        let loop_merge = &block.instructions[merge_idx];
        if branch.class.opcode != Op::BranchConditional || loop_merge.class.opcode != Op::LoopMerge
        {
            continue;
        }
        let (merge_id, continue_id) = match (id_ref_at(loop_merge, 0), id_ref_at(loop_merge, 1)) {
            (Some(merge_id), Some(continue_id)) => (merge_id, continue_id),
            _ => continue,
        };
        if !continue_branches_to_header(func, continue_id, header_id) {
            continue;
        }
        let body_start = block
            .instructions
            .iter()
            .take_while(|inst| inst.class.opcode == Op::Phi)
            .count();
        if body_start == 0 || body_start >= merge_idx {
            continue;
        }
        let body = &block.instructions[body_start..merge_idx];
        if body.iter().any(|inst| {
            matches!(
                inst.class.opcode,
                Op::Branch
                    | Op::BranchConditional
                    | Op::Switch
                    | Op::SelectionMerge
                    | Op::LoopMerge
                    | Op::ControlBarrier
                    | Op::MemoryBarrier
            )
        }) {
            continue;
        }
        if !body
            .iter()
            .any(|inst| is_workgroup_atomic_pointer_op(ctx, inst))
        {
            continue;
        }
        let phis = collect_phi_specs(&block.instructions[..body_start], continue_id)?;
        let trip_count = counted_trip_count(ctx, block, &phis, merge_id, continue_id)
            .or_else(|| bool_toggle_trip_count(ctx, block, &phis, merge_id, continue_id))?;
        if !(1..=MAX_UNROLL_TRIP_COUNT).contains(&trip_count) {
            continue;
        }
        return Some(LoopPlan {
            header_idx,
            header_id,
            preheader_idx: unique_unconditional_preheader(func, header_idx, header_id, continue_id),
            merge_id,
            trip_count,
            phis,
            body_start,
            body_end: merge_idx,
        });
    }
    None
}

fn apply_unroll_plan(ctx: &mut Ctx, entry_idx: usize, plan: LoopPlan) {
    let body = ctx.module.functions[entry_idx].blocks[plan.header_idx].instructions
        [plan.body_start..plan.body_end]
        .to_vec();
    let mut current: HashMap<Word, Word> =
        plan.phis.iter().map(|phi| (phi.result, phi.init)).collect();
    let mut unrolled = Vec::new();
    for _ in 0..plan.trip_count {
        let mut iteration_map = current.clone();
        for inst in &body {
            let mut cloned = inst.clone();
            for operand in &mut cloned.operands {
                remap_operand(operand, &iteration_map);
            }
            if cloned.class.opcode == Op::UConvert {
                if let (Some(old_result), Some(result_type), Some(value)) = (
                    inst.result_id,
                    cloned.result_type,
                    id_ref_at(&cloned, 0).and_then(|id| const_u32(ctx, id)),
                ) {
                    let constant = ctx.const_int_of(result_type, i64::from(value));
                    iteration_map.insert(old_result, constant);
                    continue;
                }
            }
            if let Some(old_result) = inst.result_id {
                let new_result = ctx.module.fresh_id();
                cloned.result_id = Some(new_result);
                iteration_map.insert(old_result, new_result);
            }
            unrolled.push(cloned);
        }
        for phi in &plan.phis {
            let next = iteration_map.get(&phi.recur).copied().unwrap_or(phi.recur);
            current.insert(phi.result, next);
        }
    }
    unrolled.push(Instruction::new(
        Op::Branch,
        None,
        None,
        vec![Operand::IdRef(plan.merge_id)],
    ));
    if let Some(preheader_idx) = plan.preheader_idx {
        let preheader = &mut ctx.module.functions[entry_idx].blocks[preheader_idx];
        match splice_unrolled_preheader(preheader, plan.header_id, unrolled) {
            Ok(()) => return,
            Err(retained) => unrolled = retained,
        }
    }
    let block = &mut ctx.module.functions[entry_idx].blocks[plan.header_idx];
    block.instructions = unrolled;
}

fn splice_unrolled_preheader(
    preheader: &mut Block,
    eliminated_header: Word,
    mut unrolled: Vec<Instruction>,
) -> Result<(), Vec<Instruction>> {
    if !preheader.instructions.last().is_some_and(|instruction| {
        instruction.class.opcode == Op::Branch
            && id_ref_at(instruction, 0) == Some(eliminated_header)
    }) {
        return Err(unrolled);
    }
    let Some(replacement_branch) = unrolled.pop() else {
        return Err(unrolled);
    };
    preheader.instructions.pop();
    let enclosing_loop_merge = preheader
        .instructions
        .last()
        .is_some_and(|instruction| instruction.class.opcode == Op::LoopMerge)
        .then(|| preheader.instructions.pop())
        .flatten();
    preheader.instructions.extend(unrolled);
    preheader.instructions.extend(enclosing_loop_merge);
    preheader.instructions.push(replacement_branch);
    Ok(())
}

fn collect_phi_specs(phis: &[Instruction], continue_id: Word) -> Option<Vec<PhiSpec>> {
    let mut specs = Vec::new();
    for phi in phis {
        if phi.class.opcode != Op::Phi || phi.operands.len() < 4 || phi.operands.len() % 2 != 0 {
            return None;
        }
        let result = phi.result_id?;
        let mut init = None;
        let mut recur = None;
        for pair in phi.operands.chunks_exact(2) {
            let value = match pair[0] {
                Operand::IdRef(id) => id,
                _ => return None,
            };
            let parent = match pair[1] {
                Operand::IdRef(id) => id,
                _ => return None,
            };
            if parent == continue_id {
                recur = Some(value);
            } else if init.replace(value).is_some() {
                return None;
            }
        }
        specs.push(PhiSpec {
            result,
            init: init?,
            recur: recur?,
        });
    }
    Some(specs)
}

fn counted_trip_count(
    ctx: &Ctx,
    block: &Block,
    phis: &[PhiSpec],
    merge_id: Word,
    continue_id: Word,
) -> Option<u32> {
    let branch = block.instructions.last()?;
    if id_ref_at(branch, 1) != Some(merge_id) || id_ref_at(branch, 2) != Some(continue_id) {
        return None;
    }
    let cond = id_ref_at(branch, 0)?;
    let cond_inst = block
        .instructions
        .iter()
        .find(|inst| inst.result_id == Some(cond))?;
    if cond_inst.class.opcode != Op::IEqual {
        return None;
    }
    let lhs = id_ref_at(cond_inst, 0)?;
    let rhs = id_ref_at(cond_inst, 1)?;
    let (next, trip_const) = const_u32(ctx, rhs).map_or_else(
        || const_u32(ctx, lhs).map(|trip| (rhs, trip)),
        |trip| Some((lhs, trip)),
    )?;
    let add = block
        .instructions
        .iter()
        .find(|inst| inst.result_id == Some(next) && inst.class.opcode == Op::IAdd)?;
    let add_lhs = id_ref_at(add, 0)?;
    let add_rhs = id_ref_at(add, 1)?;
    let counter = if const_u32(ctx, add_rhs) == Some(1) {
        add_lhs
    } else if const_u32(ctx, add_lhs) == Some(1) {
        add_rhs
    } else {
        return None;
    };
    let phi = phis.iter().find(|phi| phi.result == counter)?;
    if const_u32(ctx, phi.init) != Some(0) || phi.recur != next {
        return None;
    }
    Some(trip_const)
}

fn bool_toggle_trip_count(
    ctx: &Ctx,
    block: &Block,
    phis: &[PhiSpec],
    merge_id: Word,
    continue_id: Word,
) -> Option<u32> {
    let branch = block.instructions.last()?;
    if id_ref_at(branch, 1) != Some(continue_id) || id_ref_at(branch, 2) != Some(merge_id) {
        return None;
    }
    let cond = id_ref_at(branch, 0)?;
    let phi = phis.iter().find(|phi| phi.result == cond)?;
    if const_bool(ctx, phi.init) == Some(true) && const_bool(ctx, phi.recur) == Some(false) {
        Some(2)
    } else {
        None
    }
}

fn continue_branches_to_header(func: &Function, continue_id: Word, header_id: Word) -> bool {
    let Some(block) = func
        .blocks
        .iter()
        .find(|block| block_label_id(block) == Some(continue_id))
    else {
        return false;
    };
    let Some(term) = block.instructions.last() else {
        return false;
    };
    match term.class.opcode {
        Op::Branch => id_ref_at(term, 0) == Some(header_id),
        Op::BranchConditional => {
            id_ref_at(term, 1) == Some(header_id) || id_ref_at(term, 2) == Some(header_id)
        }
        _ => false,
    }
}

fn unique_unconditional_preheader(
    func: &Function,
    header_idx: usize,
    header_id: Word,
    continue_id: Word,
) -> Option<usize> {
    let mut found = None;
    for (idx, block) in func.blocks.iter().enumerate() {
        if idx == header_idx {
            continue;
        }
        if block_label_id(block) == Some(continue_id) {
            continue;
        }
        let branches_to_header = block.instructions.last().is_some_and(|inst| {
            inst.class.opcode == Op::Branch && id_ref_at(inst, 0) == Some(header_id)
        });
        if !branches_to_header {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(idx);
    }
    found
}

fn is_workgroup_atomic_pointer_op(ctx: &Ctx, inst: &Instruction) -> bool {
    if !matches!(
        inst.class.opcode,
        Op::AtomicLoad
            | Op::AtomicStore
            | Op::AtomicExchange
            | Op::AtomicCompareExchange
            | Op::AtomicCompareExchangeWeak
            | Op::AtomicIIncrement
            | Op::AtomicIDecrement
            | Op::AtomicIAdd
            | Op::AtomicISub
            | Op::AtomicSMin
            | Op::AtomicUMin
            | Op::AtomicSMax
            | Op::AtomicUMax
            | Op::AtomicAnd
            | Op::AtomicOr
            | Op::AtomicXor
    ) {
        return false;
    }
    let Some(ptr) = id_ref_at(inst, 0) else {
        return false;
    };
    let Some(ptr_ty) = value_result_type(ctx, ptr) else {
        return false;
    };
    let Some(ptr_def) = type_def_of(ctx, ptr_ty) else {
        return false;
    };
    ptr_def.class.opcode == Op::TypePointer
        && ptr_def.operands.first() == Some(&Operand::StorageClass(StorageClass::Workgroup))
}

fn block_label_id(block: &Block) -> Option<Word> {
    block.label.as_ref().and_then(|label| label.result_id)
}

pub(in crate::passes::workgroup) fn id_ref_at(inst: &Instruction, index: usize) -> Option<Word> {
    match inst.operands.get(index) {
        Some(Operand::IdRef(id)) => Some(*id),
        _ => None,
    }
}

fn const_bool(ctx: &Ctx, id: Word) -> Option<bool> {
    ctx.module
        .types_global_values
        .iter()
        .chain(ctx.new_globals.iter())
        .find(|inst| {
            matches!(inst.class.opcode, Op::ConstantTrue | Op::ConstantFalse)
                && inst.result_id == Some(id)
        })
        .map(|inst| inst.class.opcode == Op::ConstantTrue)
}

fn remap_operand(operand: &mut Operand, ids: &HashMap<Word, Word>) {
    match operand {
        Operand::IdRef(id) | Operand::IdScope(id) | Operand::IdMemorySemantics(id) => {
            if let Some(mapped) = ids.get(id) {
                *id = *mapped;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instruction(opcode: Op, result_id: Option<Word>, operands: Vec<Operand>) -> Instruction {
        Instruction::new(opcode, None, result_id, operands)
    }

    #[test]
    fn nested_atomic_unroll_keeps_enclosing_loop_merge_adjacent_to_branch() {
        let mut preheader = Block {
            label: Some(instruction(Op::Label, Some(1), vec![])),
            instructions: vec![
                instruction(
                    Op::IAdd,
                    Some(10),
                    vec![Operand::IdRef(8), Operand::IdRef(9)],
                ),
                instruction(
                    Op::LoopMerge,
                    None,
                    vec![
                        Operand::IdRef(5),
                        Operand::IdRef(4),
                        Operand::LoopControl(spirv::LoopControl::NONE),
                    ],
                ),
                instruction(Op::Branch, None, vec![Operand::IdRef(2)]),
            ],
        };
        let unrolled = vec![
            instruction(
                Op::AtomicIAdd,
                Some(11),
                vec![Operand::IdRef(20), Operand::IdRef(21)],
            ),
            instruction(Op::Branch, None, vec![Operand::IdRef(3)]),
        ];

        splice_unrolled_preheader(&mut preheader, 2, unrolled).expect("splice");

        assert_eq!(
            preheader
                .instructions
                .iter()
                .map(|instruction| instruction.class.opcode)
                .collect::<Vec<_>>(),
            vec![Op::IAdd, Op::AtomicIAdd, Op::LoopMerge, Op::Branch]
        );
        assert_eq!(
            id_ref_at(preheader.instructions.last().expect("branch"), 0),
            Some(3)
        );
    }
}
