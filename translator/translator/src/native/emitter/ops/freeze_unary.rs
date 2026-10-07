use super::*;

impl Emitter {
    pub(in crate::native::emitter) fn emit_freeze_resolved(
        &mut self,
        value: TypedValue,
        name: String,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let result_ty = self.resolve_type(&value.ty)?;
        let pointer_meta = match result_ty {
            LlType::Ptr(addrspace) => self.pointer_meta_for_value(&value.value, addrspace)?,
            _ => None,
        };
        let result_type = self.pointer_aware_type_id(&result_ty, pointer_meta.as_ref())?;
        let result = self.result_id(&name, &result_ty)?;
        let value_id = self.value_id_in(&value.value, &value.ty, instructions)?;
        instructions.push(Self::inst(
            Op::CopyObject,
            Some(result_type),
            Some(result),
            vec![Operand::IdRef(value_id)],
        ));
        self.record_freeze_metadata(&name, &value.value, &result_ty, pointer_meta)?;
        Ok(())
    }

    pub(in crate::native::emitter) fn record_freeze_metadata(
        &mut self,
        name: &str,
        value: &LlValue,
        result_ty: &LlType,
        pointer_meta: Option<PointerMeta>,
    ) -> Result<(), String> {
        if let Some(meta) = pointer_meta {
            self.record_pointer_meta(name.to_string(), meta);
        }
        if matches!(result_ty, LlType::Int(_)) {
            self.record_int_alignment(name, result_ty, self.int_value_alignment(value));
        }
        let LlValue::Local(src_name) = value else {
            return Ok(());
        };
        if let Some(is_null) = self.pointer_nullness.get(src_name).copied() {
            self.record_pointer_nullness(name.to_string(), is_null);
        }
        if let Some(raw) = self.raw_offsets.get(src_name).cloned() {
            self.raw_offsets.insert(name.to_string(), raw);
        }
        if let Some(provenance) = self.gep_provenance.get(src_name).cloned() {
            self.gep_provenance.insert(name.to_string(), provenance);
        }
        if let Some(selected) = self.selected_pointers.get(src_name).cloned() {
            self.selected_pointers.insert(name.to_string(), selected);
        }
        if let Some(tree) = self.selected_access_trees.get(src_name).cloned() {
            self.selected_access_trees.insert(name.to_string(), tree);
        }
        if let Some(selected) = self.selected_load_pointers.get(src_name).cloned() {
            self.selected_load_pointers
                .insert(name.to_string(), selected);
        }
        if self.unmodeled_pointers.contains(src_name) {
            self.unmodeled_pointers.insert(name.to_string());
        }
        if self.byte_view_pointers.contains(src_name) {
            self.byte_view_pointers.insert(name.to_string());
        }
        if self.param_values.contains(src_name) {
            self.param_values.insert(name.to_string());
        }
        Ok(())
    }

    pub(in crate::native::emitter) fn emit_unary_float_op_resolved(
        &mut self,
        op: Op,
        value: TypedValue,
        name: String,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), String> {
        let result_ty = self.resolve_type(&value.ty)?;
        if let Some(n) = bfloat_lanes(&result_ty) {
            let result = self.result_id(&name, &result_ty)?;
            let value_id = self.value_id_in(&value.value, &value.ty, instructions)?;
            let value_f32 = self.bfloat_bits_to_float_shaped_id(value_id, n, instructions)?;
            let float_ty = self.type_id(&shaped_type(LlType::Float, n))?;
            let result_f32 = self.fresh();
            instructions.push(Self::inst(
                op,
                Some(float_ty),
                Some(result_f32),
                vec![Operand::IdRef(value_f32)],
            ));
            return self.emit_float_to_bfloat_bits_shaped(result_f32, result, n, instructions);
        }
        if !is_float_type(&result_ty) {
            return Err(format!(
                "native emitter: {op:?} currently supports float/half types, got {result_ty:?}"
            ));
        }
        let result_type = self.type_id(&result_ty)?;
        let result = self.result_id(&name, &result_ty)?;
        let value_id = self.value_id_in(&value.value, &value.ty, instructions)?;
        instructions.push(Self::inst(
            op,
            Some(result_type),
            Some(result),
            vec![Operand::IdRef(value_id)],
        ));
        Ok(())
    }
}
