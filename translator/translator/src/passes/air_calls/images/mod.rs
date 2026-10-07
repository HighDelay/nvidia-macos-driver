use super::*;

mod sample;
pub(in crate::passes) use sample::*;
mod gather;
pub(in crate::passes) use gather::*;
mod sample_depth;
pub(in crate::passes) use sample_depth::*;
mod write_read;
pub(in crate::passes) use write_read::*;
mod atomic;
pub(in crate::passes) use atomic::*;
mod fetch_coord;
pub(in crate::passes) use fetch_coord::*;
mod query_offset;
pub(in crate::passes) use query_offset::*;
mod resolve;
pub(in crate::passes) use resolve::*;

pub(in crate::passes) fn sampled_operand_image_info(
    ctx: &mut Ctx,
    image: Word,
    dim: Dim,
    arrayed: bool,
    comp: crate::passes::ImageComp,
) -> (Word, Dim, bool, crate::passes::ImageComp) {
    if let Some((ty, dim, arrayed, comp)) = image_type_from_value(ctx, image) {
        return (ty, dim, arrayed, comp);
    }
    (ctx.ty_image(dim, arrayed, comp), dim, arrayed, comp)
}

pub(in crate::passes) fn image_shape_or_recorded(
    ctx: &Ctx,
    image: Word,
) -> (Dim, bool, crate::passes::ImageComp) {
    if let Some((_, dim, arrayed, comp)) = image_type_from_value_or_pointer(ctx, image) {
        return (dim, arrayed, comp);
    }
    if let Some((dim, arrayed, comp)) = selected_image_shape(ctx, image) {
        return (dim, arrayed, comp);
    }
    let (dim, arrayed) = ctx
        .image_dims
        .get(&image)
        .copied()
        .unwrap_or((Dim::Dim2D, false));
    let comp = ctx
        .image_comp
        .get(&image)
        .copied()
        .unwrap_or(crate::passes::ImageComp::Float);
    (dim, arrayed, comp)
}

fn selected_image_shape(ctx: &Ctx, image: Word) -> Option<(Dim, bool, crate::passes::ImageComp)> {
    let inst = value_inst(ctx, image)?;
    if inst.class.opcode != Op::Select {
        return None;
    }
    let mut arms = inst
        .operands
        .iter()
        .skip(1)
        .filter_map(|operand| match operand {
            Operand::IdRef(id) => image_type_from_value_or_pointer(ctx, *id)
                .map(|(_, dim, arrayed, comp)| (dim, arrayed, comp))
                .or_else(|| {
                    let (dim, arrayed) = ctx.image_dims.get(id).copied()?;
                    let comp = ctx
                        .image_comp
                        .get(id)
                        .copied()
                        .unwrap_or(crate::passes::ImageComp::Float);
                    Some((dim, arrayed, comp))
                }),
            _ => None,
        });
    let first = arms.next()?;
    if arms.all(|shape| shape == first) {
        Some(first)
    } else {
        None
    }
}

pub(in crate::passes) fn load_image_if_pointer(
    ctx: &mut Ctx,
    image: Word,
    out: &mut Vec<Instruction>,
) -> Word {
    let Some(ty) = value_result_type(ctx, image) else {
        return image;
    };
    let Some(pointee) = type_def_of(ctx, ty).and_then(|def| {
        if def.class.opcode != Op::TypePointer {
            return None;
        }
        match def.operands.get(1) {
            Some(Operand::IdRef(pointee)) => Some(*pointee),
            _ => None,
        }
    }) else {
        return image;
    };
    let Some((dim, arrayed, comp)) = image_type_info(ctx, pointee) else {
        return image;
    };
    let loaded = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::Load,
        Some(pointee),
        Some(loaded),
        vec![Operand::IdRef(image)],
    ));
    ctx.image_dims.insert(loaded, (dim, arrayed));
    ctx.image_comp.insert(loaded, comp);
    if ctx.image_multisampled.contains(&image) || image_type_is_multisampled(ctx, pointee) {
        ctx.image_multisampled.insert(loaded);
    }
    if ctx.image_storage.contains(&image) || image_type_is_storage(ctx, pointee) {
        ctx.image_storage.insert(loaded);
        if let Some((metal_index, state)) = ctx.runtime_storage_image_values.get(&image).copied() {
            ctx.register_runtime_storage_image_value(loaded, metal_index, Some(state));
        }
    }
    loaded
}

fn image_type_from_value(
    ctx: &Ctx,
    image: Word,
) -> Option<(Word, Dim, bool, crate::passes::ImageComp)> {
    let ty = value_result_type(ctx, image)?;
    let (dim, arrayed, comp) = image_type_info(ctx, ty)?;
    Some((ty, dim, arrayed, comp))
}

pub(in crate::passes) fn image_type_from_value_or_pointer(
    ctx: &Ctx,
    image: Word,
) -> Option<(Word, Dim, bool, crate::passes::ImageComp)> {
    if let Some(info) = image_type_from_value(ctx, image) {
        return Some(info);
    }
    let ty = value_result_type(ctx, image)?;
    let def = type_def_of(ctx, ty)?;
    if def.class.opcode != Op::TypePointer {
        return None;
    }
    let pointee = match def.operands.get(1)? {
        Operand::IdRef(pointee) => *pointee,
        _ => return None,
    };
    let (dim, arrayed, comp) = image_type_info(ctx, pointee)?;
    Some((pointee, dim, arrayed, comp))
}

fn image_type_info(ctx: &Ctx, image_ty: Word) -> Option<(Dim, bool, crate::passes::ImageComp)> {
    let def = type_def_of(ctx, image_ty)?;
    if def.class.opcode != Op::TypeImage {
        return None;
    }
    let sampled_ty = match def.operands.first()? {
        Operand::IdRef(id) => *id,
        _ => return None,
    };
    let dim = match def.operands.get(1)? {
        Operand::Dim(dim) => *dim,
        _ => return None,
    };
    let arrayed = match def.operands.get(3)? {
        Operand::LiteralBit32(value) => *value != 0,
        _ => return None,
    };
    let sampled_def = type_def_of(ctx, sampled_ty)?;
    let comp = match sampled_def.class.opcode {
        Op::TypeFloat => crate::passes::ImageComp::Float,
        Op::TypeInt => match sampled_def.operands.get(1) {
            Some(Operand::LiteralBit32(0)) => crate::passes::ImageComp::Uint,
            Some(Operand::LiteralBit32(_)) => crate::passes::ImageComp::Sint,
            _ => return None,
        },
        _ => return None,
    };
    Some((dim, arrayed, comp))
}

fn image_type_is_storage(ctx: &Ctx, image_ty: Word) -> bool {
    type_def_of(ctx, image_ty).is_some_and(|def| {
        def.class.opcode == Op::TypeImage
            && matches!(def.operands.get(5), Some(Operand::LiteralBit32(2)))
    })
}

pub(in crate::passes) fn image_value_is_multisampled(ctx: &Ctx, image: Word) -> bool {
    ctx.image_multisampled.contains(&image)
        || value_result_type(ctx, image).is_some_and(|ty| image_type_is_multisampled(ctx, ty))
}

fn image_type_is_multisampled(ctx: &Ctx, image_ty: Word) -> bool {
    type_def_of(ctx, image_ty).is_some_and(|def| {
        def.class.opcode == Op::TypeImage
            && matches!(def.operands.get(4), Some(Operand::LiteralBit32(1)))
    })
}

pub(in crate::passes) fn image_value_dim(ctx: &Ctx, image: Word) -> Option<spirv::Dim> {
    if let Some((dim, _)) = ctx.image_dims.get(&image) {
        return Some(*dim);
    }
    let def = type_def_of(ctx, value_result_type(ctx, image)?)?;
    match (def.class.opcode, def.operands.get(1)) {
        (Op::TypeImage, Some(Operand::Dim(dim))) => Some(*dim),
        _ => None,
    }
}

pub(in crate::passes) fn image_size_query_op(ctx: &Ctx, image: Word) -> Op {
    if image_is_storage(ctx, image)
        || image_value_is_multisampled(ctx, image)
        || image_value_dim(ctx, image) == Some(spirv::Dim::DimBuffer)
    {
        Op::ImageQuerySize
    } else {
        Op::ImageQuerySizeLod
    }
}

#[cfg(test)]
mod coord_tests {
    use super::*;
    use crate::spirv_module::Module;

    fn ctx_with_coord(vec_lanes: u32) -> (Ctx, Word, Word, Vec<Instruction>) {
        let mut ctx = Ctx::new(Module::new());
        let vty = ctx.ty_vecf(vec_lanes);
        let layer = ctx.const_uint(0);
        let coord = ctx.module.fresh_id();
        let out = vec![Instruction::new(Op::Undef, Some(vty), Some(coord), vec![])];
        (ctx, coord, layer, out)
    }

    #[test]
    fn build_sample_coord_non_arrayed_passes_coord_through() {
        for dim in [Dim::Dim1D, Dim::Dim2D, Dim::Dim3D, Dim::DimCube] {
            let (mut ctx, coord, _layer, mut out) = ctx_with_coord(2);
            let n_before = out.len();
            let got = build_sample_coord(&mut ctx, dim, false, coord, &[], &mut out).unwrap();
            assert_eq!(
                got, coord,
                "non-arrayed {dim:?} must return the coord unchanged"
            );
            assert_eq!(out.len(), n_before, "non-arrayed path emits nothing");
        }
    }

    #[test]
    fn build_sample_coord_2d_arrayed_appends_layer_as_vecf3() {
        let (mut ctx, coord, layer, mut out) = ctx_with_coord(2);
        let args = [0u32, 0, 0, layer];
        let combined =
            build_sample_coord(&mut ctx, Dim::Dim2D, true, coord, &args, &mut out).unwrap();
        let vf3 = ctx.ty_vecf(3);
        let last = out.last().unwrap();
        assert_eq!(last.class.opcode, Op::CompositeConstruct);
        assert_eq!(last.result_type, Some(vf3), "arrayed 2D coord is v3float");
        assert_eq!(last.result_id, Some(combined));
        assert_eq!(last.operands.len(), 3);
    }

    #[test]
    fn build_gather_coord_2d_matches_sample_coord_shape() {
        let (mut ctx_s, coord_s, layer_s, mut out_s) = ctx_with_coord(2);
        let args = [0u32, 0, 0, layer_s];
        let sample =
            build_sample_coord(&mut ctx_s, Dim::Dim2D, true, coord_s, &args, &mut out_s).unwrap();
        let sample_last = out_s.last().unwrap().clone();

        let (mut ctx_g, coord_g, layer_g, mut out_g) = ctx_with_coord(2);
        let gather =
            build_gather_coord_2d(&mut ctx_g, true, coord_g, Some(layer_g), &mut out_g).unwrap();
        let gather_last = out_g.last().unwrap();

        assert_eq!(sample_last.class.opcode, gather_last.class.opcode);
        assert_eq!(sample_last.result_type, gather_last.result_type);
        assert_eq!(sample_last.operands.len(), gather_last.operands.len());
        assert_eq!(out_s.len(), out_g.len());
        let _ = (sample, gather);
    }
}
