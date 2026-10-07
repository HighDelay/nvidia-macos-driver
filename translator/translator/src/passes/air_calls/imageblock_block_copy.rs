use super::block_split::{labelled_block, CallSiteSplit};
use super::*;

pub(in crate::passes) fn lower_imageblock_block_copies(
    ctx: &mut Ctx,
    entry_idx: usize,
) -> Result<(), String> {
    loop {
        let names = air_names(&ctx.module);
        let Some(site) = find_block_copy_call(ctx, entry_idx, &names) else {
            return Ok(());
        };
        rewrite_block_copy(ctx, entry_idx, site)?;
    }
}

struct BlockCopySite {
    block: usize,
    inst: usize,
    call: Instruction,
    region: Option<[u32; 2]>,
}

fn find_block_copy_call(
    ctx: &Ctx,
    entry_idx: usize,
    names: &HashMap<Word, String>,
) -> Option<BlockCopySite> {
    for (block, blk) in ctx.module.functions[entry_idx].blocks.iter().enumerate() {
        for (inst, call) in blk.instructions.iter().enumerate() {
            if call.class.opcode != Op::FunctionCall {
                continue;
            }
            let Some(Operand::IdRef(callee)) = call.operands.first() else {
                continue;
            };
            if !names
                .get(callee)
                .is_some_and(|name| name.starts_with("air.write_imageblock_slice_to_texture"))
            {
                continue;
            }
            let args = call_args(call);
            if args.len() < 6 {
                continue;
            }
            let region = static_imageblock_region(ctx, &args);
            match region {
                Some([width, height])
                    if width.checked_mul(height).is_none_or(|cells| cells <= 1) =>
                {
                    continue
                }
                Some(_) => {}
                None if !imageblock_region_is_explicit(ctx, &args) => continue,
                None => {}
            }
            return Some(BlockCopySite {
                block,
                inst,
                call: call.clone(),
                region,
            });
        }
    }
    None
}

fn call_args(inst: &Instruction) -> Vec<Word> {
    inst.operands[1..]
        .iter()
        .filter_map(|operand| match operand {
            Operand::IdRef(id) => Some(*id),
            _ => None,
        })
        .collect()
}

pub(in crate::passes) fn imageblock_cell_chain(
    ctx: &Ctx,
    pointer: Word,
) -> Result<(Word, Word, Vec<Word>), String> {
    let mut indices: Vec<Word> = Vec::new();
    let mut current = pointer;
    while let Some(def) = value_def_instruction(ctx, current) {
        if !matches!(def.class.opcode, Op::AccessChain | Op::InBoundsAccessChain) {
            break;
        }
        let mut ids = def.operands.iter().filter_map(|operand| match operand {
            Operand::IdRef(id) => Some(*id),
            _ => None,
        });
        let Some(base) = ids.next() else {
            break;
        };
        let mut leading = ids.collect::<Vec<_>>();
        leading.append(&mut indices);
        indices = leading;
        current = base;
    }
    let storage = value_result_type(ctx, current)
        .filter(|_| !indices.is_empty())
        .and_then(|ty| ptr_storage(&type_defs(&ctx.module), ty));
    if storage != Some(StorageClass::Workgroup) {
        return Err(format!(
            "air.write_imageblock_slice_to_texture copies a block of cells, but its pointer does \
             not index a threadgroup imageblock cell array (it roots in {storage:?}), and this \
             translator has no other storage a neighbouring cell could come from"
        ));
    }
    Ok((current, indices.remove(0), indices))
}

fn imageblock_cell_array_len(ctx: &Ctx, cell_array: Word) -> Option<u32> {
    let ptr_ty = value_result_type(ctx, cell_array)?;
    let array = type_def_of(ctx, pointer_pointee_type(ctx, ptr_ty)?)?;
    if array.class.opcode != Op::TypeArray {
        return None;
    }
    let Some(Operand::IdRef(length)) = array.operands.get(1) else {
        return None;
    };
    match value_def_instruction(ctx, *length)?.operands.first() {
        Some(Operand::LiteralBit32(literal)) => Some(*literal),
        _ => None,
    }
}

pub(in crate::passes) fn imageblock_row_stride(ctx: &Ctx, cell_index: Word) -> Option<Word> {
    let add = value_def_instruction(ctx, cell_index)?;
    if add.class.opcode != Op::IAdd {
        return None;
    }
    let Some(Operand::IdRef(row)) = add.operands.first() else {
        return None;
    };
    let mul = value_def_instruction(ctx, *row)?;
    if mul.class.opcode != Op::IMul {
        return None;
    }
    match mul.operands.get(1) {
        Some(Operand::IdRef(width)) => Some(*width),
        _ => None,
    }
}

fn constant_splat(ctx: &mut Ctx, ty: Word, value: u32) -> Option<Word> {
    let def = type_def_of(ctx, ty)?;
    if def.class.opcode != Op::TypeVector {
        return None;
    }
    let Some(Operand::IdRef(elem)) = def.operands.first() else {
        return None;
    };
    let Some(Operand::LiteralBit32(lanes)) = def.operands.get(1) else {
        return None;
    };
    let (elem, lanes) = (*elem, *lanes);
    let component = ctx.get_or_create(Op::Constant, Some(elem), vec![Operand::LiteralBit32(value)]);
    Some(ctx.get_or_create(
        Op::ConstantComposite,
        Some(ty),
        vec![Operand::IdRef(component); lanes as usize],
    ))
}

fn rewrite_block_copy(ctx: &mut Ctx, entry_idx: usize, site: BlockCopySite) -> Result<(), String> {
    const WHAT: &str = "air.write_imageblock_slice_to_texture block copy";
    let BlockCopySite {
        block,
        inst,
        call,
        region,
    } = site;
    let args = call_args(&call);
    let extent = match region {
        Some([width, height]) => format!("{width}x{height}"),
        None => "runtime-sized".to_string(),
    };
    let multi_row = region.is_none_or(|[_, height]| height > 1);
    if let Some([width, height]) = region {
        width
            .checked_mul(height)
            .ok_or("air.write_imageblock_slice_to_texture block extent overflows a cell count")?;
    }

    let (cell_array, linearised_index, tail) = imageblock_cell_chain(ctx, args[1])?;
    let capacity = imageblock_cell_array_len(ctx, cell_array);
    if let (Some([width, height]), Some(capacity)) = (region, capacity) {
        if width * height > capacity {
            return Err(format!(
                "air.write_imageblock_slice_to_texture copies a {extent} block, but the \
                 imageblock holds {capacity} cells, so the block names cells that do not exist"
            ));
        }
    }
    let cell_ptr_ty = value_result_type(ctx, args[1])
        .ok_or("air.write_imageblock_slice_to_texture cell pointer has no result type")?;
    let row_stride = if multi_row {
        Some(imageblock_row_stride(ctx, linearised_index).ok_or_else(|| {
            format!(
                "air.write_imageblock_slice_to_texture copies a {extent} block, but its \
                 cell index is not the `y * width + x` the imageblock cell array is linearised by, \
                 so the stride from one block row to the next is unknown"
            )
        })?)
    } else {
        None
    };
    let coord_ty = value_result_type(ctx, args[5])
        .ok_or("air.write_imageblock_slice_to_texture destination coordinate has no type")?;
    let coord_lanes = vector_type_shape(ctx, coord_ty).map_or(1, |(_, lanes)| lanes);
    if coord_lanes < 2 && multi_row {
        return Err(format!(
            "air.write_imageblock_slice_to_texture copies a {extent} block to a texture \
             whose destination coordinate has {coord_lanes} component(s), which cannot name the \
             second block axis"
        ));
    }
    let region_ty = value_result_type(ctx, args[4])
        .ok_or("air.write_imageblock_slice_to_texture size operand has no type")?;
    let unit_region = constant_splat(ctx, region_ty, 1).ok_or_else(|| {
        format!(
            "air.write_imageblock_slice_to_texture size operand type %{region_ty} is not a vector"
        )
    })?;
    let flag_ty = value_result_type(ctx, args[2]).unwrap_or_else(|| ctx.ty_bool());
    let flag_true = ctx.const_bool_of(flag_ty, true);

    let mut split = CallSiteSplit::open(ctx, entry_idx, block, inst, WHAT)?;
    let uint = ctx.ty_uint();
    let bool_ty = ctx.ty_bool();
    let zero = ctx.const_uint(0);
    let one = ctx.const_uint(1);
    let (total, width_id) = match region {
        Some([width, height]) => (ctx.const_uint(width * height), ctx.const_uint(width)),
        None => {
            let region32 = if scalar_bit_width(ctx, region_ty) == 32 {
                args[4]
            } else {
                let id = ctx.module.fresh_id();
                let wide = ctx.ty_vec_uint(2);
                split.prefix.push(Instruction::new(
                    Op::UConvert,
                    Some(wide),
                    Some(id),
                    vec![Operand::IdRef(args[4])],
                ));
                id
            };
            let extract = |ctx: &mut Ctx, split: &mut CallSiteSplit, lane: u32| {
                let id = ctx.module.fresh_id();
                split.prefix.push(Instruction::new(
                    Op::CompositeExtract,
                    Some(uint),
                    Some(id),
                    vec![Operand::IdRef(region32), Operand::LiteralBit32(lane)],
                ));
                id
            };
            let width_id = extract(ctx, &mut split, 0);
            let height_id = extract(ctx, &mut split, 1);
            let total = ctx.module.fresh_id();
            split.prefix.push(Instruction::new(
                Op::IMul,
                Some(uint),
                Some(total),
                vec![Operand::IdRef(width_id), Operand::IdRef(height_id)],
            ));
            (total, width_id)
        }
    };
    let (coord, coord32_ty) = if scalar_bit_width(ctx, coord_ty) == 32 {
        (args[5], coord_ty)
    } else {
        let wide = if coord_lanes == 1 {
            uint
        } else {
            ctx.ty_vec_uint(coord_lanes)
        };
        let id = ctx.module.fresh_id();
        split.prefix.push(Instruction::new(
            Op::UConvert,
            Some(wide),
            Some(id),
            vec![Operand::IdRef(args[5])],
        ));
        (id, wide)
    };

    let entry_label = split.entry_label();
    let header = ctx.module.fresh_id();
    let cond = ctx.module.fresh_id();
    let body = ctx.module.fresh_id();
    let latch = ctx.module.fresh_id();
    let merge = ctx.module.fresh_id();
    split.branch_prefix_to(header);
    let continuation = split.continuation(ctx);

    let counter = ctx.module.fresh_id();
    let next_counter = ctx.module.fresh_id();
    let header_block = labelled_block(
        header,
        vec![
            Instruction::new(
                Op::Phi,
                Some(uint),
                Some(counter),
                vec![
                    Operand::IdRef(zero),
                    Operand::IdRef(entry_label),
                    Operand::IdRef(next_counter),
                    Operand::IdRef(latch),
                ],
            ),
            Instruction::new(
                Op::LoopMerge,
                None,
                None,
                vec![
                    Operand::IdRef(merge),
                    Operand::IdRef(latch),
                    Operand::LoopControl(spirv::LoopControl::NONE),
                ],
            ),
            Instruction::new(Op::Branch, None, None, vec![Operand::IdRef(cond)]),
        ],
    );

    let binary = |ctx: &mut Ctx, out: &mut Vec<Instruction>, op, ty, a: Word, b: Word| {
        let id = ctx.module.fresh_id();
        out.push(Instruction::new(
            op,
            Some(ty),
            Some(id),
            vec![Operand::IdRef(a), Operand::IdRef(b)],
        ));
        id
    };

    let mut cond_insts = Vec::new();
    let (dx, dy) = match row_stride {
        None => (counter, zero),
        Some(_) => (
            binary(ctx, &mut cond_insts, Op::UMod, uint, counter, width_id),
            binary(ctx, &mut cond_insts, Op::UDiv, uint, counter, width_id),
        ),
    };
    let offset = match row_stride {
        None => dx,
        Some(stride) => {
            let row = binary(ctx, &mut cond_insts, Op::IMul, uint, dy, stride);
            binary(ctx, &mut cond_insts, Op::IAdd, uint, row, dx)
        }
    };
    let cell_index = offset;
    let counted = binary(ctx, &mut cond_insts, Op::ULessThan, bool_ty, counter, total);
    let in_range = match capacity {
        None => counted,
        Some(capacity) => {
            let limit = ctx.const_uint(capacity);
            let fits = binary(
                ctx,
                &mut cond_insts,
                Op::ULessThan,
                bool_ty,
                cell_index,
                limit,
            );
            binary(ctx, &mut cond_insts, Op::LogicalAnd, bool_ty, counted, fits)
        }
    };
    cond_insts.push(Instruction::new(
        Op::BranchConditional,
        None,
        None,
        vec![
            Operand::IdRef(in_range),
            Operand::IdRef(body),
            Operand::IdRef(merge),
        ],
    ));
    let cond_block = labelled_block(cond, cond_insts);

    let mut body_insts = Vec::new();
    let cell = ctx.module.fresh_id();
    let mut chain = vec![Operand::IdRef(cell_array), Operand::IdRef(cell_index)];
    chain.extend(tail.into_iter().map(Operand::IdRef));
    body_insts.push(Instruction::new(
        Op::InBoundsAccessChain,
        Some(cell_ptr_ty),
        Some(cell),
        chain,
    ));
    let texel_coord = if coord_lanes == 1 {
        binary(ctx, &mut body_insts, Op::IAdd, coord32_ty, coord, dx)
    } else {
        let mut components = vec![Operand::IdRef(dx), Operand::IdRef(dy)];
        components.resize(coord_lanes as usize, Operand::IdRef(zero));
        let delta = ctx.module.fresh_id();
        body_insts.push(Instruction::new(
            Op::CompositeConstruct,
            Some(coord32_ty),
            Some(delta),
            components,
        ));
        binary(ctx, &mut body_insts, Op::IAdd, coord32_ty, coord, delta)
    };

    let mut residual = call.clone();
    for (arg, operand) in residual.operands.iter_mut().skip(1).enumerate() {
        match arg {
            1 => *operand = Operand::IdRef(cell),
            2 => *operand = Operand::IdRef(flag_true),
            4 => *operand = Operand::IdRef(unit_region),
            5 => *operand = Operand::IdRef(texel_coord),
            _ => {}
        }
    }
    body_insts.push(residual);
    body_insts.push(Instruction::new(
        Op::Branch,
        None,
        None,
        vec![Operand::IdRef(latch)],
    ));

    let latch_block = labelled_block(
        latch,
        vec![
            Instruction::new(
                Op::IAdd,
                Some(uint),
                Some(next_counter),
                vec![Operand::IdRef(counter), Operand::IdRef(one)],
            ),
            Instruction::new(Op::Branch, None, None, vec![Operand::IdRef(header)]),
        ],
    );
    let merge_block = labelled_block(
        merge,
        vec![Instruction::new(
            Op::Branch,
            None,
            None,
            vec![Operand::IdRef(continuation)],
        )],
    );

    split.finish(
        ctx,
        entry_idx,
        vec![
            header_block,
            cond_block,
            labelled_block(body, body_insts),
            latch_block,
            merge_block,
        ],
    );
    Ok(())
}
