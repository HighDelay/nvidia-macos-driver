use super::*;

impl Emitter {
    pub(in crate::native::emitter) fn tir_inst_typed_operands(
        &self,
        inst: &crate::native::tir::TirInst,
    ) -> Option<Vec<TypedValue>> {
        inst.operands
            .iter()
            .map(crate::native::tir::TirOperand::as_typed_value)
            .collect()
    }

    pub(in crate::native::emitter) fn overlay_call_args(
        args: &mut [TypedValue],
        operands: &[TypedValue],
    ) -> bool {
        if operands.len() != args.len() {
            return false;
        }
        for (arg, op) in args.iter_mut().zip(operands) {
            *arg = op.clone();
        }
        true
    }

    pub(in crate::native::emitter) fn apply_tir_inst_call_args(
        &self,
        inst: &crate::native::tir::TirInst,
        diagnostic_name: &str,
        call: &mut LlCall,
    ) {
        match self.tir_inst_typed_operands(inst) {
            Some(ops) if Self::overlay_call_args(&mut call.args, &ops) => {}
            _ => Self::tir_only_gate(diagnostic_name, "call"),
        }
    }

    pub(in crate::native::emitter) fn tir_only_gate(name: &str, opcode_class: &str) {
        if crate::env_vars::tir_only() {
            panic!("METAL2VULKAN_TIR_ONLY: {opcode_class} {name} fell back to string parse");
        }
    }

    pub(in crate::native::emitter) fn convert_dst_type(
        &self,
        name: &str,
        dst_text: &str,
    ) -> Result<LlType, String> {
        if let Some(ty) = self.tir_result_types.get(name) {
            return self.resolve_type(ty);
        }
        self.resolve_type(&parse_type(dst_text)?)
    }
}
