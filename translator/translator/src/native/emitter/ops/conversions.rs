use super::*;

impl Emitter {
    pub(in crate::native::emitter) fn emit_i8_array_integer_bitcast(
        &mut self,
        src: Word,
        src_ty: &LlType,
        dst_ty: &LlType,
        result: Word,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        match (src_ty, dst_ty) {
            (LlType::Array(elem, lanes) | LlType::Vector(elem, lanes), LlType::Int(bits))
                if **elem == LlType::Int(8)
                    && *lanes > 0
                    && lanes.checked_mul(8) == Some(*bits)
                    && *bits <= 64
                    && (matches!(src_ty, LlType::Array(_, _)) || *lanes > 4) =>
            {
                let byte_ty = self.type_id(&LlType::Int(8))?;
                let integer_ty = self.type_id(dst_ty)?;
                let mut packed = None;
                for lane in 0..*lanes {
                    let byte = self.fresh();
                    instructions.push(Self::inst(
                        Op::CompositeExtract,
                        Some(byte_ty),
                        Some(byte),
                        vec![Operand::IdRef(src), Operand::LiteralBit32(lane)],
                    ));
                    let widened = self.fresh();
                    instructions.push(Self::inst(
                        Op::UConvert,
                        Some(integer_ty),
                        Some(widened),
                        vec![Operand::IdRef(byte)],
                    ));
                    let shifted = if lane == 0 {
                        widened
                    } else {
                        let shift = self.const_int(*bits, u64::from(lane) * 8)?;
                        let shifted = self.fresh();
                        instructions.push(Self::inst(
                            Op::ShiftLeftLogical,
                            Some(integer_ty),
                            Some(shifted),
                            vec![Operand::IdRef(widened), Operand::IdRef(shift)],
                        ));
                        shifted
                    };
                    packed = Some(match packed {
                        None => shifted,
                        Some(low) => {
                            let combined = if lane + 1 == *lanes {
                                result
                            } else {
                                self.fresh()
                            };
                            instructions.push(Self::inst(
                                Op::BitwiseOr,
                                Some(integer_ty),
                                Some(combined),
                                vec![Operand::IdRef(low), Operand::IdRef(shifted)],
                            ));
                            combined
                        }
                    });
                }
                if *lanes == 1 {
                    instructions.push(Self::inst(
                        Op::CopyObject,
                        Some(integer_ty),
                        Some(result),
                        vec![Operand::IdRef(packed.expect("non-empty byte array"))],
                    ));
                }
                Ok(true)
            }
            (LlType::Int(bits), LlType::Array(elem, lanes) | LlType::Vector(elem, lanes))
                if **elem == LlType::Int(8)
                    && *lanes > 0
                    && lanes.checked_mul(8) == Some(*bits)
                    && *bits <= 64
                    && (matches!(dst_ty, LlType::Array(_, _)) || *lanes > 4) =>
            {
                let integer_ty = self.type_id(src_ty)?;
                let byte_ty = self.type_id(&LlType::Int(8))?;
                let array_ty = self.type_id(dst_ty)?;
                let mut bytes = Vec::with_capacity(*lanes as usize);
                for lane in 0..*lanes {
                    let shifted = if lane == 0 {
                        src
                    } else {
                        let shift = self.const_int(*bits, u64::from(lane) * 8)?;
                        let shifted = self.fresh();
                        instructions.push(Self::inst(
                            Op::ShiftRightLogical,
                            Some(integer_ty),
                            Some(shifted),
                            vec![Operand::IdRef(src), Operand::IdRef(shift)],
                        ));
                        shifted
                    };
                    let byte = self.fresh();
                    instructions.push(Self::inst(
                        Op::UConvert,
                        Some(byte_ty),
                        Some(byte),
                        vec![Operand::IdRef(shifted)],
                    ));
                    bytes.push(Operand::IdRef(byte));
                }
                instructions.push(Self::inst(
                    Op::CompositeConstruct,
                    Some(array_ty),
                    Some(result),
                    bytes,
                ));
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(in crate::native::emitter) fn emit_int_convert_resolved(
        &mut self,
        op: Op,
        src: TypedValue,
        dst_ty: LlType,
        name: String,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let src_ty = self.resolve_type(&src.ty)?;
        if is_bool_type(&src_ty) && is_integer_type(&dst_ty) {
            let result_type = self.type_id(&dst_ty)?;
            let result = self.result_id(&name, &dst_ty)?;
            let src_id = self.value_id_in(&src.value, &src.ty, instructions)?;
            let true_value = if op == Op::SConvert { -1 } else { 1 };
            let one = self.int_constant_like(&dst_ty, true_value)?;
            let zero = self.int_constant_like(&dst_ty, 0)?;
            instructions.push(Self::inst(
                Op::Select,
                Some(result_type),
                Some(result),
                vec![
                    Operand::IdRef(src_id),
                    Operand::IdRef(one),
                    Operand::IdRef(zero),
                ],
            ));
            self.record_int_alignment(&name, &dst_ty, 1);
            return Ok(());
        }
        if is_integer_type(&src_ty) && is_bool_type(&dst_ty) {
            let result_type = self.type_id(&dst_ty)?;
            let result = self.result_id(&name, &dst_ty)?;
            let src_id = self.value_id_in(&src.value, &src.ty, instructions)?;
            let zero = self.int_constant_like(&src_ty, 0)?;
            instructions.push(Self::inst(
                Op::INotEqual,
                Some(result_type),
                Some(result),
                vec![Operand::IdRef(src_id), Operand::IdRef(zero)],
            ));
            return Ok(());
        }
        if !int_convert_supported(&src_ty, &dst_ty) {
            return Err(format!(
                "native emitter: integer conversion needs matching integer shapes, got {src_ty:?} to {dst_ty:?}"
            ));
        }
        let result_type = self.type_id(&dst_ty)?;
        let source_type = self.type_id(&src_ty)?;
        let result = self.result_id(&name, &dst_ty)?;
        let src_id = self.value_id_in(&src.value, &src.ty, instructions)?;
        let converted_src = if op == Op::SConvert {
            if let Some(bits) = nonstandard_scalar_int_bits(&src_ty) {
                let legal = spirv_int_width(bits)?;
                let shift = self.const_int(legal, u64::from(legal - bits))?;
                let shifted = self.fresh();
                instructions.push(Self::inst(
                    Op::ShiftLeftLogical,
                    Some(source_type),
                    Some(shifted),
                    vec![Operand::IdRef(src_id), Operand::IdRef(shift)],
                ));
                let extended = self.fresh();
                instructions.push(Self::inst(
                    Op::ShiftRightArithmetic,
                    Some(source_type),
                    Some(extended),
                    vec![Operand::IdRef(shifted), Operand::IdRef(shift)],
                ));
                extended
            } else {
                src_id
            }
        } else {
            src_id
        };
        let dst_resolved = self.resolve_type(&dst_ty)?;
        if let Some(bits) = nonstandard_scalar_int_bits(&dst_resolved) {
            let legal = spirv_int_width(bits)?;
            let mask = self.const_int(legal, (1u64 << bits) - 1)?;
            let masked_source = if source_type == result_type {
                converted_src
            } else {
                let converted = self.fresh();
                instructions.push(Self::inst(
                    op,
                    Some(result_type),
                    Some(converted),
                    vec![Operand::IdRef(converted_src)],
                ));
                converted
            };
            instructions.push(Self::inst(
                Op::BitwiseAnd,
                Some(result_type),
                Some(result),
                vec![Operand::IdRef(masked_source), Operand::IdRef(mask)],
            ));
            self.record_int_alignment(&name, &dst_ty, self.int_value_alignment(&src.value));
            return Ok(());
        }
        let emitted_op = if source_type == result_type {
            Op::CopyObject
        } else {
            op
        };
        instructions.push(Self::inst(
            emitted_op,
            Some(result_type),
            Some(result),
            vec![Operand::IdRef(converted_src)],
        ));
        self.record_int_alignment(&name, &dst_ty, self.int_value_alignment(&src.value));
        Ok(())
    }

    pub(in crate::native::emitter) fn emit_float_convert_resolved(
        &mut self,
        src: TypedValue,
        dst_ty: LlType,
        name: String,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let src_ty = self.resolve_type(&src.ty)?;
        if let (Some(src_lanes), Some(dst_lanes)) = (bfloat_lanes(&src_ty), float_lanes(&dst_ty)) {
            if src_lanes == dst_lanes {
                let src_id = self.value_id_in(&src.value, &src.ty, instructions)?;
                let result = self.result_id(&name, &dst_ty)?;
                return self.bfloat_bits_to_float_shaped(src_id, result, src_lanes, instructions);
            }
        }
        if let (Some(src_lanes), Some(dst_lanes)) = (float_lanes(&src_ty), bfloat_lanes(&dst_ty)) {
            if src_lanes == dst_lanes {
                let src_id = self.value_id_in(&src.value, &src.ty, instructions)?;
                let result = self.result_id(&name, &dst_ty)?;
                return self.emit_float_to_bfloat_bits_shaped(
                    src_id,
                    result,
                    src_lanes,
                    instructions,
                );
            }
        }
        if !float_convert_supported(&src_ty, &dst_ty) {
            return Err(format!(
                "native emitter: float conversion needs matching float/half shapes, got {src_ty:?} to {dst_ty:?}"
            ));
        }
        let result_type = self.type_id(&dst_ty)?;
        let source_type = self.type_id(&src_ty)?;
        let result = self.result_id(&name, &dst_ty)?;
        let src_id = self.value_id_in(&src.value, &src.ty, instructions)?;
        instructions.push(Self::inst(
            if source_type == result_type {
                Op::CopyObject
            } else {
                Op::FConvert
            },
            Some(result_type),
            Some(result),
            vec![Operand::IdRef(src_id)],
        ));
        Ok(())
    }

    pub(in crate::native::emitter) fn emit_int_to_float_convert_resolved(
        &mut self,
        op: Op,
        src: TypedValue,
        dst_ty: LlType,
        name: String,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let src_ty = self.resolve_type(&src.ty)?;
        if is_bool_type(&src_ty) {
            let result_type = self.type_id(&dst_ty)?;
            let result = self.result_id(&name, &dst_ty)?;
            let src_id = self.value_id_in(&src.value, &src.ty, instructions)?;
            let one = self.float_constant_like_fptoint(
                &dst_ty,
                if op == Op::ConvertSToF { -1.0 } else { 1.0 },
            )?;
            let zero = self.float_constant_like_fptoint(&dst_ty, 0.0)?;
            instructions.push(Self::inst(
                Op::Select,
                Some(result_type),
                Some(result),
                vec![
                    Operand::IdRef(src_id),
                    Operand::IdRef(one),
                    Operand::IdRef(zero),
                ],
            ));
            return Ok(());
        }
        if !int_to_float_convert_supported(&src_ty, &dst_ty) {
            return Err(format!(
                "native emitter: integer-to-float conversion needs matching integer-to-float shapes, got {src_ty:?} to {dst_ty:?}"
            ));
        }
        let result_type = self.type_id(&dst_ty)?;
        let result = self.result_id(&name, &dst_ty)?;
        let src_id = self.value_id_in(&src.value, &src.ty, instructions)?;
        instructions.push(Self::inst(
            op,
            Some(result_type),
            Some(result),
            vec![Operand::IdRef(src_id)],
        ));
        Ok(())
    }

    fn float_constant_like_fptoint(&mut self, ty: &LlType, value: f32) -> Result<Word, String> {
        match ty {
            LlType::Float => self.const_float32(value),
            LlType::Half => self.const_float16_bits(if value == 1.0 {
                0x3C00
            } else if value == -1.0 {
                0xBC00
            } else {
                0
            }),
            LlType::Vector(elem, lanes) => {
                let scalar = self.float_constant_like_fptoint(elem, value)?;
                let result_type = self.type_id(ty)?;
                let result = self.fresh();
                self.module.types_global_values.push(Self::inst(
                    Op::ConstantComposite,
                    Some(result_type),
                    Some(result),
                    (0..*lanes).map(|_| Operand::IdRef(scalar)).collect(),
                ));
                Ok(result)
            }
            other => Err(format!(
                "native emitter: float constant requested for non-float type {other:?}"
            )),
        }
    }

    pub(in crate::native::emitter) fn emit_float_to_int_convert_resolved(
        &mut self,
        op: Op,
        src: TypedValue,
        dst_ty: LlType,
        name: String,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let src_ty = self.resolve_type(&src.ty)?;
        if !int_to_float_convert_supported(&dst_ty, &src_ty) {
            return Err(format!(
                "native emitter: float-to-integer conversion needs matching float-to-integer shapes, got {src_ty:?} to {dst_ty:?}"
            ));
        }
        let result_type = self.type_id(&dst_ty)?;
        let result = self.result_id(&name, &dst_ty)?;
        let src_id = self.value_id_in(&src.value, &src.ty, instructions)?;
        instructions.push(Self::inst(
            op,
            Some(result_type),
            Some(result),
            vec![Operand::IdRef(src_id)],
        ));
        Ok(())
    }

    pub(in crate::native::emitter) fn bfloat_bits_to_float_shaped_id(
        &mut self,
        src_id: Word,
        n: u32,
        instructions: &mut Vec<Instruction>,
    ) -> Result<Word, String> {
        let result = self.fresh();
        self.bfloat_bits_to_float_shaped(src_id, result, n, instructions)?;
        Ok(result)
    }

    pub(in crate::native::emitter) fn bfloat_bits_to_float_shaped(
        &mut self,
        src_id: Word,
        result: Word,
        n: u32,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let i32_ty = self.type_id(&shaped_type(LlType::Int(32), n))?;
        let float_ty = self.type_id(&shaped_type(LlType::Float, n))?;
        let widened = self.fresh();
        instructions.push(Self::inst(
            Op::UConvert,
            Some(i32_ty),
            Some(widened),
            vec![Operand::IdRef(src_id)],
        ));
        let shift = self.const_uint_shaped(16, n)?;
        let shifted = self.fresh();
        instructions.push(Self::inst(
            Op::ShiftLeftLogical,
            Some(i32_ty),
            Some(shifted),
            vec![Operand::IdRef(widened), Operand::IdRef(shift)],
        ));
        instructions.push(Self::inst(
            Op::Bitcast,
            Some(float_ty),
            Some(result),
            vec![Operand::IdRef(shifted)],
        ));
        Ok(())
    }

    pub(in crate::native::emitter) fn emit_float_to_bfloat_bits_shaped(
        &mut self,
        src_id: Word,
        result: Word,
        n: u32,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let i32_ty = self.type_id(&shaped_type(LlType::Int(32), n))?;
        let result_type = self.type_id(&shaped_type(LlType::BFloat, n))?;
        let bits = self.fresh();
        instructions.push(Self::inst(
            Op::Bitcast,
            Some(i32_ty),
            Some(bits),
            vec![Operand::IdRef(src_id)],
        ));
        let shift = self.const_uint_shaped(16, n)?;
        let high = self.fresh();
        instructions.push(Self::inst(
            Op::ShiftRightLogical,
            Some(i32_ty),
            Some(high),
            vec![Operand::IdRef(bits), Operand::IdRef(shift)],
        ));
        let one = self.const_uint_shaped(1, n)?;
        let lsb = self.fresh();
        instructions.push(Self::inst(
            Op::BitwiseAnd,
            Some(i32_ty),
            Some(lsb),
            vec![Operand::IdRef(high), Operand::IdRef(one)],
        ));
        let bias_base = self.const_uint_shaped(0x7fff, n)?;
        let bias = self.fresh();
        instructions.push(Self::inst(
            Op::IAdd,
            Some(i32_ty),
            Some(bias),
            vec![Operand::IdRef(bias_base), Operand::IdRef(lsb)],
        ));
        let rounded = self.fresh();
        instructions.push(Self::inst(
            Op::IAdd,
            Some(i32_ty),
            Some(rounded),
            vec![Operand::IdRef(bits), Operand::IdRef(bias)],
        ));
        let shifted = self.fresh();
        instructions.push(Self::inst(
            Op::ShiftRightLogical,
            Some(i32_ty),
            Some(shifted),
            vec![Operand::IdRef(rounded), Operand::IdRef(shift)],
        ));
        let narrowed = self.select_canonical_bfloat_nan_bits(bits, shifted, n, instructions)?;
        instructions.push(Self::inst(
            Op::UConvert,
            Some(result_type),
            Some(result),
            vec![Operand::IdRef(narrowed)],
        ));
        Ok(())
    }

    fn select_canonical_bfloat_nan_bits(
        &mut self,
        bits: Word,
        narrowed: Word,
        n: u32,
        instructions: &mut Vec<Instruction>,
    ) -> Result<Word, String> {
        let i32_ty = self.type_id(&shaped_type(LlType::Int(32), n))?;
        let bool_ty = self.type_id(&shaped_type(LlType::Bool, n))?;
        let exp_mask = self.const_uint_shaped(0x7f80_0000, n)?;
        let mant_mask = self.const_uint_shaped(0x007f_ffff, n)?;
        let zero = self.const_uint_shaped(0, n)?;

        let exp_bits = self.fresh();
        instructions.push(Self::inst(
            Op::BitwiseAnd,
            Some(i32_ty),
            Some(exp_bits),
            vec![Operand::IdRef(bits), Operand::IdRef(exp_mask)],
        ));
        let exp_all_ones = self.fresh();
        instructions.push(Self::inst(
            Op::IEqual,
            Some(bool_ty),
            Some(exp_all_ones),
            vec![Operand::IdRef(exp_bits), Operand::IdRef(exp_mask)],
        ));

        let mant_bits = self.fresh();
        instructions.push(Self::inst(
            Op::BitwiseAnd,
            Some(i32_ty),
            Some(mant_bits),
            vec![Operand::IdRef(bits), Operand::IdRef(mant_mask)],
        ));
        let mant_nonzero = self.fresh();
        instructions.push(Self::inst(
            Op::INotEqual,
            Some(bool_ty),
            Some(mant_nonzero),
            vec![Operand::IdRef(mant_bits), Operand::IdRef(zero)],
        ));

        let is_nan = self.fresh();
        instructions.push(Self::inst(
            Op::LogicalAnd,
            Some(bool_ty),
            Some(is_nan),
            vec![Operand::IdRef(exp_all_ones), Operand::IdRef(mant_nonzero)],
        ));

        let canonical_nan = self.const_uint_shaped(0x7fc0, n)?;
        let selected = self.fresh();
        instructions.push(Self::inst(
            Op::Select,
            Some(i32_ty),
            Some(selected),
            vec![
                Operand::IdRef(is_nan),
                Operand::IdRef(canonical_nan),
                Operand::IdRef(narrowed),
            ],
        ));
        Ok(selected)
    }

    pub(in crate::native::emitter) fn emit_i32_to_v4i8(
        &mut self,
        src: Word,
        instructions: &mut Vec<Instruction>,
    ) -> Result<Word, String> {
        let i32_ty = self.type_id(&LlType::Int(32))?;
        let i8_ty = self.type_id(&LlType::Int(8))?;
        let v4i8_ty = self.type_id(&LlType::Vector(Box::new(LlType::Int(8)), 4))?;
        let mask = self.const_uint(0xff)?;
        let mut lanes = Vec::with_capacity(4);
        for lane in 0..4u32 {
            let shifted = if lane == 0 {
                src
            } else {
                let shift = self.const_uint(lane * 8)?;
                let id = self.fresh();
                instructions.push(Self::inst(
                    Op::ShiftRightLogical,
                    Some(i32_ty),
                    Some(id),
                    vec![Operand::IdRef(src), Operand::IdRef(shift)],
                ));
                id
            };
            let masked = self.fresh();
            instructions.push(Self::inst(
                Op::BitwiseAnd,
                Some(i32_ty),
                Some(masked),
                vec![Operand::IdRef(shifted), Operand::IdRef(mask)],
            ));
            let narrow = self.fresh();
            instructions.push(Self::inst(
                Op::UConvert,
                Some(i8_ty),
                Some(narrow),
                vec![Operand::IdRef(masked)],
            ));
            lanes.push(narrow);
        }
        let result = self.fresh();
        instructions.push(Self::inst(
            Op::CompositeConstruct,
            Some(v4i8_ty),
            Some(result),
            lanes.into_iter().map(Operand::IdRef).collect(),
        ));
        Ok(result)
    }
}
