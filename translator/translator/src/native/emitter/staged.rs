use crate::spirv_module::Instruction;

pub(in crate::native::emitter) fn staged_emit(
    instructions: &mut Vec<Instruction>,
    emit: impl FnOnce(&mut Vec<Instruction>) -> Result<bool, String>,
) -> Result<bool, String> {
    let mut staged = Vec::new();
    if emit(&mut staged)? {
        instructions.append(&mut staged);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spirv_module::Operand;
    use spirv::Op;

    fn nop() -> Instruction {
        Instruction::new(Op::Nop, None, None, vec![Operand::LiteralBit32(0)])
    }

    #[test]
    fn a_declined_step_leaves_the_stream_it_was_given() {
        let mut instructions = vec![nop()];
        let declined = staged_emit(&mut instructions, |staged| {
            staged.push(nop());
            staged.push(nop());
            Ok(false)
        });
        assert_eq!(declined, Ok(false));
        assert_eq!(instructions.len(), 1, "a decline must emit nothing");
    }

    #[test]
    fn a_handled_step_appends_in_order() {
        let mut instructions = vec![nop()];
        let handled = staged_emit(&mut instructions, |staged| {
            staged.push(nop());
            staged.push(nop());
            Ok(true)
        });
        assert_eq!(handled, Ok(true));
        assert_eq!(instructions.len(), 3);
    }

    #[test]
    fn an_erroring_step_reports_the_error() {
        let mut instructions = Vec::new();
        let failed = staged_emit(&mut instructions, |staged| {
            staged.push(nop());
            Err("no".to_string())
        });
        assert_eq!(failed, Err("no".to_string()));
    }
}
