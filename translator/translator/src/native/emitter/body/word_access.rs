use super::*;

impl Emitter {
    pub(in crate::native::emitter) fn emit_join_i32_words_as_i64(
        &mut self,
        result: Word,
        low_word: Word,
        high_word: Word,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let uint64 = self.type_id(&LlType::Int(64))?;
        let low = self.fresh();
        instructions.push(Self::inst(
            Op::UConvert,
            Some(uint64),
            Some(low),
            vec![Operand::IdRef(low_word)],
        ));
        let high = self.fresh();
        instructions.push(Self::inst(
            Op::UConvert,
            Some(uint64),
            Some(high),
            vec![Operand::IdRef(high_word)],
        ));
        let shifted_high = self.fresh();
        let shift = self.const_signed_int(64, 32)?;
        instructions.push(Self::inst(
            Op::ShiftLeftLogical,
            Some(uint64),
            Some(shifted_high),
            vec![Operand::IdRef(high), Operand::IdRef(shift)],
        ));
        instructions.push(Self::inst(
            Op::BitwiseOr,
            Some(uint64),
            Some(result),
            vec![Operand::IdRef(low), Operand::IdRef(shifted_high)],
        ));
        Ok(())
    }

    pub(in crate::native::emitter) fn leading_i32_scalar_accesses(
        &self,
        ty: &LlType,
        byte_count: u64,
    ) -> Result<Option<Vec<(Vec<u32>, LlType)>>, String> {
        if byte_count == 0 || !byte_count.is_multiple_of(4) {
            return Ok(None);
        }
        let mut fields = Vec::new();
        self.collect_leading_i32_scalar_accesses(ty, 0, byte_count, &mut Vec::new(), &mut fields)?;
        fields.sort_by_key(|(offset, _, _)| *offset);
        if fields.len() != (byte_count / 4) as usize {
            return Ok(None);
        }
        for (index, (offset, _, _)) in fields.iter().enumerate() {
            if *offset != (index as u64) * 4 {
                return Ok(None);
            }
        }
        Ok(Some(
            fields.into_iter().map(|(_, path, ty)| (path, ty)).collect(),
        ))
    }

    pub(in crate::native::emitter) fn collect_leading_i32_scalar_accesses(
        &self,
        ty: &LlType,
        base_offset: u64,
        byte_limit: u64,
        path: &mut Vec<u32>,
        out: &mut Vec<(u64, Vec<u32>, LlType)>,
    ) -> Result<(), String> {
        if base_offset >= byte_limit {
            return Ok(());
        }
        match self.resolve_type(ty)? {
            LlType::Float | LlType::Int(32) => {
                if base_offset + 4 <= byte_limit {
                    out.push((base_offset, path.clone(), self.resolve_type(ty)?));
                }
            }
            LlType::Vector(elem, lanes) => {
                let elem = self.resolve_type(&elem)?;
                let (elem_size, _) = self.raw_type_size_align(&elem)?;
                for lane in 0..lanes {
                    let offset = base_offset + elem_size * u64::from(lane);
                    if offset >= byte_limit {
                        break;
                    }
                    path.push(lane);
                    self.collect_leading_i32_scalar_accesses(&elem, offset, byte_limit, path, out)?;
                    path.pop();
                }
            }
            LlType::Array(elem, len) => {
                let elem = self.resolve_type(&elem)?;
                let (elem_size, elem_align) = self.raw_type_size_align(&elem)?;
                let stride = round_up_u64(elem_size, elem_align);
                for index in 0..len {
                    let offset = base_offset + stride * u64::from(index);
                    if offset >= byte_limit {
                        break;
                    }
                    path.push(index);
                    self.collect_leading_i32_scalar_accesses(&elem, offset, byte_limit, path, out)?;
                    path.pop();
                }
            }
            LlType::Struct(fields) => {
                for index in 0..fields.len() {
                    let (offset, field) = self.raw_struct_member(&fields, index as u64)?;
                    let offset = base_offset + offset;
                    if offset >= byte_limit {
                        break;
                    }
                    path.push(index as u32);
                    self.collect_leading_i32_scalar_accesses(
                        &field, offset, byte_limit, path, out,
                    )?;
                    path.pop();
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(in crate::native::emitter) fn leading_byte_scalar_accesses(
        &self,
        ty: &LlType,
        byte_count: u64,
    ) -> Result<Option<Vec<Vec<u32>>>, String> {
        if byte_count == 0 {
            return Ok(None);
        }
        let mut bytes = Vec::new();
        self.collect_leading_byte_scalar_accesses(ty, 0, byte_count, &mut Vec::new(), &mut bytes)?;
        bytes.sort_by_key(|(offset, _)| *offset);
        if bytes.len() as u64 != byte_count {
            return Ok(None);
        }
        for (index, (offset, _)) in bytes.iter().enumerate() {
            if *offset != index as u64 {
                return Ok(None);
            }
        }
        Ok(Some(bytes.into_iter().map(|(_, path)| path).collect()))
    }

    pub(in crate::native::emitter) fn collect_leading_byte_scalar_accesses(
        &self,
        ty: &LlType,
        base_offset: u64,
        byte_limit: u64,
        path: &mut Vec<u32>,
        out: &mut Vec<(u64, Vec<u32>)>,
    ) -> Result<(), String> {
        if base_offset >= byte_limit {
            return Ok(());
        }
        match self.resolve_type(ty)? {
            LlType::Int(8) => out.push((base_offset, path.clone())),
            LlType::Vector(elem, lanes) => {
                let elem = self.resolve_type(&elem)?;
                let (elem_size, _) = self.raw_type_size_align(&elem)?;
                for lane in 0..lanes {
                    let offset = base_offset + elem_size * u64::from(lane);
                    if offset >= byte_limit {
                        break;
                    }
                    path.push(lane);
                    self.collect_leading_byte_scalar_accesses(
                        &elem, offset, byte_limit, path, out,
                    )?;
                    path.pop();
                }
            }
            LlType::Array(elem, len) => {
                let elem = self.resolve_type(&elem)?;
                let (elem_size, elem_align) = self.raw_type_size_align(&elem)?;
                let stride = round_up_u64(elem_size, elem_align);
                for index in 0..len {
                    let offset = base_offset + stride * u64::from(index);
                    if offset >= byte_limit {
                        break;
                    }
                    path.push(index);
                    self.collect_leading_byte_scalar_accesses(
                        &elem, offset, byte_limit, path, out,
                    )?;
                    path.pop();
                }
            }
            LlType::Struct(fields) => {
                for index in 0..fields.len() {
                    let (offset, field) = self.raw_struct_member(&fields, index as u64)?;
                    let offset = base_offset + offset;
                    if offset >= byte_limit {
                        break;
                    }
                    path.push(index as u32);
                    self.collect_leading_byte_scalar_accesses(
                        &field, offset, byte_limit, path, out,
                    )?;
                    path.pop();
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(in crate::native::emitter) fn byte_value_lanes(
        &self,
        ty: &LlType,
    ) -> Result<Option<(u32, u32, LlType)>, String> {
        fn lane(ty: &LlType) -> Option<(u32, LlType)> {
            match ty {
                LlType::Int(bits) if *bits >= 8 && bits % 8 == 0 => Some((bits / 8, ty.clone())),
                LlType::Half => Some((2, LlType::Half)),
                LlType::Float => Some((4, LlType::Float)),
                _ => None,
            }
        }
        Ok(match self.resolve_type(ty)? {
            LlType::Vector(elem, lanes) if lanes > 0 => {
                lane(&self.resolve_type(&elem)?).map(|(bytes, lane_ty)| (lanes, bytes, lane_ty))
            }
            other => lane(&other).map(|(bytes, lane_ty)| (1, bytes, lane_ty)),
        })
    }

    pub(in crate::native::emitter) fn emit_value_from_bytes(
        &mut self,
        result: Word,
        result_ty: &LlType,
        bytes: &[Word],
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let (lanes, lane_bytes, lane_ty) = self.byte_value_lanes(result_ty)?.ok_or_else(|| {
            format!("native emitter: {result_ty:?} cannot be assembled from bytes")
        })?;
        if bytes.len() != (lanes * lane_bytes) as usize {
            return Err(format!(
                "native emitter: {} byte(s) cannot assemble {result_ty:?} ({lanes} x {lane_bytes} B)",
                bytes.len()
            ));
        }
        let int_bits = lane_bytes * 8;
        let int_ty_id = self.type_id(&LlType::Int(int_bits))?;
        let mut lane_ids = Vec::with_capacity(lanes as usize);
        for lane in 0..lanes as usize {
            let mut acc: Option<Word> = None;
            for byte in 0..lane_bytes as usize {
                let loaded = bytes[lane * lane_bytes as usize + byte];
                let widened = if int_bits == 8 {
                    loaded
                } else {
                    let widened = self.fresh();
                    instructions.push(Self::inst(
                        Op::UConvert,
                        Some(int_ty_id),
                        Some(widened),
                        vec![Operand::IdRef(loaded)],
                    ));
                    widened
                };
                let term = if byte == 0 {
                    widened
                } else {
                    let shift = self.const_signed_int(int_bits, (byte as i64) * 8)?;
                    let shifted = self.fresh();
                    instructions.push(Self::inst(
                        Op::ShiftLeftLogical,
                        Some(int_ty_id),
                        Some(shifted),
                        vec![Operand::IdRef(widened), Operand::IdRef(shift)],
                    ));
                    shifted
                };
                acc = Some(match acc {
                    None => term,
                    Some(prev) => {
                        let combined = self.fresh();
                        instructions.push(Self::inst(
                            Op::BitwiseOr,
                            Some(int_ty_id),
                            Some(combined),
                            vec![Operand::IdRef(prev), Operand::IdRef(term)],
                        ));
                        combined
                    }
                });
            }
            let packed =
                acc.ok_or_else(|| "native emitter: a byte lane had no bytes".to_string())?;
            let lane_value = if matches!(lane_ty, LlType::Int(_)) {
                packed
            } else {
                let lane_ty_id = self.type_id(&lane_ty)?;
                let cast = self.fresh();
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(lane_ty_id),
                    Some(cast),
                    vec![Operand::IdRef(packed)],
                ));
                cast
            };
            lane_ids.push(lane_value);
        }
        let result_ty_id = self.type_id(result_ty)?;
        if lanes == 1 {
            instructions.push(Self::inst(
                Op::CopyObject,
                Some(result_ty_id),
                Some(result),
                vec![Operand::IdRef(lane_ids[0])],
            ));
        } else {
            instructions.push(Self::inst(
                Op::CompositeConstruct,
                Some(result_ty_id),
                Some(result),
                lane_ids.into_iter().map(Operand::IdRef).collect(),
            ));
        }
        Ok(())
    }

    pub(in crate::native::emitter) fn emit_value_to_bytes(
        &mut self,
        value: Word,
        value_ty: &LlType,
        instructions: &mut Vec<Instruction>,
    ) -> Result<Vec<Word>, String> {
        let (lanes, lane_bytes, lane_ty) = self
            .byte_value_lanes(value_ty)?
            .ok_or_else(|| format!("native emitter: {value_ty:?} cannot be split into bytes"))?;
        let int_bits = lane_bytes * 8;
        let int_ty_id = self.type_id(&LlType::Int(int_bits))?;
        let byte_ty_id = self.type_id(&LlType::Int(8))?;
        let mut bytes = Vec::with_capacity((lanes * lane_bytes) as usize);
        for lane in 0..lanes {
            let lane_value = if lanes == 1 {
                value
            } else {
                let lane_ty_id = self.type_id(&lane_ty)?;
                let extracted = self.fresh();
                instructions.push(Self::inst(
                    Op::CompositeExtract,
                    Some(lane_ty_id),
                    Some(extracted),
                    vec![Operand::IdRef(value), Operand::LiteralBit32(lane)],
                ));
                extracted
            };
            let as_int = if matches!(lane_ty, LlType::Int(_)) {
                lane_value
            } else {
                let cast = self.fresh();
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(int_ty_id),
                    Some(cast),
                    vec![Operand::IdRef(lane_value)],
                ));
                cast
            };
            for byte in 0..lane_bytes {
                let shifted = if byte == 0 {
                    as_int
                } else {
                    let shift = self.const_signed_int(int_bits, i64::from(byte * 8))?;
                    let shifted = self.fresh();
                    instructions.push(Self::inst(
                        Op::ShiftRightLogical,
                        Some(int_ty_id),
                        Some(shifted),
                        vec![Operand::IdRef(as_int), Operand::IdRef(shift)],
                    ));
                    shifted
                };
                bytes.push(if int_bits == 8 {
                    shifted
                } else {
                    let narrowed = self.fresh();
                    instructions.push(Self::inst(
                        Op::UConvert,
                        Some(byte_ty_id),
                        Some(narrowed),
                        vec![Operand::IdRef(shifted)],
                    ));
                    narrowed
                });
            }
        }
        Ok(bytes)
    }

    pub(in crate::native::emitter) fn emit_i32_scalar_field_load(
        &mut self,
        ptr: Word,
        storage: StorageClass,
        access_path: &[u32],
        field_ty: &LlType,
        instructions: &mut Vec<Instruction>,
    ) -> Result<Word, String> {
        let field_ptr_ty = self.ptr_type_id(storage, field_ty)?;
        let field_ptr = self.fresh();
        let mut ops = vec![Operand::IdRef(ptr)];
        for index in access_path {
            ops.push(Operand::IdRef(self.const_uint(*index)?));
        }
        instructions.push(Self::inst(
            Op::InBoundsAccessChain,
            Some(field_ptr_ty),
            Some(field_ptr),
            ops,
        ));

        let field_type = self.type_id(field_ty)?;
        let loaded = self.fresh();
        instructions.push(Self::inst(
            Op::Load,
            Some(field_type),
            Some(loaded),
            vec![Operand::IdRef(field_ptr)],
        ));
        match self.resolve_type(field_ty)? {
            LlType::Int(32) => Ok(loaded),
            LlType::Float => {
                let uint = self.type_id(&LlType::Int(32))?;
                let word = self.fresh();
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(uint),
                    Some(word),
                    vec![Operand::IdRef(loaded)],
                ));
                Ok(word)
            }
            other => Err(format!(
                "native emitter: aggregate prefix scalar load field {other:?} is not i32-sized"
            )),
        }
    }

    pub(in crate::native::emitter) fn emit_i32_scalar_field_store(
        &mut self,
        ptr: Word,
        storage: StorageClass,
        access_path: &[u32],
        field_ty: &LlType,
        word: Word,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let field_ptr_ty = self.ptr_type_id(storage, field_ty)?;
        let field_ptr = self.fresh();
        let mut ops = vec![Operand::IdRef(ptr)];
        for index in access_path {
            ops.push(Operand::IdRef(self.const_uint(*index)?));
        }
        instructions.push(Self::inst(
            Op::InBoundsAccessChain,
            Some(field_ptr_ty),
            Some(field_ptr),
            ops,
        ));

        let stored = match self.resolve_type(field_ty)? {
            LlType::Int(32) => word,
            LlType::Float => {
                let float = self.type_id(&LlType::Float)?;
                let cast = self.fresh();
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(float),
                    Some(cast),
                    vec![Operand::IdRef(word)],
                ));
                cast
            }
            other => {
                return Err(format!(
                    "native emitter: aggregate prefix scalar store field {other:?} is not i32-sized"
                ))
            }
        };
        instructions.push(Self::inst(
            Op::Store,
            None,
            None,
            vec![Operand::IdRef(field_ptr), Operand::IdRef(stored)],
        ));
        Ok(())
    }

    pub(in crate::native::emitter) fn emit_i32_pair_struct_field_store(
        &mut self,
        ptr: Word,
        storage: StorageClass,
        member: u32,
        value: Word,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let field_ptr_ty = self.ptr_type_id(storage, &LlType::Int(32))?;
        let index = self.const_uint(member)?;
        let field_ptr = self.fresh();
        instructions.push(Self::inst(
            Op::InBoundsAccessChain,
            Some(field_ptr_ty),
            Some(field_ptr),
            vec![Operand::IdRef(ptr), Operand::IdRef(index)],
        ));
        instructions.push(Self::inst(
            Op::Store,
            None,
            None,
            vec![Operand::IdRef(field_ptr), Operand::IdRef(value)],
        ));
        Ok(())
    }

    pub(in crate::native::emitter) fn vector_index_id(
        &mut self,
        idx: &TypedValue,
        instructions: &mut Vec<Instruction>,
    ) -> Result<Word, String> {
        if !is_bool_type(&self.resolve_type(&idx.ty)?) {
            return self.value_id_in(&idx.value, &idx.ty, instructions);
        }
        let uint_ty = self.type_id(&LlType::Int(32))?;
        let cond = self.value_id_in(&idx.value, &idx.ty, instructions)?;
        let one = self.const_uint(1)?;
        let zero = self.const_uint(0)?;
        let result = self.fresh();
        instructions.push(Self::inst(
            Op::Select,
            Some(uint_ty),
            Some(result),
            vec![
                Operand::IdRef(cond),
                Operand::IdRef(one),
                Operand::IdRef(zero),
            ],
        ));
        Ok(result)
    }

    pub(in crate::native::emitter) fn record_vector_word_pointer_gep(
        &mut self,
        name: &str,
        base: Word,
        base_storage: StorageClass,
        base_pointee: Option<&LlType>,
        source_ty: &LlType,
        indices: &[TypedValue],
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        if base_storage != StorageClass::Workgroup || indices.len() != 1 {
            return Ok(false);
        }
        let source_ty = self.resolve_type(source_ty)?;
        let base_points_to_vector_array = if let Some(base_pointee) = base_pointee {
            match self.resolve_type(base_pointee)? {
                LlType::Array(elem, _) => types_compatible(&elem, &source_ty),
                _ => false,
            }
        } else {
            false
        };
        if let Some(root) = self.vector_word_root_from_vector_ty(
            base_storage,
            &source_ty,
            !base_points_to_vector_array,
        )? {
            self.vector_word_roots.entry(base).or_insert(root);
            return Ok(false);
        }
        if source_ty != LlType::Int(32) {
            return Ok(false);
        }
        let root = if let Some(base_pointee) = base_pointee {
            self.vector_word_root_from_pointee(base_storage, base_pointee)?
        } else {
            None
        }
        .or_else(|| self.vector_word_roots.get(&base).cloned());
        let Some(root) = root else {
            return Ok(false);
        };
        let word_index = self.raw_index_u32(&indices[0], instructions)?;
        self.vector_word_pointers.insert(
            name.to_string(),
            VectorWordPointer {
                base,
                storage: root.storage,
                vector_ty: root.vector_ty,
                lanes: root.lanes,
                lanes_per_word: root.lanes_per_word,
                words_per_vector: root.words_per_vector,
                base_is_vector_pointer: root.base_is_vector_pointer,
                word_index,
            },
        );
        self.pointer_storage.insert(name.to_string(), root.storage);
        self.pointer_pointees
            .insert(name.to_string(), LlType::Int(32));
        if !self.pointer_phi_values.is_empty() {
            let is_null = self.const_bool(false)?;
            self.record_pointer_nullness(name.to_string(), is_null);
        }
        Ok(true)
    }

    pub(in crate::native::emitter) fn vector_word_root_from_pointee(
        &mut self,
        storage: StorageClass,
        pointee: &LlType,
    ) -> Result<Option<VectorWordRoot>, String> {
        match self.resolve_type(pointee)? {
            LlType::Array(vector_ty, _) => {
                self.vector_word_root_from_vector_ty(storage, &vector_ty, false)
            }
            LlType::Vector(_, _) => self.vector_word_root_from_vector_ty(storage, pointee, true),
            _ => Ok(None),
        }
    }

    pub(in crate::native::emitter) fn vector_word_root_from_vector_ty(
        &mut self,
        storage: StorageClass,
        vector_ty: &LlType,
        base_is_vector_pointer: bool,
    ) -> Result<Option<VectorWordRoot>, String> {
        let vector_ty = self.resolve_type(vector_ty)?;
        let LlType::Vector(elem_ty, lanes) = &vector_ty else {
            return Ok(None);
        };
        let lanes = *lanes;
        let elem_ty = self.resolve_type(elem_ty)?;
        let Some(elem_bits) = bitcast_width(&elem_ty) else {
            return Ok(None);
        };
        if elem_bits != 16 || lanes == 0 || 32 % elem_bits != 0 {
            return Ok(None);
        }
        let lanes_per_word = 32 / elem_bits;
        if lanes < lanes_per_word || lanes % lanes_per_word != 0 {
            return Ok(None);
        }
        Ok(Some(VectorWordRoot {
            storage,
            vector_ty,
            lanes,
            lanes_per_word,
            words_per_vector: lanes / lanes_per_word,
            base_is_vector_pointer,
        }))
    }

    pub(in crate::native::emitter) fn emit_vector_word_element_pointer(
        &mut self,
        pointer: &VectorWordPointer,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(Word, Word), String> {
        let uint_ty = self.type_id(&LlType::Int(32))?;
        let vector_index = if pointer.words_per_vector == 1 {
            pointer.word_index
        } else {
            let divisor = self.const_uint(pointer.words_per_vector)?;
            let result = self.fresh();
            instructions.push(Self::inst(
                Op::UDiv,
                Some(uint_ty),
                Some(result),
                vec![Operand::IdRef(pointer.word_index), Operand::IdRef(divisor)],
            ));
            result
        };
        let word_in_vector = if pointer.words_per_vector == 1 {
            self.const_uint(0)?
        } else {
            let divisor = self.const_uint(pointer.words_per_vector)?;
            let result = self.fresh();
            instructions.push(Self::inst(
                Op::UMod,
                Some(uint_ty),
                Some(result),
                vec![Operand::IdRef(pointer.word_index), Operand::IdRef(divisor)],
            ));
            result
        };
        let lane_base = if pointer.lanes_per_word == 1 {
            word_in_vector
        } else {
            let scale = self.const_uint(pointer.lanes_per_word)?;
            let result = self.fresh();
            instructions.push(Self::inst(
                Op::IMul,
                Some(uint_ty),
                Some(result),
                vec![Operand::IdRef(word_in_vector), Operand::IdRef(scale)],
            ));
            result
        };
        let ptr_ty = self.ptr_type_id(pointer.storage, &pointer.vector_ty)?;
        let ptr = self.fresh();
        let op = if pointer.base_is_vector_pointer
            && ptr_access_chain_allowed_storage(pointer.storage)
            && !self.is_workgroup_indexed_container_root(pointer.base)
        {
            Op::PtrAccessChain
        } else {
            Op::InBoundsAccessChain
        };
        instructions.push(Self::inst(
            op,
            Some(ptr_ty),
            Some(ptr),
            vec![Operand::IdRef(pointer.base), Operand::IdRef(vector_index)],
        ));
        Ok((ptr, lane_base))
    }

    pub(in crate::native::emitter) fn emit_vector_word_high_lane(
        &mut self,
        lane_base: Word,
        instructions: &mut Vec<Instruction>,
    ) -> Result<Word, String> {
        let uint_ty = self.type_id(&LlType::Int(32))?;
        let one = self.const_uint(1)?;
        let high_lane = self.fresh();
        instructions.push(Self::inst(
            Op::IAdd,
            Some(uint_ty),
            Some(high_lane),
            vec![Operand::IdRef(lane_base), Operand::IdRef(one)],
        ));
        Ok(high_lane)
    }

    pub(in crate::native::emitter) fn vector_root_for_pointer(
        &mut self,
        ptr: &TypedValue,
    ) -> Result<Option<(Word, VectorWordRoot)>, String> {
        if matches!(ptr.value, LlValue::Gep(_)) {
            return Ok(None);
        }
        let ptr_id = self.value_id(&ptr.value, &ptr.ty)?;
        Ok(self
            .vector_word_roots
            .get(&ptr_id)
            .cloned()
            .map(|root| (ptr_id, root)))
    }

    pub(in crate::native::emitter) fn emit_vector_root_element_zero_pointer(
        &mut self,
        root_id: Word,
        root: &VectorWordRoot,
        instructions: &mut Vec<Instruction>,
    ) -> Result<Word, String> {
        let ptr_ty = self.ptr_type_id(root.storage, &root.vector_ty)?;
        let zero = self.const_uint(0)?;
        let ptr = self.fresh();
        let op = if root.base_is_vector_pointer
            && ptr_access_chain_allowed_storage(root.storage)
            && !self.is_workgroup_indexed_container_root(root_id)
        {
            Op::PtrAccessChain
        } else {
            Op::InBoundsAccessChain
        };
        instructions.push(Self::inst(
            op,
            Some(ptr_ty),
            Some(ptr),
            vec![Operand::IdRef(root_id), Operand::IdRef(zero)],
        ));
        Ok(ptr)
    }

    pub(in crate::native::emitter) fn emit_vector_root_store(
        &mut self,
        ptr: &TypedValue,
        object: &TypedValue,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        let Some((root_id, root)) = self.vector_root_for_pointer(ptr)? else {
            return Ok(false);
        };
        if !types_compatible(&self.resolve_type(&object.ty)?, &root.vector_ty) {
            return Ok(false);
        }
        let object_id = self.value_id_in(&object.value, &object.ty, instructions)?;
        let elem_ptr = self.emit_vector_root_element_zero_pointer(root_id, &root, instructions)?;
        instructions.push(Self::inst(
            Op::Store,
            None,
            None,
            vec![Operand::IdRef(elem_ptr), Operand::IdRef(object_id)],
        ));
        Ok(true)
    }

    pub(in crate::native::emitter) fn emit_vector_root_load(
        &mut self,
        result: Word,
        result_ty: &LlType,
        ptr: &TypedValue,
        instructions: &mut Vec<Instruction>,
    ) -> Result<bool, String> {
        let Some((root_id, root)) = self.vector_root_for_pointer(ptr)? else {
            return Ok(false);
        };
        if !types_compatible(&self.resolve_type(result_ty)?, &root.vector_ty) {
            return Ok(false);
        }
        let elem_ptr = self.emit_vector_root_element_zero_pointer(root_id, &root, instructions)?;
        let result_type = self.type_id(&root.vector_ty)?;
        instructions.push(Self::inst(
            Op::Load,
            Some(result_type),
            Some(result),
            vec![Operand::IdRef(elem_ptr)],
        ));
        Ok(true)
    }

    pub(in crate::native::emitter) fn emit_vector_word_bits_vector(
        &mut self,
        pointer: &VectorWordPointer,
        vector: Word,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(LlType, Word), String> {
        let bits_ty = LlType::Vector(Box::new(LlType::Int(16)), pointer.lanes);
        if types_compatible(&pointer.vector_ty, &bits_ty) {
            return Ok((bits_ty, vector));
        }
        let bits_type = self.type_id(&bits_ty)?;
        let bits = self.fresh();
        instructions.push(Self::inst(
            Op::Bitcast,
            Some(bits_type),
            Some(bits),
            vec![Operand::IdRef(vector)],
        ));
        Ok((bits_ty, bits))
    }

    pub(in crate::native::emitter) fn emit_vector_word_load(
        &mut self,
        result: Word,
        result_ty: &LlType,
        pointer: &VectorWordPointer,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        if self.resolve_type(result_ty)? != LlType::Int(32) {
            return Err(format!(
                "native emitter: vector word load result must be i32, got {result_ty:?}"
            ));
        }
        let (ptr, lane_base) = self.emit_vector_word_element_pointer(pointer, instructions)?;
        let vector_ty = self.type_id(&pointer.vector_ty)?;
        let vector = self.fresh();
        instructions.push(Self::inst(
            Op::Load,
            Some(vector_ty),
            Some(vector),
            vec![Operand::IdRef(ptr)],
        ));
        let (_bits_ty, bits) = self.emit_vector_word_bits_vector(pointer, vector, instructions)?;
        let u16_ty = self.type_id(&LlType::Int(16))?;
        let low16 = self.fresh();
        instructions.push(Self::inst(
            Op::VectorExtractDynamic,
            Some(u16_ty),
            Some(low16),
            vec![Operand::IdRef(bits), Operand::IdRef(lane_base)],
        ));
        let high_lane = self.emit_vector_word_high_lane(lane_base, instructions)?;
        let high16 = self.fresh();
        instructions.push(Self::inst(
            Op::VectorExtractDynamic,
            Some(u16_ty),
            Some(high16),
            vec![Operand::IdRef(bits), Operand::IdRef(high_lane)],
        ));
        let uint_ty = self.type_id(&LlType::Int(32))?;
        let low32 = self.fresh();
        instructions.push(Self::inst(
            Op::UConvert,
            Some(uint_ty),
            Some(low32),
            vec![Operand::IdRef(low16)],
        ));
        let high32 = self.fresh();
        instructions.push(Self::inst(
            Op::UConvert,
            Some(uint_ty),
            Some(high32),
            vec![Operand::IdRef(high16)],
        ));
        let shift = self.const_uint(16)?;
        let shifted_high = self.fresh();
        instructions.push(Self::inst(
            Op::ShiftLeftLogical,
            Some(uint_ty),
            Some(shifted_high),
            vec![Operand::IdRef(high32), Operand::IdRef(shift)],
        ));
        instructions.push(Self::inst(
            Op::BitwiseOr,
            Some(uint_ty),
            Some(result),
            vec![Operand::IdRef(low32), Operand::IdRef(shifted_high)],
        ));
        Ok(())
    }

    pub(in crate::native::emitter) fn emit_vector_word_store(
        &mut self,
        pointer: &VectorWordPointer,
        object_ty: &LlType,
        value: Word,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        if self.resolve_type(object_ty)? != LlType::Int(32) {
            return Err(format!(
                "native emitter: vector word store value must be i32, got {object_ty:?}"
            ));
        }
        let (ptr, lane_base) = self.emit_vector_word_element_pointer(pointer, instructions)?;
        let vector_ty = self.type_id(&pointer.vector_ty)?;
        let u16_ty = self.type_id(&LlType::Int(16))?;
        let low16 = self.fresh();
        instructions.push(Self::inst(
            Op::UConvert,
            Some(u16_ty),
            Some(low16),
            vec![Operand::IdRef(value)],
        ));
        let uint_ty = self.type_id(&LlType::Int(32))?;
        let shift = self.const_uint(16)?;
        let shifted = self.fresh();
        instructions.push(Self::inst(
            Op::ShiftRightLogical,
            Some(uint_ty),
            Some(shifted),
            vec![Operand::IdRef(value), Operand::IdRef(shift)],
        ));
        let high16 = self.fresh();
        instructions.push(Self::inst(
            Op::UConvert,
            Some(u16_ty),
            Some(high16),
            vec![Operand::IdRef(shifted)],
        ));
        if matches!(
            pointer.storage,
            StorageClass::Workgroup | StorageClass::Function | StorageClass::Private
        ) {
            let elem_ty = match self.resolve_type(&pointer.vector_ty)? {
                LlType::Vector(elem, _) => self.resolve_type(&elem)?,
                other => {
                    return Err(format!(
                        "native emitter: vector word store target is not a vector: {other:?}"
                    ))
                }
            };
            let [low_stored, high_stored] = if types_compatible(&elem_ty, &LlType::Int(16)) {
                [low16, high16]
            } else {
                let elem_type = self.type_id(&elem_ty)?;
                let low_cast = self.fresh();
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(elem_type),
                    Some(low_cast),
                    vec![Operand::IdRef(low16)],
                ));
                let high_cast = self.fresh();
                instructions.push(Self::inst(
                    Op::Bitcast,
                    Some(elem_type),
                    Some(high_cast),
                    vec![Operand::IdRef(high16)],
                ));
                [low_cast, high_cast]
            };
            let high_lane = self.emit_vector_word_high_lane(lane_base, instructions)?;
            let elem_ptr_ty = self.ptr_type_id(pointer.storage, &elem_ty)?;
            for (scalar, lane) in [(low_stored, lane_base), (high_stored, high_lane)] {
                let elem_ptr = self.fresh();
                instructions.push(Self::inst(
                    Op::InBoundsAccessChain,
                    Some(elem_ptr_ty),
                    Some(elem_ptr),
                    vec![Operand::IdRef(ptr), Operand::IdRef(lane)],
                ));
                instructions.push(Self::inst(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(elem_ptr), Operand::IdRef(scalar)],
                ));
            }
            return Ok(());
        }
        let vector = self.fresh();
        instructions.push(Self::inst(
            Op::Load,
            Some(vector_ty),
            Some(vector),
            vec![Operand::IdRef(ptr)],
        ));
        let (bits_ty, bits) = self.emit_vector_word_bits_vector(pointer, vector, instructions)?;
        let inserted_low = self.fresh();
        let bits_type = self.type_id(&bits_ty)?;
        instructions.push(Self::inst(
            Op::VectorInsertDynamic,
            Some(bits_type),
            Some(inserted_low),
            vec![
                Operand::IdRef(bits),
                Operand::IdRef(low16),
                Operand::IdRef(lane_base),
            ],
        ));
        let high_lane = self.emit_vector_word_high_lane(lane_base, instructions)?;
        let inserted_high = self.fresh();
        instructions.push(Self::inst(
            Op::VectorInsertDynamic,
            Some(bits_type),
            Some(inserted_high),
            vec![
                Operand::IdRef(inserted_low),
                Operand::IdRef(high16),
                Operand::IdRef(high_lane),
            ],
        ));
        let stored = if types_compatible(&pointer.vector_ty, &bits_ty) {
            inserted_high
        } else {
            let stored = self.fresh();
            instructions.push(Self::inst(
                Op::Bitcast,
                Some(vector_ty),
                Some(stored),
                vec![Operand::IdRef(inserted_high)],
            ));
            stored
        };
        instructions.push(Self::inst(
            Op::Store,
            None,
            None,
            vec![Operand::IdRef(ptr), Operand::IdRef(stored)],
        ));
        Ok(())
    }
}
