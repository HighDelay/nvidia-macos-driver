use super::*;

pub(in crate::passes) fn bind_kernel_uvec3_builtin(
    ctx: &mut Ctx,
    defs: &HashMap<Word, Instruction>,
    bindings: &mut Vec<(Word, ParamBinding)>,
    pid: Word,
    pty: Word,
    builtin: BuiltIn,
) {
    let var = bind_kernel_v3uint_builtin(ctx, builtin);
    bind_kernel_uvec3_builtin_var(ctx, defs, bindings, pid, pty, var);
}

pub(in crate::passes) fn bind_kernel_uvec3_builtin_var(
    ctx: &mut Ctx,
    defs: &HashMap<Word, Instruction>,
    bindings: &mut Vec<(Word, ParamBinding)>,
    pid: Word,
    pty: Word,
    var: Word,
) {
    let uint_ty = ctx.ty_uint();
    let v2u = ctx.ty_vec_uint(2);
    let v3u = ctx.ty_vec_uint(3);
    let vector_lanes = scalar_or_vector_component(defs, pty).and_then(|(_component, lanes)| lanes);
    if pty == v3u {
        bindings.push((pid, ParamBinding::LoadVar { var, ty: v3u }));
    } else if pty == v2u || vector_lanes == Some(2) {
        bindings.push((
            pid,
            ParamBinding::LoadVarVectorPrefix {
                var,
                vec_ty: v3u,
                scalar_ty: uint_ty,
                prefix_ty: v2u,
                out_ty: if pty == v2u { v2u } else { pty },
                lanes: 2,
            },
        ));
    } else if vector_lanes == Some(3) {
        bindings.push((
            pid,
            ParamBinding::LoadVarVectorPrefix {
                var,
                vec_ty: v3u,
                scalar_ty: uint_ty,
                prefix_ty: v3u,
                out_ty: pty,
                lanes: 3,
            },
        ));
    } else {
        let scalar_ty = if pty == uint_ty { pty } else { uint_ty };
        bindings.push((
            pid,
            ParamBinding::LoadVarComponent {
                var,
                vec_ty: v3u,
                scalar_ty,
                out_ty: pty,
                comp: 0,
            },
        ));
    }
}

pub(in crate::passes) fn const_kernel_local_size(
    ctx: &mut Ctx,
    defs: &HashMap<Word, Instruction>,
    ty: Word,
    values: [u32; 3],
) -> Option<Word> {
    let def = defs.get(&ty)?;
    match def.class.opcode {
        Op::TypeInt => Some(ctx.const_int_of(ty, values[0] as i64)),
        Op::TypeVector => {
            let component = match def.operands.first()? {
                Operand::IdRef(component) => *component,
                _ => return None,
            };
            if !matches!(
                defs.get(&component).map(|inst| inst.class.opcode),
                Some(Op::TypeInt)
            ) {
                return None;
            }
            let lanes = match def.operands.get(1)? {
                Operand::LiteralBit32(lanes @ 2..=3) => *lanes,
                _ => return None,
            };
            let constituents = values[..lanes as usize]
                .iter()
                .copied()
                .map(|value| ctx.const_int_of(component, value as i64))
                .collect();
            Some(ctx.const_composite(ty, constituents))
        }
        _ => None,
    }
}

pub(in crate::passes) fn const_ivec(ctx: &mut Ctx, ty: Word, values: &[i32]) -> Word {
    let int_ty = ctx.ty_sint();
    let constituents = values
        .iter()
        .copied()
        .map(|value| ctx.const_int_of(int_ty, value as i64))
        .collect();
    ctx.const_composite(ty, constituents)
}

pub(in crate::passes) fn is_raw_uint_buffer_block(
    defs: &HashMap<Word, Instruction>,
    ty: Word,
) -> bool {
    let Some(block) = defs.get(&ty) else {
        return false;
    };
    if block.class.opcode != Op::TypeStruct || block.operands.len() != 1 {
        return false;
    }
    let Some(Operand::IdRef(runtime)) = block.operands.first() else {
        return false;
    };
    let Some(runtime_def) = defs.get(runtime) else {
        return false;
    };
    if runtime_def.class.opcode != Op::TypeRuntimeArray {
        return false;
    }
    let Some(Operand::IdRef(elem)) = runtime_def.operands.first() else {
        return false;
    };
    let Some(elem_def) = defs.get(elem) else {
        return false;
    };
    elem_def.class.opcode == Op::TypeInt
        && elem_def.operands.first() == Some(&Operand::LiteralBit32(32))
        && elem_def.operands.get(1) == Some(&Operand::LiteralBit32(0))
}
