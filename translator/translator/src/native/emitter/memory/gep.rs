use super::*;

impl Emitter {
    pub(in crate::native::emitter) fn compose_followup_gep(
        &mut self,
        name: &str,
        prev: &GepProvenance,
        source: &LlType,
        indices: &[TypedValue],
        instructions: &mut Vec<Instruction>,
    ) -> Result<Option<Vec<TypedValue>>, String> {
        if let Some(indices) = self.compose_zero_wrapper_gep(prev, source, indices, instructions)? {
            return Ok(Some(indices));
        }

        if let Some(indices) =
            self.compose_linear_element_gep(prev, source, indices, instructions)?
        {
            return Ok(Some(indices));
        }

        self.compose_scalar_array_reinterpret_gep(name, prev, source, indices, instructions)
    }

    pub(in crate::native::emitter) fn compose_zero_wrapper_gep(
        &mut self,
        prev: &GepProvenance,
        source: &LlType,
        indices: &[TypedValue],
        instructions: &mut Vec<Instruction>,
    ) -> Result<Option<Vec<TypedValue>>, String> {
        if &prev.source_ty != source || !is_zero_wrapper_source(source) {
            return Ok(None);
        }
        let Some(prev_index) = wrapper_gep_index(&prev.indices) else {
            return Ok(None);
        };
        let Some(next_index) = wrapper_gep_index(indices) else {
            return Ok(None);
        };
        if const_index(Some(next_index)) == Some(0) {
            return Ok(None);
        }
        let combined_index = self.combine_gep_indices(prev_index, next_index, instructions)?;
        let mut combined = prev.indices.clone();
        if let Some(last) = combined.last_mut() {
            *last = combined_index;
        }
        Ok(Some(combined))
    }

    pub(in crate::native::emitter) fn compose_linear_element_gep(
        &mut self,
        prev: &GepProvenance,
        source: &LlType,
        indices: &[TypedValue],
        instructions: &mut Vec<Instruction>,
    ) -> Result<Option<Vec<TypedValue>>, String> {
        if indices.is_empty() || prev.indices.is_empty() {
            return Ok(None);
        }
        if gep_pointee(&prev.source_ty, &prev.indices)? != *source {
            return Ok(None);
        }
        if !gep_can_offset_element_pointer(&prev.source_ty, &prev.indices, source) {
            return Ok(None);
        }

        let mut combined = prev.indices.clone();
        if let Some(last) = combined.last_mut() {
            *last = self.combine_gep_indices(last, &indices[0], instructions)?;
        }
        combined.extend(indices.iter().skip(1).cloned());
        Ok(Some(combined))
    }

    pub(in crate::native::emitter) fn compose_scalar_array_reinterpret_gep(
        &mut self,
        name: &str,
        prev: &GepProvenance,
        source: &LlType,
        indices: &[TypedValue],
        instructions: &mut Vec<Instruction>,
    ) -> Result<Option<Vec<TypedValue>>, String> {
        if indices.is_empty() || indices.len() > 2 || prev.indices.len() != 2 {
            return Ok(None);
        }
        let prev_source = self.resolve_type(&prev.source_ty)?;
        let source = self.resolve_type(source)?;
        if !types_compatible(&prev_source, &source) {
            return Ok(None);
        }
        let LlType::Array(elem, len) = source else {
            return Ok(None);
        };
        if len == 0 {
            return Ok(None);
        }
        let elem = self.resolve_type(&elem)?;
        if matches!(
            elem,
            LlType::Array(_, _) | LlType::Struct(_) | LlType::Vector(_, _) | LlType::Ptr(_)
        ) {
            return Ok(None);
        }
        let prev_pointee = self.resolve_type(&gep_pointee(&prev.source_ty, &prev.indices)?)?;
        if !types_compatible(&prev_pointee, &elem) {
            return Ok(None);
        }

        let mut combined = prev.indices.clone();
        let mut linear = combined
            .pop()
            .expect("prev.indices length checked above for scalar array reinterpret GEP");
        let scaled_object = self.scale_gep_index(&indices[0], len, name, instructions)?;
        linear = self.combine_gep_indices(&linear, &scaled_object, instructions)?;
        if let Some(member) = indices.get(1) {
            linear = self.combine_gep_indices(&linear, member, instructions)?;
        }
        combined.push(linear);
        Ok(Some(combined))
    }

    pub(in crate::native::emitter) fn combine_gep_indices(
        &mut self,
        lhs: &TypedValue,
        rhs: &TypedValue,
        instructions: &mut Vec<Instruction>,
    ) -> Result<TypedValue, String> {
        match (const_index(Some(lhs)), const_index(Some(rhs))) {
            (Some(a), Some(b)) => {
                let mut out = lhs.clone();
                out.value = LlValue::Int((a + b) as u64);
                return Ok(out);
            }
            (_, Some(0)) => return Ok(lhs.clone()),
            (Some(0), _) => return Ok(rhs.clone()),
            _ => {}
        }

        let lhs_ty = self.resolve_type(&lhs.ty)?;
        let rhs_ty = self.resolve_type(&rhs.ty)?;
        let (LlType::Int(lhs_bits), LlType::Int(rhs_bits)) = (&lhs_ty, &rhs_ty) else {
            return Err(format!(
                "native emitter: cannot compose GEP indices with types {lhs_ty:?} and {rhs_ty:?}"
            ));
        };
        let result_bits = (*lhs_bits).max(*rhs_bits);
        let result_ty = LlType::Int(result_bits);
        let result_type = self.type_id(&result_ty)?;
        let lhs_original = self.value_id(&lhs.value, &lhs.ty)?;
        let rhs_original = self.value_id(&rhs.value, &rhs.ty)?;
        let mut widen = |id, bits| {
            if bits == result_bits {
                return id;
            }
            let widened = self.fresh();
            instructions.push(Self::inst(
                Op::SConvert,
                Some(result_type),
                Some(widened),
                vec![Operand::IdRef(id)],
            ));
            widened
        };
        let lhs_id = widen(lhs_original, *lhs_bits);
        let rhs_id = widen(rhs_original, *rhs_bits);
        let result = self.fresh();
        instructions.push(Self::inst(
            Op::IAdd,
            Some(result_type),
            Some(result),
            vec![Operand::IdRef(lhs_id), Operand::IdRef(rhs_id)],
        ));
        let name = format!("%air.gepidx.{result}");
        self.values
            .insert(name.clone(), (result, result_ty.clone()));
        self.record_int_alignment(
            &name,
            &result_ty,
            add_int_alignment(
                self.int_value_alignment(&lhs.value),
                self.int_value_alignment(&rhs.value),
            ),
        );
        Ok(TypedValue {
            ty: result_ty,
            value: LlValue::Local(name),
        })
    }

    pub(in crate::native::emitter) fn apply_raw_gep(
        &self,
        mut raw: RawBufferOffset,
        source: &LlType,
        indices: &[TypedValue],
    ) -> Result<RawBufferOffset, String> {
        let mut cur = self.resolve_type(source)?;
        if let Some(first) = indices.first() {
            let (size, _) = self.raw_type_size_align(&cur)?;
            if let Some(value) = const_index_i64(first) {
                raw.const_off += value * size as i64;
            } else {
                raw.dyn_terms.push((first.clone(), size as i64));
            }
        }

        for index in indices.iter().skip(1) {
            match cur {
                LlType::Struct(ref fields) => {
                    let Some(value) = const_index_i64(index) else {
                        raw.unmodelable = true;
                        return Ok(raw);
                    };
                    let (offset, member) = self.raw_struct_member(fields, value as u64)?;
                    raw.const_off += offset as i64;
                    cur = member;
                }
                LlType::Array(ref elem, _) => {
                    let (size, align) = self.raw_type_size_align(elem)?;
                    let stride = round_up_u64(size, align);
                    if let Some(value) = const_index_i64(index) {
                        raw.const_off += (value as u64 * stride) as i64;
                    } else {
                        raw.dyn_terms.push((index.clone(), stride as i64));
                    }
                    cur = self.resolve_type(elem)?;
                }
                LlType::Vector(ref elem, _) => {
                    let (size, _) = self.raw_type_size_align(elem)?;
                    if let Some(value) = const_index_i64(index) {
                        raw.const_off += (value as u64 * size) as i64;
                    } else {
                        raw.dyn_terms.push((index.clone(), size as i64));
                    }
                    cur = self.resolve_type(elem)?;
                }
                ref other => {
                    return Err(format!(
                        "native emitter: raw GEP through {other:?} is not covered yet"
                    ));
                }
            }
        }
        Ok(raw)
    }
}
