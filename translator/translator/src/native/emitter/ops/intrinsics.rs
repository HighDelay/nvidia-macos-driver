use super::*;

impl Emitter {
    pub(in crate::native::emitter) fn emit_llvm_bitreverse_call(
        &mut self,
        call: &LlCall,
        name: &str,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        if !call.callee.starts_with("llvm.bitreverse.") {
            return Ok(false);
        }
        let [value] = call.args.as_slice() else {
            return Err(format!(
                "native emitter: {} expects one operand",
                call.callee
            ));
        };
        let result_ty = self.resolve_type(&call.ret)?;
        let value_ty = self.resolve_type(&value.ty)?;
        let supported = match &result_ty {
            LlType::Int(32) => true,
            LlType::Vector(element, _) => **element == LlType::Int(32),
            _ => false,
        };
        if !supported || value_ty != result_ty {
            return Err(format!(
                "native emitter: {} requires matching 32-bit integer scalar/vector operands, got {value_ty:?}, {result_ty:?}",
                call.callee
            ));
        }
        let result_type = self.type_id(&result_ty)?;
        let value = self.value_id_in(&value.value, &value.ty, instructions)?;
        let result = self.result_id(name, &result_ty)?;
        instructions.push(Self::inst(
            Op::BitReverse,
            Some(result_type),
            Some(result),
            vec![Operand::IdRef(value)],
        ));
        self.record_int_alignment(name, &result_ty, 1);
        Ok(true)
    }

    pub(in crate::native::emitter) fn emit_llvm_ctpop_call(
        &mut self,
        call: &LlCall,
        name: &str,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        if !call.callee.starts_with("llvm.ctpop.") {
            return Ok(false);
        }
        let [value] = call.args.as_slice() else {
            return Err(format!(
                "native emitter: {} expects one operand",
                call.callee
            ));
        };
        let result_ty = self.resolve_type(&call.ret)?;
        let value_ty = self.resolve_type(&value.ty)?;
        if !is_integer_type(&result_ty) || value_ty != result_ty {
            return Err(format!(
                "native emitter: {} operand/result type mismatch {value_ty:?}, {result_ty:?}",
                call.callee
            ));
        }

        let result_type = self.type_id(&result_ty)?;
        let value = self.value_id_in(&value.value, &value.ty, instructions)?;
        let result = self.result_id(name, &result_ty)?;
        instructions.push(Self::inst(
            Op::BitCount,
            Some(result_type),
            Some(result),
            vec![Operand::IdRef(value)],
        ));
        self.record_int_alignment(name, &result_ty, 1);
        Ok(true)
    }

    pub(in crate::native::emitter) fn emit_llvm_int_minmax_call(
        &mut self,
        call: &LlCall,
        name: &str,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        let cmp_op = if call.callee.starts_with("llvm.umax.") {
            Op::UGreaterThan
        } else if call.callee.starts_with("llvm.umin.") {
            Op::ULessThan
        } else if call.callee.starts_with("llvm.smax.") {
            Op::SGreaterThan
        } else if call.callee.starts_with("llvm.smin.") {
            Op::SLessThan
        } else {
            return Ok(false);
        };
        if call.args.len() != 2 {
            return Err(format!(
                "native emitter: {} expects two operands",
                call.callee
            ));
        }
        let result_ty = self.resolve_type(&call.ret)?;
        if !is_integer_type(&result_ty) {
            return Err(format!(
                "native emitter: {} expects integer result, got {result_ty:?}",
                call.callee
            ));
        }
        let lhs = &call.args[0];
        let rhs = &call.args[1];
        let lhs_ty = self.resolve_type(&lhs.ty)?;
        let rhs_ty = self.resolve_type(&rhs.ty)?;
        if lhs_ty != result_ty || rhs_ty != result_ty {
            return Err(format!(
                "native emitter: {} operand/result type mismatch {lhs_ty:?}, {rhs_ty:?}, {result_ty:?}",
                call.callee
            ));
        }
        let cmp_ty = int_compare_result_type(&result_ty)?;
        let cmp_ty_id = self.type_id(&cmp_ty)?;
        let result_ty_id = self.type_id(&result_ty)?;
        let lhs_id = self.value_id_in(&lhs.value, &lhs.ty, instructions)?;
        let rhs_id = self.value_id_in(&rhs.value, &rhs.ty, instructions)?;
        let cmp = self.fresh();
        instructions.push(Self::inst(
            cmp_op,
            Some(cmp_ty_id),
            Some(cmp),
            vec![Operand::IdRef(lhs_id), Operand::IdRef(rhs_id)],
        ));
        let result = self.result_id(name, &result_ty)?;
        instructions.push(Self::inst(
            Op::Select,
            Some(result_ty_id),
            Some(result),
            vec![
                Operand::IdRef(cmp),
                Operand::IdRef(lhs_id),
                Operand::IdRef(rhs_id),
            ],
        ));
        self.record_int_alignment(name, &result_ty, 1);
        Ok(true)
    }

    pub(in crate::native::emitter) fn emit_llvm_abs_call(
        &mut self,
        call: &LlCall,
        name: &str,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        if !call.callee.starts_with("llvm.abs.") {
            return Ok(false);
        }
        if call.args.len() != 2 {
            return Err(format!(
                "native emitter: {} expects two operands",
                call.callee
            ));
        }
        let result_ty = self.resolve_type(&call.ret)?;
        if !is_integer_type(&result_ty) {
            return Err(format!(
                "native emitter: {} expects integer result, got {result_ty:?}",
                call.callee
            ));
        }
        let value_ty = self.resolve_type(&call.args[0].ty)?;
        if value_ty != result_ty {
            return Err(format!(
                "native emitter: {} operand/result type mismatch {value_ty:?}, {result_ty:?}",
                call.callee
            ));
        }
        let poison_ty = self.resolve_type(&call.args[1].ty)?;
        if poison_ty != LlType::Bool {
            return Err(format!(
                "native emitter: {} poison flag is {poison_ty:?}",
                call.callee
            ));
        }

        let result_type = self.type_id(&result_ty)?;
        let value = self.value_id_in(&call.args[0].value, &call.args[0].ty, instructions)?;
        let result = self.result_id(name, &result_ty)?;
        let glsl = self.glsl_ext_inst_import();
        instructions.push(Self::inst(
            Op::ExtInst,
            Some(result_type),
            Some(result),
            vec![
                Operand::IdRef(glsl),
                Operand::LiteralExtInstInteger(GlslStd450Op::SAbs as u32),
                Operand::IdRef(value),
            ],
        ));
        self.record_int_alignment(name, &result_ty, 1);
        Ok(true)
    }

    pub(in crate::native::emitter) fn emit_llvm_usub_sat_call(
        &mut self,
        call: &LlCall,
        name: &str,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        if !call.callee.starts_with("llvm.usub.sat.") {
            return Ok(false);
        }
        if call.args.len() != 2 {
            return Err(format!(
                "native emitter: {} expects two operands",
                call.callee
            ));
        }
        let result_ty = self.resolve_type(&call.ret)?;
        if !is_integer_type(&result_ty) {
            return Err(format!(
                "native emitter: {} expects integer result, got {result_ty:?}",
                call.callee
            ));
        }
        let lhs = &call.args[0];
        let rhs = &call.args[1];
        let lhs_ty = self.resolve_type(&lhs.ty)?;
        let rhs_ty = self.resolve_type(&rhs.ty)?;
        if lhs_ty != result_ty || rhs_ty != result_ty {
            return Err(format!(
                "native emitter: {} operand/result type mismatch {lhs_ty:?}, {rhs_ty:?}, {result_ty:?}",
                call.callee
            ));
        }

        let cmp_ty = int_compare_result_type(&result_ty)?;
        let cmp_ty_id = self.type_id(&cmp_ty)?;
        let result_ty_id = self.type_id(&result_ty)?;
        let lhs_id = self.value_id_in(&lhs.value, &lhs.ty, instructions)?;
        let rhs_id = self.value_id_in(&rhs.value, &rhs.ty, instructions)?;
        let underflow = self.fresh();
        instructions.push(Self::inst(
            Op::ULessThan,
            Some(cmp_ty_id),
            Some(underflow),
            vec![Operand::IdRef(lhs_id), Operand::IdRef(rhs_id)],
        ));
        let sub = self.fresh();
        instructions.push(Self::inst(
            Op::ISub,
            Some(result_ty_id),
            Some(sub),
            vec![Operand::IdRef(lhs_id), Operand::IdRef(rhs_id)],
        ));
        let zero = self.const_null(&result_ty)?;
        let result = self.result_id(name, &result_ty)?;
        instructions.push(Self::inst(
            Op::Select,
            Some(result_ty_id),
            Some(result),
            vec![
                Operand::IdRef(underflow),
                Operand::IdRef(zero),
                Operand::IdRef(sub),
            ],
        ));
        self.record_int_alignment(name, &result_ty, 1);
        Ok(true)
    }

    pub(in crate::native::emitter) fn emit_air_saturating_add_sub_call(
        &mut self,
        call: &LlCall,
        name: &str,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        const MEMBERS: [(&str, bool, bool); 4] = [
            ("air.add_sat.u.", true, false),
            ("air.add_sat.s.", true, true),
            ("air.sub_sat.u.", false, false),
            ("air.sub_sat.s.", false, true),
        ];
        let Some(&(_, is_add, signed)) = MEMBERS
            .iter()
            .find(|(prefix, _, _)| call.callee.starts_with(prefix))
        else {
            return Ok(false);
        };
        let (result_ty, width, lhs_id, rhs_id) = self.int_binary_operands(call, instructions, 2)?;
        let cmp_ty_id = self.type_id(&int_compare_result_type(&result_ty)?)?;
        let result_ty_id = self.type_id(&result_ty)?;

        let raw = self.fresh();
        instructions.push(Self::inst(
            if is_add { Op::IAdd } else { Op::ISub },
            Some(result_ty_id),
            Some(raw),
            vec![Operand::IdRef(lhs_id), Operand::IdRef(rhs_id)],
        ));

        let (overflowed, limit) = if signed {
            let (left, right) = if is_add {
                ((lhs_id, raw), (rhs_id, raw))
            } else {
                ((lhs_id, rhs_id), (lhs_id, raw))
            };
            let disagree_left = self.fresh();
            instructions.push(Self::inst(
                Op::BitwiseXor,
                Some(result_ty_id),
                Some(disagree_left),
                vec![Operand::IdRef(left.0), Operand::IdRef(left.1)],
            ));
            let disagree_right = self.fresh();
            instructions.push(Self::inst(
                Op::BitwiseXor,
                Some(result_ty_id),
                Some(disagree_right),
                vec![Operand::IdRef(right.0), Operand::IdRef(right.1)],
            ));
            let both = self.fresh();
            instructions.push(Self::inst(
                Op::BitwiseAnd,
                Some(result_ty_id),
                Some(both),
                vec![
                    Operand::IdRef(disagree_left),
                    Operand::IdRef(disagree_right),
                ],
            ));
            let zero = self.const_null(&result_ty)?;
            let overflowed = self.fresh();
            instructions.push(Self::inst(
                Op::SLessThan,
                Some(cmp_ty_id),
                Some(overflowed),
                vec![Operand::IdRef(both), Operand::IdRef(zero)],
            ));
            let sign_bits = self.fresh();
            let shift = self.int_constant_like(&result_ty, i64::from(width - 1))?;
            instructions.push(Self::inst(
                Op::ShiftRightArithmetic,
                Some(result_ty_id),
                Some(sign_bits),
                vec![Operand::IdRef(lhs_id), Operand::IdRef(shift)],
            ));
            let int_max = self.int_constant_like(&result_ty, i64::MAX >> (64 - width))?;
            let limit = self.fresh();
            instructions.push(Self::inst(
                Op::BitwiseXor,
                Some(result_ty_id),
                Some(limit),
                vec![Operand::IdRef(sign_bits), Operand::IdRef(int_max)],
            ));
            (overflowed, limit)
        } else {
            let overflowed = self.fresh();
            instructions.push(Self::inst(
                Op::ULessThan,
                Some(cmp_ty_id),
                Some(overflowed),
                if is_add {
                    vec![Operand::IdRef(raw), Operand::IdRef(lhs_id)]
                } else {
                    vec![Operand::IdRef(lhs_id), Operand::IdRef(rhs_id)]
                },
            ));
            let limit = if is_add {
                self.int_constant_like(&result_ty, -1)?
            } else {
                self.const_null(&result_ty)?
            };
            (overflowed, limit)
        };

        let result = self.result_id(name, &result_ty)?;
        instructions.push(Self::inst(
            Op::Select,
            Some(result_ty_id),
            Some(result),
            vec![
                Operand::IdRef(overflowed),
                Operand::IdRef(limit),
                Operand::IdRef(raw),
            ],
        ));
        self.record_int_alignment(name, &result_ty, 1);
        Ok(true)
    }

    pub(in crate::native::emitter) fn emit_air_halving_add_call(
        &mut self,
        call: &LlCall,
        name: &str,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        const MEMBERS: [(&str, bool, bool); 4] = [
            ("air.hadd.u.", false, false),
            ("air.hadd.s.", false, true),
            ("air.rhadd.u.", true, false),
            ("air.rhadd.s.", true, true),
        ];
        let Some(&(_, rounded, signed)) = MEMBERS
            .iter()
            .find(|(prefix, _, _)| call.callee.starts_with(prefix))
        else {
            return Ok(false);
        };
        let (result_ty, _, lhs_id, rhs_id) = self.int_binary_operands(call, instructions, 2)?;
        let result_ty_id = self.type_id(&result_ty)?;

        let common = self.fresh();
        instructions.push(Self::inst(
            if rounded {
                Op::BitwiseOr
            } else {
                Op::BitwiseAnd
            },
            Some(result_ty_id),
            Some(common),
            vec![Operand::IdRef(lhs_id), Operand::IdRef(rhs_id)],
        ));
        let differing = self.fresh();
        instructions.push(Self::inst(
            Op::BitwiseXor,
            Some(result_ty_id),
            Some(differing),
            vec![Operand::IdRef(lhs_id), Operand::IdRef(rhs_id)],
        ));
        let half = self.fresh();
        let one = self.int_constant_like(&result_ty, 1)?;
        instructions.push(Self::inst(
            if signed {
                Op::ShiftRightArithmetic
            } else {
                Op::ShiftRightLogical
            },
            Some(result_ty_id),
            Some(half),
            vec![Operand::IdRef(differing), Operand::IdRef(one)],
        ));
        let result = self.result_id(name, &result_ty)?;
        instructions.push(Self::inst(
            if rounded { Op::ISub } else { Op::IAdd },
            Some(result_ty_id),
            Some(result),
            vec![Operand::IdRef(common), Operand::IdRef(half)],
        ));
        self.record_int_alignment(name, &result_ty, 1);
        Ok(true)
    }

    fn int_binary_operands(
        &mut self,
        call: &LlCall,
        instructions: &mut Vec<Instruction>,
        arity: usize,
    ) -> Result<(LlType, u32, Word, Word), String> {
        if call.args.len() != arity {
            return Err(format!(
                "native emitter: {} expects {arity} operands",
                call.callee
            ));
        }
        let result_ty = self.resolve_type(&call.ret)?;
        let width = match &result_ty {
            LlType::Int(bits) => *bits,
            LlType::Vector(elem, _) => match elem.as_ref() {
                LlType::Int(bits) => *bits,
                _ => 0,
            },
            _ => 0,
        };
        if width == 0 {
            return Err(format!(
                "native emitter: {} expects integer result, got {result_ty:?}",
                call.callee
            ));
        }
        for arg in &call.args {
            let arg_ty = self.resolve_type(&arg.ty)?;
            if arg_ty != result_ty {
                return Err(format!(
                    "native emitter: {} operand/result type mismatch {arg_ty:?}, {result_ty:?}",
                    call.callee
                ));
            }
        }
        let lhs = self.value_id_in(&call.args[0].value, &call.args[0].ty, instructions)?;
        let rhs = self.value_id_in(&call.args[1].value, &call.args[1].ty, instructions)?;
        Ok((result_ty, width, lhs, rhs))
    }

    pub(in crate::native::emitter) fn emit_llvm_fshl_call(
        &mut self,
        call: &LlCall,
        name: &str,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        let Some(bits) = call
            .callee
            .strip_prefix("llvm.fshl.i")
            .and_then(|suffix| suffix.parse::<u32>().ok())
            .filter(|bits| matches!(bits, 8 | 16 | 32 | 64))
        else {
            return Ok(false);
        };
        if call.args.len() != 3 {
            return Err(format!(
                "native emitter: llvm.fshl.i{bits} expects three operands"
            ));
        }
        let result_ty = self.resolve_type(&call.ret)?;
        let int_ty = LlType::Int(bits);
        if result_ty != int_ty {
            return Err(format!(
                "native emitter: llvm.fshl.i{bits} returned {result_ty:?}"
            ));
        }
        for arg in &call.args {
            let arg_ty = self.resolve_type(&arg.ty)?;
            if arg_ty != int_ty {
                return Err(format!(
                    "native emitter: llvm.fshl.i{bits} operand is {arg_ty:?}"
                ));
            }
        }

        let uint = self.type_id(&int_ty)?;
        let bool_ty = self.type_id(&LlType::Bool)?;
        let result = self.result_id(name, &result_ty)?;
        let lhs = self.value_id_in(&call.args[0].value, &call.args[0].ty, instructions)?;
        let rhs = self.value_id_in(&call.args[1].value, &call.args[1].ty, instructions)?;
        let shift = self.value_id_in(&call.args[2].value, &call.args[2].ty, instructions)?;
        let mask = self.const_int(bits, u64::from(bits - 1))?;
        let normalized_shift = self.fresh();
        instructions.push(Self::inst(
            Op::BitwiseAnd,
            Some(uint),
            Some(normalized_shift),
            vec![Operand::IdRef(shift), Operand::IdRef(mask)],
        ));

        let left = self.fresh();
        instructions.push(Self::inst(
            Op::ShiftLeftLogical,
            Some(uint),
            Some(left),
            vec![Operand::IdRef(lhs), Operand::IdRef(normalized_shift)],
        ));

        let width = self.const_int(bits, u64::from(bits))?;
        let inverse_unmasked = self.fresh();
        instructions.push(Self::inst(
            Op::ISub,
            Some(uint),
            Some(inverse_unmasked),
            vec![Operand::IdRef(width), Operand::IdRef(normalized_shift)],
        ));
        let inverse_shift = self.fresh();
        instructions.push(Self::inst(
            Op::BitwiseAnd,
            Some(uint),
            Some(inverse_shift),
            vec![Operand::IdRef(inverse_unmasked), Operand::IdRef(mask)],
        ));

        let right_raw = self.fresh();
        instructions.push(Self::inst(
            Op::ShiftRightLogical,
            Some(uint),
            Some(right_raw),
            vec![Operand::IdRef(rhs), Operand::IdRef(inverse_shift)],
        ));
        let zero = self.const_int(bits, 0)?;
        let is_zero_shift = self.fresh();
        instructions.push(Self::inst(
            Op::IEqual,
            Some(bool_ty),
            Some(is_zero_shift),
            vec![Operand::IdRef(normalized_shift), Operand::IdRef(zero)],
        ));
        let right = self.fresh();
        instructions.push(Self::inst(
            Op::Select,
            Some(uint),
            Some(right),
            vec![
                Operand::IdRef(is_zero_shift),
                Operand::IdRef(zero),
                Operand::IdRef(right_raw),
            ],
        ));

        instructions.push(Self::inst(
            Op::BitwiseOr,
            Some(uint),
            Some(result),
            vec![Operand::IdRef(left), Operand::IdRef(right)],
        ));
        self.record_int_alignment(name, &result_ty, 1);
        Ok(true)
    }

    pub(in crate::native::emitter) fn emit_llvm_cttz_i32_call(
        &mut self,
        call: &LlCall,
        name: &str,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        if call.callee != "llvm.cttz.i32" {
            return Ok(false);
        }
        if call.args.len() != 2 {
            return Err("native emitter: llvm.cttz.i32 expects two operands".to_string());
        }
        let result_ty = self.resolve_type(&call.ret)?;
        if result_ty != LlType::Int(32) {
            return Err(format!(
                "native emitter: llvm.cttz.i32 returned {result_ty:?}"
            ));
        }
        let value_ty = self.resolve_type(&call.args[0].ty)?;
        if value_ty != LlType::Int(32) {
            return Err(format!(
                "native emitter: llvm.cttz.i32 value operand is {value_ty:?}"
            ));
        }
        let zero_undef_ty = self.resolve_type(&call.args[1].ty)?;
        if zero_undef_ty != LlType::Bool {
            return Err(format!(
                "native emitter: llvm.cttz.i32 zero-undef operand is {zero_undef_ty:?}"
            ));
        }
        let zero_is_undef = matches!(call.args[1].value, LlValue::Bool(true));

        let result_type = self.type_id(&result_ty)?;
        let value = self.value_id_in(&call.args[0].value, &call.args[0].ty, instructions)?;
        let lsb = if zero_is_undef {
            self.result_id(name, &result_ty)?
        } else {
            self.fresh()
        };
        let glsl = self.glsl_ext_inst_import();
        instructions.push(Self::inst(
            Op::ExtInst,
            Some(result_type),
            Some(lsb),
            vec![
                Operand::IdRef(glsl),
                Operand::LiteralExtInstInteger(GlslStd450Op::FindILsb as u32),
                Operand::IdRef(value),
            ],
        ));
        if !zero_is_undef {
            let zero = self.const_null(&result_ty)?;
            let bit_width = self.const_uint(32)?;
            let cmp_ty = self.type_id(&LlType::Bool)?;
            let is_zero = self.fresh();
            instructions.push(Self::inst(
                Op::IEqual,
                Some(cmp_ty),
                Some(is_zero),
                vec![Operand::IdRef(value), Operand::IdRef(zero)],
            ));
            let result = self.result_id(name, &result_ty)?;
            instructions.push(Self::inst(
                Op::Select,
                Some(result_type),
                Some(result),
                vec![
                    Operand::IdRef(is_zero),
                    Operand::IdRef(bit_width),
                    Operand::IdRef(lsb),
                ],
            ));
        }
        self.record_int_alignment(name, &result_ty, 1);
        Ok(true)
    }
}
