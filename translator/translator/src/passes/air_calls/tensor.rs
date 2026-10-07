use super::*;

pub(in crate::passes) fn lower_descriptor_size_tensor(
    ctx: &mut Ctx,
    name: &str,
    res: Option<Word>,
    rty: Option<Word>,
    args: &[Word],
) -> Result<Vec<Instruction>, String> {
    let (res, rty) = match (res, rty) {
        (Some(res), Some(rty)) => (res, rty),
        _ => return Err(format!("{name} has no result")),
    };
    if !is_int_scalar_width(ctx, rty, 16) {
        return Err(format!("{name} result is not i16"));
    }
    let [rank, index_bytes] = args else {
        return Err(format!("{name} expects 2 operands, got {}", args.len()));
    };
    for (ordinal, value) in args.iter().enumerate() {
        let ty = value_result_type(ctx, *value)
            .ok_or_else(|| format!("{name} operand {ordinal} has no type"))?;
        if ty != rty {
            return Err(format!(
                "{name} operand {ordinal} is not the result i16 type"
            ));
        }
    }

    let three = ctx.const_int_of(rty, 3);
    let one = ctx.const_int_of(rty, 1);
    let header = ctx.const_int_of(rty, 8);
    let round = ctx.const_int_of(rty, 7);
    let align_mask = ctx.const_int_of(rty, -8);
    let scaled_rank = ctx.module.fresh_id();
    let fields = ctx.module.fresh_id();
    let payload = ctx.module.fresh_id();
    let unaligned = ctx.module.fresh_id();
    let rounded = ctx.module.fresh_id();

    Ok(vec![
        Instruction::new(
            Op::IMul,
            Some(rty),
            Some(scaled_rank),
            vec![Operand::IdRef(*rank), Operand::IdRef(three)],
        ),
        Instruction::new(
            Op::IAdd,
            Some(rty),
            Some(fields),
            vec![Operand::IdRef(scaled_rank), Operand::IdRef(one)],
        ),
        Instruction::new(
            Op::IMul,
            Some(rty),
            Some(payload),
            vec![Operand::IdRef(fields), Operand::IdRef(*index_bytes)],
        ),
        Instruction::new(
            Op::IAdd,
            Some(rty),
            Some(unaligned),
            vec![Operand::IdRef(payload), Operand::IdRef(header)],
        ),
        Instruction::new(
            Op::IAdd,
            Some(rty),
            Some(rounded),
            vec![Operand::IdRef(unaligned), Operand::IdRef(round)],
        ),
        Instruction::new(
            Op::BitwiseAnd,
            Some(rty),
            Some(res),
            vec![Operand::IdRef(rounded), Operand::IdRef(align_mask)],
        ),
    ])
}
