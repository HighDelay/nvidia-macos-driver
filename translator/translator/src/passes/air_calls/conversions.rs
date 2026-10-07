use super::bfloat_glsl::{narrow_f32_to_bf16, widen_bf16_to_f32};
use super::*;

pub(in crate::passes) fn is_i1_type_token(tok: &str) -> bool {
    let t = tok.strip_prefix('v').map_or(tok, |rest| {
        rest.trim_start_matches(|c: char| c.is_ascii_digit())
    });
    t == "i1"
}

fn is_int_result(ctx: &Ctx, rty: Word) -> bool {
    let elem = element_type(ctx, rty);
    type_def_of(ctx, elem)
        .map(|d| d.class.opcode == Op::TypeInt)
        .unwrap_or(false)
}

fn vector_len_of_value(ctx: &Ctx, v: Word) -> u32 {
    value_result_type(ctx, v)
        .map(|t| vector_len(ctx, t))
        .unwrap_or(1)
}

fn int_splat_or_scalar(ctx: &mut Ctx, rty: Word, iv: i64, n: u32) -> Word {
    let elem = element_type(ctx, rty);
    let s = ctx.const_int_of(elem, iv);
    if n <= 1 {
        s
    } else {
        splat(ctx, rty, s, n)
    }
}

pub(super) fn lower_convert(
    ctx: &mut Ctx,
    name: &str,
    res: Word,
    rty: Word,
    args: &[Word],
) -> Result<Vec<Instruction>, String> {
    if args.len() == 1
        && type_def_of(ctx, rty).is_some_and(|definition| definition.class.opcode == Op::TypeArray)
    {
        let source_ty = value_result_type(ctx, args[0])
            .ok_or_else(|| format!("{name} wide source has no result type"))?;
        let (result_element, result_lanes) = composite_shape(ctx, rty)
            .ok_or_else(|| format!("{name} wide result has no fixed element shape"))?;
        let (source_element, source_lanes) = composite_shape(ctx, source_ty)
            .ok_or_else(|| format!("{name} wide source has no fixed element shape"))?;
        if source_lanes != result_lanes {
            return Err(format!(
                "{name} wide source/result lane mismatch: {source_lanes} vs {result_lanes}"
            ));
        }
        let lane_name = scalarized_convert_name(name);
        let mut out = Vec::new();
        let mut converted = Vec::with_capacity(result_lanes as usize);
        for lane in 0..result_lanes {
            let source = ctx.module.fresh_id();
            out.push(Instruction::new(
                Op::CompositeExtract,
                Some(source_element),
                Some(source),
                vec![Operand::IdRef(args[0]), Operand::LiteralBit32(lane)],
            ));
            ctx.phase_value_types
                .get_or_insert_with(Default::default)
                .insert(source, source_element);
            let result = ctx.module.fresh_id();
            out.extend(lower_convert_narrow(
                ctx,
                &lane_name,
                result,
                result_element,
                &[source],
            )?);
            converted.push(Operand::IdRef(result));
        }
        out.push(Instruction::new(
            Op::CompositeConstruct,
            Some(rty),
            Some(res),
            converted,
        ));
        return Ok(out);
    }
    lower_convert_narrow(ctx, name, res, rty, args)
}

fn lower_convert_narrow(
    ctx: &mut Ctx,
    name: &str,
    res: Word,
    rty: Word,
    args: &[Word],
) -> Result<Vec<Instruction>, String> {
    let parts: Vec<&str> = name.trim_start_matches("air.convert.").split('.').collect();
    let dst_type = parts.get(1).copied().unwrap_or("");
    let src_type = parts.last().copied().unwrap_or("");

    if is_i1_type_token(src_type) {
        let n = vector_len(ctx, rty);
        let elem_is_int = is_int_result(ctx, rty);
        let (one, zero) = if elem_is_int {
            (
                int_splat_or_scalar(ctx, rty, 1, n),
                int_splat_or_scalar(ctx, rty, 0, n),
            )
        } else {
            (
                splat_or_scalar(ctx, rty, 1.0, n),
                splat_or_scalar(ctx, rty, 0.0, n),
            )
        };
        return Ok(vec![Instruction::new(
            Op::Select,
            Some(rty),
            Some(res),
            vec![
                Operand::IdRef(args[0]),
                Operand::IdRef(one),
                Operand::IdRef(zero),
            ],
        )]);
    }
    if is_i1_type_token(dst_type) {
        let src_kind = parts
            .get(parts.len().saturating_sub(2))
            .and_then(|p| p.chars().next())
            .unwrap_or('f');
        let n = vector_len_of_value(ctx, args[0]);
        if src_kind == 'f' {
            let zero = splat_or_scalar(ctx, value_result_type(ctx, args[0]).unwrap_or(rty), 0.0, n);
            return Ok(vec![Instruction::new(
                Op::FUnordNotEqual,
                Some(rty),
                Some(res),
                vec![Operand::IdRef(args[0]), Operand::IdRef(zero)],
            )]);
        }
        let src_ty = value_result_type(ctx, args[0]).unwrap_or(rty);
        let zero = int_splat_or_scalar(ctx, src_ty, 0, n);
        return Ok(vec![Instruction::new(
            Op::INotEqual,
            Some(rty),
            Some(res),
            vec![Operand::IdRef(args[0]), Operand::IdRef(zero)],
        )]);
    }
    let kinds: Vec<char> = parts
        .iter()
        .filter(|p| p.len() == 1 && matches!(p.chars().next().unwrap(), 'f' | 's' | 'u'))
        .map(|p| p.chars().next().unwrap())
        .collect();
    let (dst, src) = match (kinds.first(), kinds.last()) {
        (Some(d), Some(s)) if kinds.len() >= 2 => (*d, *s),
        _ => return Err(format!("cannot parse convert kinds from {name}")),
    };
    if token_is_bf16(src_type) || token_is_bf16(dst_type) {
        return lower_convert_bf16(ctx, res, rty, args, dst_type, src_type, dst, src);
    }
    if let Some(token) = [src_type, dst_type]
        .into_iter()
        .find(|token| token_is_narrow_float(token))
    {
        return Err(format!(
            "{name} converts the 8-bit float format {}, which has no SPIR-V type and no modelled \
             bit layout here",
            scalarize_convert_token(token)
        ));
    }
    if (dst, src) == ('f', 's') {
        let mut out = Vec::new();
        let (signed, _) = bitcast_to_integer_signedness(ctx, &mut out, args[0], true)?;
        out.push(Instruction::new(
            Op::ConvertSToF,
            Some(rty),
            Some(res),
            vec![Operand::IdRef(signed)],
        ));
        return Ok(out);
    }
    if (dst, src) == ('u', 's') || (dst, src) == ('s', 'u') {
        let mut out = Vec::new();
        let signed_source = src == 's';
        let (input, input_ty) =
            bitcast_to_integer_signedness(ctx, &mut out, args[0], signed_source)?;
        let same_shape = scalar_bit_width(ctx, input_ty) == scalar_bit_width(ctx, rty)
            && vector_len(ctx, input_ty) == vector_len(ctx, rty);
        let instruction = if same_shape {
            copy_or_bitcast_result(rty, res, input_ty, input)
        } else {
            let opcode = if signed_source {
                Op::SConvert
            } else {
                Op::UConvert
            };
            Instruction::new(opcode, Some(rty), Some(res), vec![Operand::IdRef(input)])
        };
        out.push(instruction);
        return Ok(out);
    }
    let op = match (dst, src) {
        ('f', 'u') => Op::ConvertUToF,
        ('u', 'f') => Op::ConvertFToU,
        ('s', 'f') => Op::ConvertFToS,
        ('u', 'u') => Op::UConvert,
        ('s', 's') => Op::SConvert,
        ('f', 'f') => Op::FConvert,
        _ => return Err(format!("unhandled convert kinds {dst}->{src} in {name}")),
    };
    Ok(vec![Instruction::new(
        op,
        Some(rty),
        Some(res),
        vec![Operand::IdRef(args[0])],
    )])
}

fn bitcast_to_integer_signedness(
    ctx: &mut Ctx,
    out: &mut Vec<Instruction>,
    value: Word,
    signed: bool,
) -> Result<(Word, Word), String> {
    let ty = value_result_type(ctx, value).ok_or("air.convert integer source has no type")?;
    let target_ty = integer_type_like(ctx, ty, signed)?;
    if target_ty == ty {
        return Ok((value, ty));
    }
    let cast = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::Bitcast,
        Some(target_ty),
        Some(cast),
        vec![Operand::IdRef(value)],
    ));
    Ok((cast, target_ty))
}

fn integer_type_like(ctx: &mut Ctx, ty: Word, signed: bool) -> Result<Word, String> {
    let def = type_def_of(ctx, ty).ok_or("air.convert integer source type is undefined")?;
    match def.class.opcode {
        Op::TypeInt => {
            let bits = literal_u32(def.operands.first())
                .ok_or("air.convert integer source int missing width")?;
            let current_signed = literal_u32(def.operands.get(1))
                .ok_or("air.convert integer source int missing signedness")?;
            if current_signed == u32::from(signed) {
                Ok(ty)
            } else {
                Ok(integer_type(ctx, bits, signed))
            }
        }
        Op::TypeVector => {
            let elem = id_ref(def.operands.first())
                .ok_or("air.convert integer source vector missing element type")?;
            let lanes = literal_u32(def.operands.get(1))
                .ok_or("air.convert integer source vector missing length")?;
            let target_elem = integer_type_like(ctx, elem, signed)?;
            if target_elem == elem {
                Ok(ty)
            } else {
                Ok(vector_type(ctx, target_elem, lanes))
            }
        }
        _ => Err("air.convert source is not an integer scalar/vector".into()),
    }
}

fn integer_type(ctx: &mut Ctx, bits: u32, signed: bool) -> Word {
    let key = SynthCacheKey::IntType { bits, signed };
    if let Some(&id) = ctx.synth_cache.get(&key) {
        return id;
    }
    let signedness = u32::from(signed);
    for inst in ctx
        .module
        .types_global_values
        .iter()
        .chain(ctx.new_globals.iter())
    {
        if inst.class.opcode == Op::TypeInt
            && inst.operands.first() == Some(&Operand::LiteralBit32(bits))
            && inst.operands.get(1) == Some(&Operand::LiteralBit32(signedness))
        {
            if let Some(rid) = inst.result_id {
                ctx.synth_cache.insert(key, rid);
                return rid;
            }
        }
    }
    let id = ctx.module.fresh_id();
    ctx.new_globals.push(Instruction::new(
        Op::TypeInt,
        None,
        Some(id),
        vec![
            Operand::LiteralBit32(bits),
            Operand::LiteralBit32(signedness),
        ],
    ));
    ctx.synth_cache.insert(key, id);
    id
}

fn vector_type(ctx: &mut Ctx, elem: Word, lanes: u32) -> Word {
    let key = SynthCacheKey::VecType { elem, lanes };
    if let Some(&id) = ctx.synth_cache.get(&key) {
        return id;
    }
    for inst in ctx
        .module
        .types_global_values
        .iter()
        .chain(ctx.new_globals.iter())
    {
        if inst.class.opcode == Op::TypeVector
            && inst.operands.first() == Some(&Operand::IdRef(elem))
            && inst.operands.get(1) == Some(&Operand::LiteralBit32(lanes))
        {
            if let Some(rid) = inst.result_id {
                ctx.synth_cache.insert(key, rid);
                return rid;
            }
        }
    }
    let id = ctx.module.fresh_id();
    ctx.new_globals.push(Instruction::new(
        Op::TypeVector,
        None,
        Some(id),
        vec![Operand::IdRef(elem), Operand::LiteralBit32(lanes)],
    ));
    ctx.synth_cache.insert(key, id);
    id
}

fn id_ref(op: Option<&Operand>) -> Option<Word> {
    match op {
        Some(Operand::IdRef(id)) => Some(*id),
        _ => None,
    }
}

fn literal_u32(op: Option<&Operand>) -> Option<u32> {
    match op {
        Some(Operand::LiteralBit32(v)) => Some(*v),
        _ => None,
    }
}

fn scalarize_convert_token(tok: &str) -> &str {
    let Some(rest) = tok.strip_prefix('v') else {
        return tok;
    };
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        tok
    } else {
        &rest[digits..]
    }
}

fn scalarized_convert_name(name: &str) -> String {
    let tokens = name
        .trim_start_matches("air.convert.")
        .split('.')
        .map(scalarize_convert_token)
        .collect::<Vec<_>>();
    format!("air.convert.{}", tokens.join("."))
}

fn token_is_bf16(tok: &str) -> bool {
    tok.contains("bf16")
}

fn token_is_narrow_float(tok: &str) -> bool {
    tok.contains("f8e")
}

fn token_lanes(tok: &str) -> u32 {
    tok.strip_prefix('v')
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(1)
        })
        .unwrap_or(1)
}

fn largest_f32_below_pow2(k: u32) -> f32 {
    f32::from_bits(((k - 1 + 127) << 23) | 0x007f_ffff)
}

fn shaped_int_zero(ctx: &mut Ctx, int_ty: Word, n: u32) -> Word {
    let elem = element_type(ctx, int_ty);
    let scalar = ctx.const_int_of(elem, 0);
    if n > 1 {
        splat(ctx, int_ty, scalar, n)
    } else {
        scalar
    }
}

fn round_int_to_odd_f32(
    ctx: &mut Ctx,
    out: &mut Vec<Instruction>,
    int_val: Word,
    int_ty: Word,
    f32val: Word,
    signed: bool,
    n: u32,
) -> Word {
    let f32_ty = ty_f32_shaped(ctx, n);
    let u32_ty = ty_u32_shaped(ctx, n);
    let bool_ty = ty_bool_shaped(ctx, n);
    let width = scalar_bit_width(ctx, int_ty);

    let bound_pow2 = if signed { width - 1 } else { width };
    let bound = splat_or_scalar(ctx, f32_ty, largest_f32_below_pow2(bound_pow2), n);
    let ext = ctx.glsl();
    let clamped = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::ExtInst,
        Some(f32_ty),
        Some(clamped),
        vec![
            Operand::IdRef(ext),
            Operand::LiteralExtInstInteger(GLSLstd450::FMin as u32),
            Operand::IdRef(f32val),
            Operand::IdRef(bound),
        ],
    ));

    let back = ctx.module.fresh_id();
    out.push(Instruction::new(
        if signed {
            Op::ConvertFToS
        } else {
            Op::ConvertFToU
        },
        Some(int_ty),
        Some(back),
        vec![Operand::IdRef(clamped)],
    ));
    let exact = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::IEqual,
        Some(bool_ty),
        Some(exact),
        vec![Operand::IdRef(back), Operand::IdRef(int_val)],
    ));

    let away = if signed {
        let zero = shaped_int_zero(ctx, int_ty, n);
        let non_negative = ctx.module.fresh_id();
        out.push(Instruction::new(
            Op::SGreaterThanEqual,
            Some(bool_ty),
            Some(non_negative),
            vec![Operand::IdRef(int_val), Operand::IdRef(zero)],
        ));
        let greater = ctx.module.fresh_id();
        out.push(Instruction::new(
            Op::SGreaterThan,
            Some(bool_ty),
            Some(greater),
            vec![Operand::IdRef(back), Operand::IdRef(int_val)],
        ));
        let less = ctx.module.fresh_id();
        out.push(Instruction::new(
            Op::SLessThan,
            Some(bool_ty),
            Some(less),
            vec![Operand::IdRef(back), Operand::IdRef(int_val)],
        ));
        let away = ctx.module.fresh_id();
        out.push(Instruction::new(
            Op::Select,
            Some(bool_ty),
            Some(away),
            vec![
                Operand::IdRef(non_negative),
                Operand::IdRef(greater),
                Operand::IdRef(less),
            ],
        ));
        away
    } else {
        let away = ctx.module.fresh_id();
        out.push(Instruction::new(
            Op::UGreaterThan,
            Some(bool_ty),
            Some(away),
            vec![Operand::IdRef(back), Operand::IdRef(int_val)],
        ));
        away
    };

    let bits = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::Bitcast,
        Some(u32_ty),
        Some(bits),
        vec![Operand::IdRef(clamped)],
    ));
    let one = shaped_u32_const(ctx, n, 1);
    let stepped_down = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::ISub,
        Some(u32_ty),
        Some(stepped_down),
        vec![Operand::IdRef(bits), Operand::IdRef(one)],
    ));
    let toward_zero = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::Select,
        Some(u32_ty),
        Some(toward_zero),
        vec![
            Operand::IdRef(away),
            Operand::IdRef(stepped_down),
            Operand::IdRef(bits),
        ],
    ));
    let odd = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::BitwiseOr,
        Some(u32_ty),
        Some(odd),
        vec![Operand::IdRef(toward_zero), Operand::IdRef(one)],
    ));
    let selected = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::Select,
        Some(u32_ty),
        Some(selected),
        vec![
            Operand::IdRef(exact),
            Operand::IdRef(bits),
            Operand::IdRef(odd),
        ],
    ));
    let result = ctx.module.fresh_id();
    out.push(Instruction::new(
        Op::Bitcast,
        Some(f32_ty),
        Some(result),
        vec![Operand::IdRef(selected)],
    ));
    result
}

#[allow(clippy::too_many_arguments)]
fn lower_convert_bf16(
    ctx: &mut Ctx,
    res: Word,
    rty: Word,
    args: &[Word],
    dst_type: &str,
    src_type: &str,
    dst_kind: char,
    src_kind: char,
) -> Result<Vec<Instruction>, String> {
    let arg = *args
        .first()
        .ok_or("air.convert bf16: missing source operand")?;
    let src_is_bf16 = token_is_bf16(src_type);
    let dst_is_bf16 = token_is_bf16(dst_type);
    let n = if src_is_bf16 {
        token_lanes(src_type)
    } else {
        token_lanes(dst_type)
    };
    let mut out = Vec::new();

    let f32_ty = ty_f32_shaped(ctx, n);
    let f32val = if src_is_bf16 {
        widen_bf16_to_f32(ctx, &mut out, arg, n)
    } else {
        let src_ty = value_result_type(ctx, arg).unwrap_or(rty);
        match src_kind {
            'f' => {
                if scalar_bit_width(ctx, src_ty) == 32 {
                    arg
                } else {
                    let id = ctx.module.fresh_id();
                    out.push(Instruction::new(
                        Op::FConvert,
                        Some(f32_ty),
                        Some(id),
                        vec![Operand::IdRef(arg)],
                    ));
                    id
                }
            }
            _ => {
                let signed = src_kind == 's';
                let id = ctx.module.fresh_id();
                out.push(Instruction::new(
                    if signed {
                        Op::ConvertSToF
                    } else {
                        Op::ConvertUToF
                    },
                    Some(f32_ty),
                    Some(id),
                    vec![Operand::IdRef(arg)],
                ));
                if dst_is_bf16 && scalar_bit_width(ctx, src_ty) >= 32 {
                    round_int_to_odd_f32(ctx, &mut out, arg, src_ty, id, signed, n)
                } else {
                    id
                }
            }
        }
    };

    if dst_is_bf16 {
        narrow_f32_to_bf16(ctx, &mut out, f32val, n, rty, res);
    } else {
        let op = match dst_kind {
            's' => Op::ConvertFToS,
            'u' => Op::ConvertFToU,
            _ => {
                if scalar_bit_width(ctx, rty) == 32 {
                    Op::CopyObject
                } else {
                    Op::FConvert
                }
            }
        };
        out.push(Instruction::new(
            op,
            Some(rty),
            Some(res),
            vec![Operand::IdRef(f32val)],
        ));
    }
    Ok(out)
}
