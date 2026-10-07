use super::*;

pub(in crate::passes) fn query_image_size(
    ctx: &mut Ctx,
    img: Word,
    spatial: usize,
    arrayed: bool,
    lod: Word,
    out: &mut Vec<Instruction>,
) -> Word {
    let result_ty = if spatial == 1 && !arrayed {
        ctx.ty_uint()
    } else {
        ctx.ty_vec_uint((spatial + usize::from(arrayed)) as u32)
    };
    let size = ctx.module.fresh_id();
    let query_op = image_size_query_op(ctx, img);
    let mut ops = vec![Operand::IdRef(img)];
    if query_op == Op::ImageQuerySizeLod {
        ops.push(Operand::IdRef(lod));
    }
    out.push(Instruction::new(query_op, Some(result_ty), Some(size), ops));
    size
}

pub(in crate::passes) fn push_image_read_or_fetch(
    ctx: &mut Ctx,
    out: &mut Vec<Instruction>,
    img: Word,
    coord: Word,
    lod: Option<Word>,
    result_ty: Word,
    result: Word,
) -> Result<(), String> {
    let mut ops = vec![Operand::IdRef(img), Operand::IdRef(coord)];
    let op = if image_is_storage(ctx, img) {
        ctx.require_runtime_storage_image_use(img, RuntimeStorageImageUse::Read)?;
        Op::ImageRead
    } else {
        if let Some(lod) = lod {
            ops.push(Operand::ImageOperands(spirv::ImageOperands::LOD));
            ops.push(Operand::IdRef(lod));
        }
        Op::ImageFetch
    };
    out.push(Instruction::new(op, Some(result_ty), Some(result), ops));
    Ok(())
}

pub(in crate::passes) fn image_size_component(
    ctx: &mut Ctx,
    size: Word,
    idx: usize,
    spatial: usize,
    arrayed: bool,
    out: &mut Vec<Instruction>,
) -> Result<Word, String> {
    if spatial == 1 && !arrayed {
        return Ok(size);
    }
    let component = ctx.module.fresh_id();
    let uint = ctx.ty_uint();
    out.push(Instruction::new(
        Op::CompositeExtract,
        Some(uint),
        Some(component),
        vec![Operand::IdRef(size), Operand::LiteralBit32(idx as u32)],
    ));
    Ok(component)
}

pub(in crate::passes) fn logical_not(
    ctx: &mut Ctx,
    value: Word,
    out: &mut Vec<Instruction>,
) -> Word {
    let result = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::LogicalNot,
        Some(ctx.ty_bool()),
        Some(result),
        vec![Operand::IdRef(value)],
    ));
    result
}

pub(in crate::passes) fn logical_and(
    ctx: &mut Ctx,
    lhs: Word,
    rhs: Word,
    out: &mut Vec<Instruction>,
) -> Word {
    let result = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::LogicalAnd,
        Some(ctx.ty_bool()),
        Some(result),
        vec![Operand::IdRef(lhs), Operand::IdRef(rhs)],
    ));
    result
}

pub(in crate::passes) fn sample_const_or_dynamic_offset(
    ctx: &Ctx,
    arrayed: bool,
    args: &[Word],
    expected_lanes: u32,
) -> Result<(Option<Vec<i32>>, Option<Word>), String> {
    let Some(arg) = find_sample_offset_arg(ctx, arrayed, args, expected_lanes) else {
        return Ok((None, None));
    };
    let constant = match expected_lanes {
        1 => const_i32_components::<1>(ctx, arg, 1).map(|v| v.map(|v| v.to_vec())),
        2 => const_i32_components::<2>(ctx, arg, 2).map(|v| v.map(|v| v.to_vec())),
        3 => const_i32_components::<3>(ctx, arg, 3).map(|v| v.map(|v| v.to_vec())),
        _ => Err("air.sample_texture pixel offset has unsupported lane count".into()),
    };
    match constant {
        Ok(Some(offset)) => Ok((Some(offset), None)),
        Ok(None) | Err(_) => Ok((None, Some(arg))),
    }
}

pub(in crate::passes) fn find_sample_offset_arg(
    ctx: &Ctx,
    arrayed: bool,
    args: &[Word],
    expected_lanes: u32,
) -> Option<Word> {
    let start = if arrayed { 4 } else { 3 };
    let rest = args.get(start..)?;
    for &arg in rest {
        let Some(ty) = value_result_type(ctx, arg) else {
            continue;
        };
        let Some(def) = type_def_of(ctx, ty) else {
            continue;
        };
        if def.class.opcode != Op::TypeVector
            || !matches!(
                def.operands.get(1),
                Some(Operand::LiteralBit32(lanes)) if *lanes == expected_lanes
            )
        {
            continue;
        }
        let Some(Operand::IdRef(elem)) = def.operands.first() else {
            continue;
        };
        if type_def_of(ctx, *elem)
            .map(|elem| elem.class.opcode != Op::TypeInt)
            .unwrap_or(true)
        {
            continue;
        }
        return Some(arg);
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub(in crate::passes) fn apply_dynamic_sample_offset(
    ctx: &mut Ctx,
    img: Word,
    arrayed: bool,
    coord: Word,
    offset: Word,
    spatial: usize,
    out: &mut Vec<Instruction>,
) -> Result<Word, String> {
    let coord_lanes = spatial + usize::from(arrayed);
    let coord_components = sample_coord_components(ctx, coord, coord_lanes as u32, out)?;
    let offset_components = dynamic_i32_offset_components(ctx, offset, spatial, out)?;
    let lod = ctx.const_uint(0);
    let size = query_image_size(ctx, img, spatial, arrayed, lod, out);
    let mut adjusted = Vec::with_capacity(coord_lanes);
    for idx in 0..spatial {
        let Operand::IdRef(coord_component) = coord_components[idx] else {
            return Err("air.sample_texture coord component is not an id".into());
        };
        let size_component = image_size_component(ctx, size, idx, spatial, arrayed, out)?;
        let size_f = ctx.module.fresh_id();
        out.push(Instruction::new(
            Op::ConvertUToF,
            Some(ctx.ty_float()),
            Some(size_f),
            vec![Operand::IdRef(size_component)],
        ));
        let delta = ctx.module.fresh_id();
        out.push(Instruction::new(
            Op::FDiv,
            Some(ctx.ty_float()),
            Some(delta),
            vec![
                Operand::IdRef(offset_components[idx]),
                Operand::IdRef(size_f),
            ],
        ));
        let shifted = ctx.module.fresh_id();
        out.push(Instruction::new(
            Op::FAdd,
            Some(ctx.ty_float()),
            Some(shifted),
            vec![Operand::IdRef(coord_component), Operand::IdRef(delta)],
        ));
        adjusted.push(Operand::IdRef(shifted));
    }
    if arrayed {
        adjusted.push(coord_components[spatial].clone());
    }
    if adjusted.len() == 1 {
        let Operand::IdRef(coord) = adjusted[0] else {
            return Err("air.sample_texture: adjusted sample coord component is not an id".into());
        };
        return Ok(coord);
    }
    let adjusted_coord = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::CompositeConstruct,
        Some(ctx.ty_vecf(adjusted.len() as u32)),
        Some(adjusted_coord),
        adjusted,
    ));
    Ok(adjusted_coord)
}

pub(in crate::passes) fn dynamic_i32_offset_components(
    ctx: &mut Ctx,
    offset: Word,
    spatial: usize,
    out: &mut Vec<Instruction>,
) -> Result<Vec<Word>, String> {
    let ty = value_result_type(ctx, offset).ok_or("air.sample_texture offset has no type")?;
    let signed = resolve::integer_is_signed(ctx, ty)
        .ok_or("air.sample_texture offset is not an integer vector")?;
    let def = type_def_of(ctx, ty).ok_or("air.sample_texture offset type is undefined")?;
    let Op::TypeVector = def.class.opcode else {
        return Err("air.sample_texture dynamic offset is not a vector".into());
    };
    let Some(Operand::LiteralBit32(lanes)) = def.operands.get(1) else {
        return Err("air.sample_texture dynamic offset vector missing length".into());
    };
    if *lanes as usize != spatial {
        return Err("air.sample_texture dynamic offset has unexpected lane count".into());
    }
    let Some(Operand::IdRef(elem_ty)) = def.operands.first() else {
        return Err("air.sample_texture dynamic offset vector missing element type".into());
    };
    let mut components = Vec::with_capacity(spatial);
    for idx in 0..spatial {
        let raw = ctx.module.fresh_id();
        out.push(Instruction::new(
            Op::CompositeExtract,
            Some(*elem_ty),
            Some(raw),
            vec![Operand::IdRef(offset), Operand::LiteralBit32(idx as u32)],
        ));
        let converted = ctx.module.fresh_id();
        out.push(Instruction::new(
            if signed {
                Op::ConvertSToF
            } else {
                Op::ConvertUToF
            },
            Some(ctx.ty_float()),
            Some(converted),
            vec![Operand::IdRef(raw)],
        ));
        components.push(converted);
    }
    Ok(components)
}

pub(in crate::passes) fn dynamic_i32_integer_offset_components(
    ctx: &mut Ctx,
    offset: Word,
    spatial: usize,
    out: &mut Vec<Instruction>,
) -> Result<Vec<Word>, String> {
    let ty = value_result_type(ctx, offset).ok_or("air.sample_texture offset has no type")?;
    let def = type_def_of(ctx, ty).ok_or("air.sample_texture offset type is undefined")?;
    let Op::TypeVector = def.class.opcode else {
        return Err("air.sample_texture dynamic offset is not a vector".into());
    };
    let Some(Operand::LiteralBit32(lanes)) = def.operands.get(1) else {
        return Err("air.sample_texture dynamic offset vector missing length".into());
    };
    if *lanes as usize != spatial {
        return Err("air.sample_texture dynamic offset has unexpected lane count".into());
    }
    let Some(Operand::IdRef(elem_ty)) = def.operands.first() else {
        return Err("air.sample_texture dynamic offset vector missing element type".into());
    };
    let sint = ctx.ty_sint();
    let mut components = Vec::with_capacity(spatial);
    for idx in 0..spatial {
        let raw = ctx.module.fresh_id();
        out.push(Instruction::new(
            Op::CompositeExtract,
            Some(*elem_ty),
            Some(raw),
            vec![Operand::IdRef(offset), Operand::LiteralBit32(idx as u32)],
        ));
        components.push(coerce_same_shape_integer(ctx, out, raw, *elem_ty, sint)?);
    }
    Ok(components)
}

pub(in crate::passes) fn const_bool_value(ctx: &Ctx, value: Word) -> Option<bool> {
    match value_inst(ctx, value)?.class.opcode {
        Op::ConstantTrue => Some(true),
        Op::ConstantFalse => Some(false),
        _ => None,
    }
}

pub(in crate::passes) fn sample_layer_to_uint(
    ctx: &mut Ctx,
    layer: Word,
    out: &mut Vec<Instruction>,
) -> Result<Word, String> {
    let ty = value_result_type(ctx, layer).ok_or("air.sample_texture layer has no type")?;
    let def = type_def_of(ctx, ty).ok_or("air.sample_texture layer type is undefined")?;
    match def.class.opcode {
        Op::TypeInt => {
            let uint = ctx.ty_uint();
            if ty == uint {
                return Ok(layer);
            }
            let layer_u = ctx.module.fresh_id();
            let op = if scalar_bit_width(ctx, ty) == 32 {
                Op::Bitcast
            } else {
                Op::UConvert
            };
            out.push(Instruction::new(
                op,
                Some(uint),
                Some(layer_u),
                vec![Operand::IdRef(layer)],
            ));
            Ok(layer_u)
        }
        Op::TypeFloat => {
            let layer_u = ctx.module.fresh_id();
            out.push(Instruction::new(
                Op::ConvertFToU,
                Some(ctx.ty_uint()),
                Some(layer_u),
                vec![Operand::IdRef(layer)],
            ));
            Ok(layer_u)
        }
        _ => Err("air.sample_texture layer is not scalar int/float".into()),
    }
}

pub(in crate::passes) fn sample_lod_to_fetch_lod(
    ctx: &mut Ctx,
    lod: Word,
    out: &mut Vec<Instruction>,
) -> Result<Word, String> {
    let ty = value_result_type(ctx, lod).ok_or("air.sample_texture lod has no type")?;
    let def = type_def_of(ctx, ty).ok_or("air.sample_texture lod type is undefined")?;
    match def.class.opcode {
        Op::TypeInt => coerce_image_coord32(ctx, lod, out, "air.sample_texture lod"),
        Op::TypeFloat => {
            let lod_u = ctx.module.fresh_id();
            out.push(Instruction::new(
                Op::ConvertFToU,
                Some(ctx.ty_uint()),
                Some(lod_u),
                vec![Operand::IdRef(lod)],
            ));
            Ok(lod_u)
        }
        _ => Err("air.sample_texture lod is not scalar int/float".into()),
    }
}
