use super::*;

impl Ctx {
    pub(super) fn ty_fn_void(&mut self, void: Word) -> Word {
        if let Some(&id) = self.singleton_types.get(&SingletonType::FnVoid) {
            return id;
        }
        for inst in &self.module.types_global_values {
            if inst.class.opcode == Op::TypeFunction
                && inst.operands.len() == 1
                && inst.operands[0] == Operand::IdRef(void)
            {
                if let Some(rid) = inst.result_id {
                    self.singleton_types.insert(SingletonType::FnVoid, rid);
                    return rid;
                }
            }
        }
        let id = self.module.fresh_id();
        self.new_globals
            .push(type_inst(Op::TypeFunction, id, vec![Operand::IdRef(void)]));
        self.singleton_types.insert(SingletonType::FnVoid, id);
        id
    }

    pub(in crate::passes) fn ty_int8(&mut self) -> Word {
        if let Some(&id) = self.singleton_types.get(&SingletonType::Int8) {
            return id;
        }
        for inst in &self.module.types_global_values {
            if inst.class.opcode == Op::TypeInt
                && inst.operands.first() == Some(&Operand::LiteralBit32(8))
            {
                if let Some(rid) = inst.result_id {
                    self.singleton_types.insert(SingletonType::Int8, rid);
                    return rid;
                }
            }
        }
        let id = self.module.fresh_id();
        self.new_globals.push(type_inst(
            Op::TypeInt,
            id,
            vec![Operand::LiteralBit32(8), Operand::LiteralBit32(0)],
        ));
        self.singleton_types.insert(SingletonType::Int8, id);
        id
    }

    pub(in crate::passes) fn ty_int16(&mut self) -> Word {
        if let Some(&id) = self.singleton_types.get(&SingletonType::Int16) {
            return id;
        }
        for inst in &self.module.types_global_values {
            if inst.class.opcode == Op::TypeInt
                && inst.operands.first() == Some(&Operand::LiteralBit32(16))
                && inst.operands.get(1) == Some(&Operand::LiteralBit32(0))
            {
                if let Some(rid) = inst.result_id {
                    self.singleton_types.insert(SingletonType::Int16, rid);
                    return rid;
                }
            }
        }
        let id = self.module.fresh_id();
        self.new_globals.push(type_inst(
            Op::TypeInt,
            id,
            vec![Operand::LiteralBit32(16), Operand::LiteralBit32(0)],
        ));
        self.singleton_types.insert(SingletonType::Int16, id);
        id
    }
}
