use crate::spirv_module::{Block, Function, Instruction, Module, ModuleHeader, Operand};
use spirv::{AddressingModel, MemoryModel, Op, StorageClass, Word};
use std::collections::BTreeMap;

struct PendingBlock {
    label: Word,
    name: String,
    instructions: Vec<Instruction>,
}

pub(in crate::native) struct CfgBuilder {
    module: Module,
    next_id: Word,
    uint: Word,
    bool_ty: Word,
    parameters: Vec<Word>,
    constants: BTreeMap<u32, Word>,
    labels: BTreeMap<String, Word>,
    blocks: Vec<PendingBlock>,
    current: Option<usize>,
}

impl CfgBuilder {
    pub(in crate::native) fn new(parameters: usize) -> Self {
        Self::build(parameters, false)
    }

    pub(in crate::native) fn new_entry_point() -> Self {
        Self::build(0, true)
    }

    fn build(parameters: usize, entry_point: bool) -> Self {
        let mut next_id = 1;
        let mut fresh = || {
            let id = next_id;
            next_id += 1;
            id
        };
        let void = fresh();
        let uint = fresh();
        let bool_ty = fresh();
        let fn_ty = fresh();
        let parameter_ids = (0..parameters).map(|_| fresh()).collect::<Vec<_>>();
        let function_id = fresh();

        let mut module = Module::new();
        module.memory_model = Some(Instruction::new(
            Op::MemoryModel,
            None,
            None,
            vec![
                Operand::AddressingModel(AddressingModel::Logical),
                Operand::MemoryModel(MemoryModel::GLSL450),
            ],
        ));
        module.types_global_values = vec![
            Instruction::new(Op::TypeVoid, None, Some(void), vec![]),
            Instruction::new(
                Op::TypeInt,
                None,
                Some(uint),
                vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
            ),
            Instruction::new(Op::TypeBool, None, Some(bool_ty), vec![]),
            Instruction::new(
                Op::TypeFunction,
                None,
                Some(fn_ty),
                std::iter::once(Operand::IdRef(if entry_point { void } else { uint }))
                    .chain(std::iter::repeat_n(
                        Operand::IdRef(uint),
                        parameter_ids.len(),
                    ))
                    .collect(),
            ),
        ];
        if entry_point {
            module.capabilities.push(Instruction::new(
                Op::Capability,
                None,
                None,
                vec![Operand::Capability(spirv::Capability::Shader)],
            ));
            module.entry_points.push(Instruction::new(
                Op::EntryPoint,
                None,
                None,
                vec![
                    Operand::ExecutionModel(spirv::ExecutionModel::GLCompute),
                    Operand::IdRef(function_id),
                    Operand::LiteralString("main".to_string()),
                ],
            ));
            module.execution_modes.push(Instruction::new(
                Op::ExecutionMode,
                None,
                None,
                vec![
                    Operand::IdRef(function_id),
                    Operand::ExecutionMode(spirv::ExecutionMode::LocalSize),
                    Operand::LiteralBit32(1),
                    Operand::LiteralBit32(1),
                    Operand::LiteralBit32(1),
                ],
            ));
        }

        let mut function = Function::new();
        function.def = Some(Instruction::new(
            Op::Function,
            Some(if entry_point { void } else { uint }),
            Some(function_id),
            vec![
                Operand::FunctionControl(spirv::FunctionControl::NONE),
                Operand::IdRef(fn_ty),
            ],
        ));
        function.parameters = parameter_ids
            .iter()
            .map(|id| Instruction::new(Op::FunctionParameter, Some(uint), Some(*id), vec![]))
            .collect();
        function.end = Some(Instruction::new(Op::FunctionEnd, None, None, vec![]));
        module.functions.push(function);

        Self {
            module,
            next_id,
            uint,
            bool_ty,
            parameters: parameter_ids,
            constants: BTreeMap::new(),
            labels: BTreeMap::new(),
            blocks: Vec::new(),
            current: None,
        }
    }

    pub(in crate::native) fn private_global(&mut self, value: u32) -> Word {
        let initializer = self.constant(value);
        let pointer = self.pointer_to_uint(StorageClass::Private);
        let id = self.fresh();
        self.module.types_global_values.push(Instruction::new(
            Op::Variable,
            Some(pointer),
            Some(id),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(initializer),
            ],
        ));
        for entry_point in &mut self.module.entry_points {
            entry_point.operands.push(Operand::IdRef(id));
        }
        id
    }

    pub(in crate::native) fn load(&mut self, pointer: Word) -> Word {
        let id = self.fresh();
        self.push(Instruction::new(
            Op::Load,
            Some(self.uint),
            Some(id),
            vec![Operand::IdRef(pointer)],
        ));
        id
    }

    fn pointer_to_uint(&mut self, storage: StorageClass) -> Word {
        let existing = self.module.types_global_values.iter().find(|instruction| {
            instruction.class.opcode == Op::TypePointer
                && instruction.operands.first() == Some(&Operand::StorageClass(storage))
                && instruction.operands.get(1) == Some(&Operand::IdRef(self.uint))
        });
        if let Some(id) = existing.and_then(|instruction| instruction.result_id) {
            return id;
        }
        let id = self.fresh();
        let uint = self.uint;
        self.module.types_global_values.push(Instruction::new(
            Op::TypePointer,
            None,
            Some(id),
            vec![Operand::StorageClass(storage), Operand::IdRef(uint)],
        ));
        id
    }

    pub(in crate::native) fn parameter(&self, index: usize) -> Word {
        self.parameters[index]
    }

    pub(in crate::native) fn constant(&mut self, value: u32) -> Word {
        if let Some(id) = self.constants.get(&value) {
            return *id;
        }
        let id = self.fresh();
        self.module.types_global_values.push(Instruction::new(
            Op::Constant,
            Some(self.uint),
            Some(id),
            vec![Operand::LiteralBit32(value)],
        ));
        self.constants.insert(value, id);
        id
    }

    pub(in crate::native) fn label(&mut self, name: &str) -> Word {
        if let Some(id) = self.labels.get(name) {
            return *id;
        }
        let id = self.fresh();
        self.labels.insert(name.to_string(), id);
        id
    }

    pub(in crate::native) fn block(&mut self, name: &str) -> &mut Self {
        let label = self.label(name);
        assert!(
            !self.blocks.iter().any(|block| block.label == label),
            "block {name} opened twice"
        );
        self.blocks.push(PendingBlock {
            label,
            name: name.to_string(),
            instructions: Vec::new(),
        });
        self.current = Some(self.blocks.len() - 1);
        self
    }

    fn fresh(&mut self) -> Word {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn push(&mut self, instruction: Instruction) {
        let index = self.current.expect("no block is open");
        self.blocks[index].instructions.push(instruction);
    }

    pub(in crate::native) fn reserve_value(&mut self) -> Word {
        self.fresh()
    }

    fn binary_into(&mut self, opcode: Op, result_type: Word, id: Word, a: Word, b: Word) {
        self.push(Instruction::new(
            opcode,
            Some(result_type),
            Some(id),
            vec![Operand::IdRef(a), Operand::IdRef(b)],
        ));
    }

    fn binary(&mut self, opcode: Op, result_type: Word, a: Word, b: Word) -> Word {
        let id = self.fresh();
        self.binary_into(opcode, result_type, id, a, b);
        id
    }

    pub(in crate::native) fn add(&mut self, a: Word, b: Word) -> Word {
        self.binary(Op::IAdd, self.uint, a, b)
    }

    pub(in crate::native) fn add_into(&mut self, result: Word, a: Word, b: Word) {
        self.binary_into(Op::IAdd, self.uint, result, a, b);
    }

    pub(in crate::native) fn bitwise_xor(&mut self, a: Word, b: Word) -> Word {
        self.binary(Op::BitwiseXor, self.uint, a, b)
    }

    pub(in crate::native) fn bitwise_xor_into(&mut self, result: Word, a: Word, b: Word) {
        self.binary_into(Op::BitwiseXor, self.uint, result, a, b);
    }

    pub(in crate::native) fn bitwise_and(&mut self, a: Word, b: Word) -> Word {
        self.binary(Op::BitwiseAnd, self.uint, a, b)
    }

    pub(in crate::native) fn less_than(&mut self, a: Word, b: Word) -> Word {
        self.binary(Op::ULessThan, self.bool_ty, a, b)
    }

    pub(in crate::native) fn equal(&mut self, a: Word, b: Word) -> Word {
        self.binary(Op::IEqual, self.bool_ty, a, b)
    }

    pub(in crate::native) fn phi(&mut self, incoming: &[(Word, &str)]) -> Word {
        let operands = incoming
            .iter()
            .flat_map(|(value, predecessor)| {
                let label = self.label(predecessor);
                [Operand::IdRef(*value), Operand::IdRef(label)]
            })
            .collect::<Vec<_>>();
        let id = self.fresh();
        self.push(Instruction::new(
            Op::Phi,
            Some(self.uint),
            Some(id),
            operands,
        ));
        id
    }

    pub(in crate::native) fn branch(&mut self, target: &str) {
        let label = self.label(target);
        self.push(Instruction::new(
            Op::Branch,
            None,
            None,
            vec![Operand::IdRef(label)],
        ));
    }

    pub(in crate::native) fn selection_merge(&mut self, merge: &str) {
        let merge = self.label(merge);
        self.push(Instruction::new(
            Op::SelectionMerge,
            None,
            None,
            vec![
                Operand::IdRef(merge),
                Operand::SelectionControl(spirv::SelectionControl::NONE),
            ],
        ));
    }

    pub(in crate::native) fn branch_conditional(
        &mut self,
        condition: Word,
        on_true: &str,
        on_false: &str,
    ) {
        let on_true = self.label(on_true);
        let on_false = self.label(on_false);
        self.push(Instruction::new(
            Op::BranchConditional,
            None,
            None,
            vec![
                Operand::IdRef(condition),
                Operand::IdRef(on_true),
                Operand::IdRef(on_false),
            ],
        ));
    }

    pub(in crate::native) fn switch(
        &mut self,
        selector: Word,
        default: &str,
        cases: &[(u32, &str)],
    ) {
        let default = self.label(default);
        let mut operands = vec![Operand::IdRef(selector), Operand::IdRef(default)];
        for (literal, target) in cases {
            let label = self.label(target);
            operands.push(Operand::LiteralBit32(*literal));
            operands.push(Operand::IdRef(label));
        }
        self.push(Instruction::new(Op::Switch, None, None, operands));
    }

    pub(in crate::native) fn store(&mut self, pointer: Word, value: Word) {
        self.push(Instruction::new(
            Op::Store,
            None,
            None,
            vec![Operand::IdRef(pointer), Operand::IdRef(value)],
        ));
    }

    pub(in crate::native) fn return_void(&mut self) {
        self.push(Instruction::new(Op::Return, None, None, vec![]));
    }

    pub(in crate::native) fn return_value(&mut self, value: Word) {
        self.push(Instruction::new(
            Op::ReturnValue,
            None,
            None,
            vec![Operand::IdRef(value)],
        ));
    }

    pub(in crate::native) fn finish(self) -> Module {
        let Self {
            mut module,
            next_id,
            labels,
            blocks,
            ..
        } = self;
        for (name, label) in &labels {
            assert!(
                blocks.iter().any(|block| block.label == *label),
                "block {name} was branched to but never opened"
            );
        }
        for block in &blocks {
            assert!(
                block
                    .instructions
                    .last()
                    .is_some_and(|instruction| is_terminator(instruction.class.opcode)),
                "block {} has no terminator",
                block.name
            );
        }
        let order = reverse_postorder(&blocks);
        assert_eq!(
            order.len(),
            blocks.len(),
            "some authored block is unreachable from the entry"
        );
        let mut by_index = blocks.into_iter().map(Some).collect::<Vec<_>>();
        let mut emitted = Vec::with_capacity(order.len());
        for index in order {
            let block = by_index[index].take().expect("each block is emitted once");
            emitted.push(Block {
                label: Some(Instruction::new(Op::Label, None, Some(block.label), vec![])),
                instructions: block.instructions,
            });
        }
        module.functions[0].blocks = emitted;
        let mut header = ModuleHeader::new(next_id);
        header.set_version(1, 5);
        module.header = Some(header);
        module
    }
}

fn reverse_postorder(blocks: &[PendingBlock]) -> Vec<usize> {
    let by_label = blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.label, index))
        .collect::<BTreeMap<_, _>>();
    let mut visited = vec![false; blocks.len()];
    let mut postorder = Vec::with_capacity(blocks.len());
    let mut stack = vec![(0usize, 0usize)];
    visited[0] = true;
    while let Some((index, next)) = stack.pop() {
        let successors = successors(&blocks[index]);
        match successors.get(next) {
            Some(label) => {
                stack.push((index, next + 1));
                let target = *by_label
                    .get(label)
                    .unwrap_or_else(|| panic!("branch to %{label}, which is not an opened block"));
                if !visited[target] {
                    visited[target] = true;
                    stack.push((target, 0));
                }
            }
            None => postorder.push(index),
        }
    }
    postorder.reverse();
    postorder
}

fn successors(block: &PendingBlock) -> Vec<Word> {
    let Some(terminator) = block.instructions.last() else {
        return Vec::new();
    };
    let labels = match terminator.class.opcode {
        Op::Branch => &terminator.operands[..],
        Op::BranchConditional => &terminator.operands[1..3.min(terminator.operands.len())],
        Op::Switch => &terminator.operands[1..],
        _ => return Vec::new(),
    };
    labels
        .iter()
        .filter_map(|operand| match operand {
            Operand::IdRef(id) => Some(*id),
            _ => None,
        })
        .collect()
}

fn is_terminator(opcode: Op) -> bool {
    matches!(
        opcode,
        Op::Branch
            | Op::BranchConditional
            | Op::Switch
            | Op::Return
            | Op::ReturnValue
            | Op::Unreachable
            | Op::Kill
            | Op::TerminateInvocation
    )
}
