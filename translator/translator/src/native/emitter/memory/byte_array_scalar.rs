use super::*;

impl Emitter {
    pub(in crate::native::emitter) fn emit_raw_scalar_load(
        &mut self,
        result: Word,
        ty: &LlType,
        raw: &RawBufferOffset,
        extra_byte: u64,
        access_align: Option<u64>,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let ty = self.resolve_type(ty)?;
        if self.emit_raw_byte_array_integer_load(result, &ty, raw, extra_byte, instructions)? {
            return Ok(());
        }
        match ty {
            LlType::Float => {
                let word = self.emit_raw_word_load_for_access(
                    raw,
                    extra_byte,
                    access_align,
                    instructions,
                )?;
                let result_type = self.type_id(&LlType::Float)?;
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(result_type),
                    Some(result),
                    vec![Operand::IdRef(word)],
                ));
            }
            LlType::Int(32) => {
                let word = self.emit_raw_word_load_for_access(
                    raw,
                    extra_byte,
                    access_align,
                    instructions,
                )?;
                let result_type = self.type_id(&LlType::Int(32))?;
                instructions.push(Self::inst(
                    Op::CopyObject,
                    Some(result_type),
                    Some(result),
                    vec![Operand::IdRef(word)],
                ));
            }
            LlType::Int(64) => {
                let low_word = self.emit_raw_word_load_for_access(
                    raw,
                    extra_byte,
                    access_align,
                    instructions,
                )?;
                let high_word = self.emit_raw_word_load_for_access(
                    raw,
                    extra_byte + 4,
                    access_align,
                    instructions,
                )?;
                let result_type = self.type_id(&LlType::Int(64))?;
                let low = self.fresh();
                instructions.push(Self::inst(
                    Op::UConvert,
                    Some(result_type),
                    Some(low),
                    vec![Operand::IdRef(low_word)],
                ));
                let high = self.fresh();
                instructions.push(Self::inst(
                    Op::UConvert,
                    Some(result_type),
                    Some(high),
                    vec![Operand::IdRef(high_word)],
                ));
                let shifted_high = self.fresh();
                let shift = self.const_signed_int(64, 32)?;
                instructions.push(Self::inst(
                    Op::ShiftLeftLogical,
                    Some(result_type),
                    Some(shifted_high),
                    vec![Operand::IdRef(high), Operand::IdRef(shift)],
                ));
                instructions.push(Self::inst(
                    Op::BitwiseOr,
                    Some(result_type),
                    Some(result),
                    vec![Operand::IdRef(low), Operand::IdRef(shifted_high)],
                ));
            }
            LlType::Half => {
                if !self.raw_offset_aligned_to(raw, extra_byte, 2) {
                    let bits =
                        self.emit_raw_u16_from_unaligned_bytes(raw, extra_byte, instructions)?;
                    let result_type = self.type_id(&LlType::Half)?;
                    instructions.push(Self::inst(
                        Op::Bitcast,
                        Some(result_type),
                        Some(result),
                        vec![Operand::IdRef(bits)],
                    ));
                    return Ok(());
                }
                let vec_ty = LlType::Vector(Box::new(LlType::Half), 2);
                let vec_ty_id = self.type_id(&vec_ty)?;
                let (word, lane) =
                    self.emit_raw_subword_source(raw, extra_byte, 2, instructions)?;
                let tmp = self.fresh();
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(vec_ty_id),
                    Some(tmp),
                    vec![Operand::IdRef(word)],
                ));
                let result_type = self.type_id(&LlType::Half)?;
                match lane {
                    RawSubwordLane::Static(lane) => {
                        instructions.push(Self::inst(
                            Op::CompositeExtract,
                            Some(result_type),
                            Some(result),
                            vec![Operand::IdRef(tmp), Operand::LiteralBit32(lane)],
                        ));
                    }
                    RawSubwordLane::Dynamic(lane) => {
                        instructions.push(Self::inst(
                            Op::VectorExtractDynamic,
                            Some(result_type),
                            Some(result),
                            vec![Operand::IdRef(tmp), Operand::IdRef(lane)],
                        ));
                    }
                }
            }
            LlType::Int(16) | LlType::BFloat => {
                if !self.raw_offset_aligned_to(raw, extra_byte, 2) {
                    let bits =
                        self.emit_raw_u16_from_unaligned_bytes(raw, extra_byte, instructions)?;
                    let result_type = self.type_id(&LlType::Int(16))?;
                    instructions.push(Self::inst(
                        Op::CopyObject,
                        Some(result_type),
                        Some(result),
                        vec![Operand::IdRef(bits)],
                    ));
                    return Ok(());
                }
                let vec_ty = LlType::Vector(Box::new(LlType::Int(16)), 2);
                let vec_ty_id = self.type_id(&vec_ty)?;
                let (word, lane) =
                    self.emit_raw_subword_source(raw, extra_byte, 2, instructions)?;
                let tmp = self.fresh();
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(vec_ty_id),
                    Some(tmp),
                    vec![Operand::IdRef(word)],
                ));
                let result_type = self.type_id(&LlType::Int(16))?;
                match lane {
                    RawSubwordLane::Static(lane) => {
                        instructions.push(Self::inst(
                            Op::CompositeExtract,
                            Some(result_type),
                            Some(result),
                            vec![Operand::IdRef(tmp), Operand::LiteralBit32(lane)],
                        ));
                    }
                    RawSubwordLane::Dynamic(lane) => {
                        instructions.push(Self::inst(
                            Op::VectorExtractDynamic,
                            Some(result_type),
                            Some(result),
                            vec![Operand::IdRef(tmp), Operand::IdRef(lane)],
                        ));
                    }
                }
            }
            LlType::Int(8) => {
                let vec_ty = LlType::Vector(Box::new(LlType::Int(8)), 4);
                let vec_ty_id = self.type_id(&vec_ty)?;
                let (word, lane) =
                    self.emit_raw_subword_source(raw, extra_byte, 1, instructions)?;
                let tmp = self.fresh();
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(vec_ty_id),
                    Some(tmp),
                    vec![Operand::IdRef(word)],
                ));
                let result_type = self.type_id(&LlType::Int(8))?;
                match lane {
                    RawSubwordLane::Static(lane) => {
                        instructions.push(Self::inst(
                            Op::CompositeExtract,
                            Some(result_type),
                            Some(result),
                            vec![Operand::IdRef(tmp), Operand::LiteralBit32(lane)],
                        ));
                    }
                    RawSubwordLane::Dynamic(lane) => {
                        instructions.push(Self::inst(
                            Op::VectorExtractDynamic,
                            Some(result_type),
                            Some(result),
                            vec![Operand::IdRef(tmp), Operand::IdRef(lane)],
                        ));
                    }
                }
            }
            LlType::Ptr(_) => {
                if raw.unmodelable {
                    return Err("native emitter: raw buffer offset is not modelable".into());
                }
                self.emit_private_zero_pointer_value_at(
                    result,
                    &LlType::Int(8),
                    "raw_buffer_offset_load_placeholder",
                )?;
            }
            other => {
                return Err(format!(
                    "native emitter: raw scalar load of {other:?} is not covered yet"
                ));
            }
        }
        Ok(())
    }

    pub(in crate::native::emitter) fn emit_raw_byte_array_integer_load(
        &mut self,
        result: Word,
        ty: &LlType,
        raw: &RawBufferOffset,
        extra_byte: u64,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        let Some(_len) = self.raw_byte_array_root_len(raw) else {
            return Ok(false);
        };
        let (bits, needs_bitcast) = match ty {
            LlType::Int(bits) => (*bits, false),
            LlType::Float => (32, true),
            LlType::Half => (16, true),
            _ => return Ok(false),
        };
        if bits % 8 != 0 {
            return Ok(false);
        }
        let byte_count = bits / 8;
        if byte_count == 0 {
            return Ok(false);
        }

        let int_ty = LlType::Int(bits);
        let result_ty = self.type_id(&int_ty)?;
        let assembled_result = if needs_bitcast { self.fresh() } else { result };
        let mut acc = None;
        for byte in 0..byte_count {
            let loaded = self.emit_raw_byte_array_byte_load(
                raw,
                extra_byte + u64::from(byte),
                instructions,
            )?;
            let widened = if bits == 8 {
                loaded
            } else {
                let widened = self.fresh();
                instructions.push(Self::inst(
                    Op::UConvert,
                    Some(result_ty),
                    Some(widened),
                    vec![Operand::IdRef(loaded)],
                ));
                widened
            };
            let term = if byte == 0 {
                widened
            } else {
                let shift = self.const_signed_int(bits, i64::from(byte * 8))?;
                let shifted = self.fresh();
                instructions.push(Self::inst(
                    Op::ShiftLeftLogical,
                    Some(result_ty),
                    Some(shifted),
                    vec![Operand::IdRef(widened), Operand::IdRef(shift)],
                ));
                shifted
            };
            acc = Some(if let Some(prev) = acc {
                let combined = if byte + 1 == byte_count {
                    assembled_result
                } else {
                    self.fresh()
                };
                instructions.push(Self::inst(
                    Op::BitwiseOr,
                    Some(result_ty),
                    Some(combined),
                    vec![Operand::IdRef(prev), Operand::IdRef(term)],
                ));
                combined
            } else {
                term
            });
        }
        if acc != Some(assembled_result) {
            instructions.push(Self::inst(
                Op::CopyObject,
                Some(result_ty),
                Some(assembled_result),
                vec![Operand::IdRef(acc.ok_or_else(|| {
                    "native emitter: raw byte-array integer load produced no accumulator \
                     (byte count must be >= 1)"
                        .to_string()
                })?)],
            ));
        }
        if needs_bitcast {
            let value_ty = self.type_id(ty)?;
            instructions.push(Self::inst(
                Op::Bitcast,
                Some(value_ty),
                Some(result),
                vec![Operand::IdRef(assembled_result)],
            ));
        }
        Ok(true)
    }

    pub(in crate::native::emitter) fn raw_byte_array_root_len(
        &self,
        raw: &RawBufferOffset,
    ) -> Option<u32> {
        let LlType::Array(elem, len) = self.pointer_pointees.get(&raw.root)? else {
            return None;
        };
        (elem.as_ref() == &LlType::Int(8)).then_some(*len)
    }

    pub(in crate::native::emitter) fn raw_root_value_id(
        &mut self,
        raw: &RawBufferOffset,
    ) -> Result<Word, String> {
        let value = if raw.root.starts_with('@') {
            LlValue::Global(raw.root.clone())
        } else {
            LlValue::Local(raw.root.clone())
        };
        self.value_id(&value, &LlType::Ptr(raw.addrspace))
    }

    pub(in crate::native::emitter) fn emit_raw_byte_array_byte_load(
        &mut self,
        raw: &RawBufferOffset,
        extra_byte: u64,
        instructions: &mut Vec<Instruction>,
    ) -> Result<Word, String> {
        let root_id = self.raw_root_value_id(raw)?;
        let storage = self.raw_access_storage(raw)?;
        let byte_ptr_ty = self.ptr_type_id(storage, &LlType::Int(8))?;
        let byte_index = self.emit_raw_byte_index(raw, extra_byte, instructions)?;
        let byte_ptr = self.fresh();
        instructions.push(Self::inst(
            Op::InBoundsAccessChain,
            Some(byte_ptr_ty),
            Some(byte_ptr),
            vec![Operand::IdRef(root_id), Operand::IdRef(byte_index)],
        ));
        let byte_ty = self.type_id(&LlType::Int(8))?;
        let loaded = self.fresh();
        instructions.push(Self::inst(
            Op::Load,
            Some(byte_ty),
            Some(loaded),
            vec![Operand::IdRef(byte_ptr)],
        ));
        Ok(loaded)
    }

    pub(in crate::native::emitter) fn emit_raw_byte_array_integer_store(
        &mut self,
        ty: &LlType,
        value: Word,
        raw: &RawBufferOffset,
        extra_byte: u64,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        if self.raw_byte_array_root_len(raw).is_none() {
            return Ok(false);
        }
        let (bits, word) = match ty {
            LlType::Int(bits) if bits % 8 == 0 && *bits > 0 => (*bits, value),
            LlType::Float => {
                let word_ty = self.type_id(&LlType::Int(32))?;
                let word = self.fresh();
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(word_ty),
                    Some(word),
                    vec![Operand::IdRef(value)],
                ));
                (32, word)
            }
            LlType::Half => {
                let half_bits_ty = self.type_id(&LlType::Int(16))?;
                let word = self.fresh();
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(half_bits_ty),
                    Some(word),
                    vec![Operand::IdRef(value)],
                ));
                (16, word)
            }
            _ => return Ok(false),
        };
        let int_ty = self.type_id(&LlType::Int(bits))?;
        let byte_ty = self.type_id(&LlType::Int(8))?;
        let root_id = self.raw_root_value_id(raw)?;
        let storage = self.raw_access_storage(raw)?;
        let byte_ptr_ty = self.ptr_type_id(storage, &LlType::Int(8))?;
        for byte in 0..bits / 8 {
            let shifted = if byte == 0 {
                word
            } else {
                let shift = self.const_signed_int(bits, i64::from(byte * 8))?;
                let shifted = self.fresh();
                instructions.push(Self::inst(
                    Op::ShiftRightLogical,
                    Some(int_ty),
                    Some(shifted),
                    vec![Operand::IdRef(word), Operand::IdRef(shift)],
                ));
                shifted
            };
            let byte_value = if bits == 8 {
                shifted
            } else {
                let narrowed = self.fresh();
                instructions.push(Self::inst(
                    Op::UConvert,
                    Some(byte_ty),
                    Some(narrowed),
                    vec![Operand::IdRef(shifted)],
                ));
                narrowed
            };
            let byte_index =
                self.emit_raw_byte_index(raw, extra_byte + u64::from(byte), instructions)?;
            let byte_ptr = self.fresh();
            instructions.push(Self::inst(
                Op::InBoundsAccessChain,
                Some(byte_ptr_ty),
                Some(byte_ptr),
                vec![Operand::IdRef(root_id), Operand::IdRef(byte_index)],
            ));
            instructions.push(Self::inst(
                Op::Store,
                None,
                None,
                vec![Operand::IdRef(byte_ptr), Operand::IdRef(byte_value)],
            ));
        }
        Ok(true)
    }

    pub(in crate::native::emitter) fn emit_raw_u16_from_unaligned_bytes(
        &mut self,
        raw: &RawBufferOffset,
        extra_byte: u64,
        instructions: &mut Vec<Instruction>,
    ) -> Result<Word, String> {
        let uint_ty = self.type_id(&LlType::Int(32))?;
        let byte_index = self.emit_raw_byte_index(raw, extra_byte, instructions)?;
        let word_index = self.fresh();
        let word_divisor = self.const_uint(4)?;
        instructions.push(Self::inst(
            Op::UDiv,
            Some(uint_ty),
            Some(word_index),
            vec![Operand::IdRef(byte_index), Operand::IdRef(word_divisor)],
        ));
        let word = self.emit_raw_word_load_at_index(raw, word_index, instructions)?;

        let byte_lane = self.fresh();
        let lane_mask = self.const_uint(3)?;
        instructions.push(Self::inst(
            Op::BitwiseAnd,
            Some(uint_ty),
            Some(byte_lane),
            vec![Operand::IdRef(byte_index), Operand::IdRef(lane_mask)],
        ));

        let low_shift = self.fresh();
        let bits_per_byte = self.const_uint(8)?;
        instructions.push(Self::inst(
            Op::IMul,
            Some(uint_ty),
            Some(low_shift),
            vec![Operand::IdRef(byte_lane), Operand::IdRef(bits_per_byte)],
        ));
        let shifted_low = self.fresh();
        instructions.push(Self::inst(
            Op::ShiftRightLogical,
            Some(uint_ty),
            Some(shifted_low),
            vec![Operand::IdRef(word), Operand::IdRef(low_shift)],
        ));
        let byte_mask = self.const_uint(0xff)?;
        let low_byte = self.fresh();
        instructions.push(Self::inst(
            Op::BitwiseAnd,
            Some(uint_ty),
            Some(low_byte),
            vec![Operand::IdRef(shifted_low), Operand::IdRef(byte_mask)],
        ));

        let next_lane_unmasked = self.fresh();
        let one = self.const_uint(1)?;
        instructions.push(Self::inst(
            Op::IAdd,
            Some(uint_ty),
            Some(next_lane_unmasked),
            vec![Operand::IdRef(byte_lane), Operand::IdRef(one)],
        ));
        let next_lane = self.fresh();
        instructions.push(Self::inst(
            Op::BitwiseAnd,
            Some(uint_ty),
            Some(next_lane),
            vec![
                Operand::IdRef(next_lane_unmasked),
                Operand::IdRef(lane_mask),
            ],
        ));
        let high_shift = self.fresh();
        instructions.push(Self::inst(
            Op::IMul,
            Some(uint_ty),
            Some(high_shift),
            vec![Operand::IdRef(next_lane), Operand::IdRef(bits_per_byte)],
        ));
        let shifted_high_same_word = self.fresh();
        instructions.push(Self::inst(
            Op::ShiftRightLogical,
            Some(uint_ty),
            Some(shifted_high_same_word),
            vec![Operand::IdRef(word), Operand::IdRef(high_shift)],
        ));
        let high_byte_same_word = self.fresh();
        instructions.push(Self::inst(
            Op::BitwiseAnd,
            Some(uint_ty),
            Some(high_byte_same_word),
            vec![
                Operand::IdRef(shifted_high_same_word),
                Operand::IdRef(byte_mask),
            ],
        ));

        let next_word_index = self.fresh();
        instructions.push(Self::inst(
            Op::IAdd,
            Some(uint_ty),
            Some(next_word_index),
            vec![Operand::IdRef(word_index), Operand::IdRef(one)],
        ));
        let next_word = self.emit_raw_word_load_at_index(raw, next_word_index, instructions)?;
        let high_byte_next_word = self.fresh();
        instructions.push(Self::inst(
            Op::BitwiseAnd,
            Some(uint_ty),
            Some(high_byte_next_word),
            vec![Operand::IdRef(next_word), Operand::IdRef(byte_mask)],
        ));

        let bool_ty = self.type_id(&LlType::Bool)?;
        let last_lane = self.fresh();
        let three = self.const_uint(3)?;
        instructions.push(Self::inst(
            Op::IEqual,
            Some(bool_ty),
            Some(last_lane),
            vec![Operand::IdRef(byte_lane), Operand::IdRef(three)],
        ));
        let high_byte = self.fresh();
        instructions.push(Self::inst(
            Op::Select,
            Some(uint_ty),
            Some(high_byte),
            vec![
                Operand::IdRef(last_lane),
                Operand::IdRef(high_byte_next_word),
                Operand::IdRef(high_byte_same_word),
            ],
        ));

        let high_byte_shifted = self.fresh();
        instructions.push(Self::inst(
            Op::ShiftLeftLogical,
            Some(uint_ty),
            Some(high_byte_shifted),
            vec![Operand::IdRef(high_byte), Operand::IdRef(bits_per_byte)],
        ));
        let assembled = self.fresh();
        instructions.push(Self::inst(
            Op::BitwiseOr,
            Some(uint_ty),
            Some(assembled),
            vec![Operand::IdRef(low_byte), Operand::IdRef(high_byte_shifted)],
        ));

        let u16_ty = self.type_id(&LlType::Int(16))?;
        let narrowed = self.fresh();
        instructions.push(Self::inst(
            Op::UConvert,
            Some(u16_ty),
            Some(narrowed),
            vec![Operand::IdRef(assembled)],
        ));
        Ok(narrowed)
    }
}
