use super::*;

mod access_provenance;
pub(in crate::passes) use access_provenance::*;
mod private_memory;
pub(in crate::passes) use private_memory::*;
mod access_chain;
pub(in crate::passes) use access_chain::*;
mod index_remap;
pub(in crate::passes) use index_remap::*;
mod vector_subword;
pub(in crate::passes) use vector_subword::*;
mod workgroup;
pub(in crate::passes) use workgroup::*;
mod byte_aggregate;
pub(in crate::passes) use byte_aggregate::*;
mod dynamic_reinterpret;
pub(in crate::passes) use dynamic_reinterpret::*;
mod raw_byte;
pub(in crate::passes) use raw_byte::*;

#[cfg(test)]
mod byte_reinterpret_tests {
    use super::*;
    use crate::spirv_module::{Block, Function};

    fn install_entry(ctx: &mut Ctx, mut body: Vec<Instruction>) {
        body.push(Instruction::new(Op::Return, None, None, vec![]));
        let label = ctx.module.fresh_id();
        let func_id = ctx.module.fresh_id();
        ctx.module.functions.push(Function {
            def: Some(Instruction::new(Op::Function, None, Some(func_id), vec![])),
            end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
            parameters: vec![],
            blocks: vec![Block {
                label: Some(Instruction::new(Op::Label, None, Some(label), vec![])),
                instructions: body,
            }],
        });
    }

    fn storage_buffer_var(ctx: &mut Ctx, ptr_ty: Word) -> Word {
        let id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::Variable,
            Some(ptr_ty),
            Some(id),
            vec![Operand::StorageClass(StorageClass::StorageBuffer)],
        ));
        id
    }

    fn decorate_array_stride(ctx: &mut Ctx, array: Word, stride: u32) {
        ctx.module.annotations.push(Instruction::new(
            Op::Decorate,
            None,
            None,
            vec![
                Operand::IdRef(array),
                Operand::Decoration(Decoration::ArrayStride),
                Operand::LiteralBit32(stride),
            ],
        ));
    }

    fn only_inst(ctx: &Ctx) -> &Instruction {
        &ctx.module.functions[0].blocks[0].instructions[0]
    }

    #[test]
    fn strided_descent_flips_overindexing_chain_to_ptr_access_chain() {
        let mut ctx = Ctx::new(Module::new());
        let float = ctx.ty_float();
        let rt = ctx.ty_runtime_array(float);
        let ptr_rt = ctx.ty_ptr(StorageClass::StorageBuffer, rt);
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let base = storage_buffer_var(&mut ctx, ptr_rt);
        let i = ctx.const_uint(0);
        let j = ctx.const_uint(1);
        let chain_id = ctx.module.fresh_id();
        let ops = vec![Operand::IdRef(base), Operand::IdRef(i), Operand::IdRef(j)];
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_float),
                Some(chain_id),
                ops.clone(),
            )],
        );

        rewrite_strided_descent_access_chains(&mut ctx, 0);

        let inst = only_inst(&ctx);
        assert_eq!(inst.class.opcode, Op::PtrAccessChain, "opcode flipped");
        assert_eq!(inst.result_type, Some(ptr_float), "result type preserved");
        assert_eq!(inst.result_id, Some(chain_id), "result id preserved");
        assert_eq!(
            inst.operands, ops,
            "operands (base + both indices) unchanged"
        );
    }

    #[test]
    fn strided_descent_is_idempotent() {
        let mut ctx = Ctx::new(Module::new());
        let float = ctx.ty_float();
        let rt = ctx.ty_runtime_array(float);
        let ptr_rt = ctx.ty_ptr(StorageClass::StorageBuffer, rt);
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let base = storage_buffer_var(&mut ctx, ptr_rt);
        let i = ctx.const_uint(0);
        let j = ctx.const_uint(1);
        let chain_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_float),
                Some(chain_id),
                vec![Operand::IdRef(base), Operand::IdRef(i), Operand::IdRef(j)],
            )],
        );

        rewrite_strided_descent_access_chains(&mut ctx, 0);
        let after_first = ctx.module.functions[0].blocks[0].instructions.clone();
        rewrite_strided_descent_access_chains(&mut ctx, 0);
        let after_second = &ctx.module.functions[0].blocks[0].instructions;
        assert_eq!(
            &after_first, after_second,
            "second application leaves the block byte-identical"
        );
    }

    #[test]
    fn strided_descent_leaves_valid_chain_untouched() {
        let mut ctx = Ctx::new(Module::new());
        let float = ctx.ty_float();
        let inner = ctx.ty_runtime_array(float);
        let outer = ctx.ty_runtime_array(inner);
        let ptr_outer = ctx.ty_ptr(StorageClass::StorageBuffer, outer);
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let base = storage_buffer_var(&mut ctx, ptr_outer);
        let i = ctx.const_uint(0);
        let j = ctx.const_uint(1);
        let chain_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_float),
                Some(chain_id),
                vec![Operand::IdRef(base), Operand::IdRef(i), Operand::IdRef(j)],
            )],
        );

        rewrite_strided_descent_access_chains(&mut ctx, 0);

        assert_eq!(
            only_inst(&ctx).class.opcode,
            Op::InBoundsAccessChain,
            "a cleanly-walking chain is not rewritten"
        );
    }

    #[test]
    fn workgroup_singleton_array_member_index_is_restored() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let len = ctx.const_uint(256);
        let array = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeArray,
            None,
            Some(array),
            vec![Operand::IdRef(uint), Operand::IdRef(len)],
        ));
        let aggregate = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(aggregate),
            vec![Operand::IdRef(array)],
        ));
        let ptr_array = ctx.ty_ptr(StorageClass::Workgroup, aggregate);
        let ptr_uint = ctx.ty_ptr(StorageClass::Workgroup, uint);
        let base = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::Variable,
            Some(ptr_array),
            Some(base),
            vec![Operand::StorageClass(StorageClass::Workgroup)],
        ));
        let stride_index = ctx.const_uint(3);
        let member_zero = ctx.const_uint(0);
        let result = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::PtrAccessChain,
                Some(ptr_uint),
                Some(result),
                vec![
                    Operand::IdRef(base),
                    Operand::IdRef(stride_index),
                    Operand::IdRef(member_zero),
                ],
            )],
        );

        split_workgroup_ptr_access_chain_descent(&mut ctx, 0);

        let instructions = &ctx.module.functions[0].blocks[0].instructions;
        assert_eq!(instructions.len(), 2);
        assert_eq!(instructions[0].class.opcode, Op::InBoundsAccessChain);
        assert_eq!(instructions[0].result_type, Some(ptr_uint));
        assert_eq!(instructions[0].result_id, Some(result));
        assert_eq!(
            instructions[0].operands,
            vec![
                Operand::IdRef(base),
                Operand::IdRef(member_zero),
                Operand::IdRef(stride_index),
            ]
        );
    }

    #[test]
    fn workgroup_array_stride_and_descent_are_split() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let len = ctx.const_uint(256);
        let array = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeArray,
            None,
            Some(array),
            vec![Operand::IdRef(uint), Operand::IdRef(len)],
        ));
        let ptr_array = ctx.ty_ptr(StorageClass::Workgroup, array);
        let ptr_uint = ctx.ty_ptr(StorageClass::Workgroup, uint);
        let base = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::Variable,
            Some(ptr_array),
            Some(base),
            vec![Operand::StorageClass(StorageClass::Workgroup)],
        ));
        let stride_index = ctx.const_uint(3);
        let element_index = ctx.const_uint(1);
        let result = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::PtrAccessChain,
                Some(ptr_uint),
                Some(result),
                vec![
                    Operand::IdRef(base),
                    Operand::IdRef(stride_index),
                    Operand::IdRef(element_index),
                ],
            )],
        );

        split_workgroup_ptr_access_chain_descent(&mut ctx, 0);

        let instructions = &ctx.module.functions[0].blocks[0].instructions;
        assert_eq!(instructions[0].class.opcode, Op::PtrAccessChain);
        assert_eq!(instructions[0].result_type, Some(ptr_array));
        assert_eq!(instructions[0].operands.len(), 2);
        let strided = instructions[0]
            .result_id
            .expect("strided aggregate pointer");
        assert_eq!(instructions[1].class.opcode, Op::InBoundsAccessChain);
        assert_eq!(instructions[1].result_type, Some(ptr_uint));
        assert_eq!(instructions[1].result_id, Some(result));
        assert_eq!(
            instructions[1].operands,
            vec![Operand::IdRef(strided), Operand::IdRef(element_index)]
        );
    }

    #[test]
    fn storage_buffer_strided_descent_keeps_combined_form() {
        let mut ctx = Ctx::new(Module::new());
        let float = ctx.ty_float();
        let array = ctx.ty_runtime_array(float);
        let ptr_array = ctx.ty_ptr(StorageClass::StorageBuffer, array);
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let base = storage_buffer_var(&mut ctx, ptr_array);
        let stride_index = ctx.const_uint(2);
        let element_index = ctx.const_uint(1);
        let result = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::PtrAccessChain,
                Some(ptr_float),
                Some(result),
                vec![
                    Operand::IdRef(base),
                    Operand::IdRef(stride_index),
                    Operand::IdRef(element_index),
                ],
            )],
        );

        split_workgroup_ptr_access_chain_descent(&mut ctx, 0);

        let instructions = &ctx.module.functions[0].blocks[0].instructions;
        assert_eq!(instructions[0].class.opcode, Op::PtrAccessChain);
        assert_eq!(instructions[0].result_id, Some(result));
        assert_eq!(instructions[0].operands.len(), 3);
    }

    fn find_inst(ctx: &Ctx, id: Word) -> &Instruction {
        ctx.module.functions[0].blocks[0]
            .instructions
            .iter()
            .find(|i| i.result_id == Some(id))
            .expect("instruction with the given result id")
    }

    fn find_inst_anywhere(ctx: &Ctx, id: Word) -> &Instruction {
        ctx.module.functions[0]
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .find(|i| i.result_id == Some(id))
            .expect("instruction with the given result id")
    }

    #[test]
    fn flat_scalar_offset_through_vector_array_splits_index_and_lane() {
        let mut ctx = Ctx::new(Module::new());
        let float = ctx.ty_float();
        let float4 = ctx.ty_vecf(4);
        let array = ctx.ty_runtime_array(float4);
        let wrapper = ctx.get_or_create(Op::TypeStruct, None, vec![Operand::IdRef(array)]);
        let ptr_wrapper = ctx.ty_ptr(StorageClass::StorageBuffer, wrapper);
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let base = storage_buffer_var(&mut ctx, ptr_wrapper);
        let zero = ctx.const_uint(0);
        let flat = ctx.const_uint(5);
        let pointer = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::PtrAccessChain,
                Some(ptr_float),
                Some(pointer),
                vec![
                    Operand::IdRef(base),
                    Operand::IdRef(zero),
                    Operand::IdRef(flat),
                ],
            )],
        );

        rewrite_flat_scalar_ptr_access_through_vector_array(&mut ctx, 0);

        let rewritten = find_inst(&ctx, pointer);
        assert_eq!(rewritten.class.opcode, Op::InBoundsAccessChain);
        let indices = rewritten.operands[1..]
            .iter()
            .map(|operand| match operand {
                Operand::IdRef(id) => const_u32(&ctx, *id),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .expect("constant typed indices");
        assert_eq!(indices, vec![0, 1, 1]);
    }

    #[test]
    fn dynamic_struct_index_reinterpret_descends_member0_and_bitcasts_load() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let float = ctx.ty_float();
        let rt = ctx.ty_runtime_array(uint);
        let struct_id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_id),
            vec![Operand::IdRef(rt)],
        ));
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let base = storage_buffer_var(&mut ctx, ptr_struct);
        let dyn_idx = ctx.module.fresh_id();
        let chain_id = ctx.module.fresh_id();
        let val_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(dyn_idx), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_float),
                    Some(chain_id),
                    vec![Operand::IdRef(base), Operand::IdRef(dyn_idx)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(val_id),
                    vec![Operand::IdRef(chain_id)],
                ),
            ],
        );

        rewrite_dynamic_struct_index_reinterpret(&mut ctx, 0).unwrap();

        let chain = find_inst(&ctx, chain_id);
        assert_eq!(chain.class.opcode, Op::InBoundsAccessChain);
        assert_eq!(chain.operands.len(), 3, "member-0 index inserted");
        assert_eq!(chain.operands[0], Operand::IdRef(base));
        let Operand::IdRef(member0) = chain.operands[1] else {
            panic!("member index is not an id");
        };
        assert_eq!(
            const_u32(&ctx, member0),
            Some(0),
            "inserted index is constant 0"
        );
        let Some(Operand::IdRef(chain_pointee)) =
            type_def_of(&ctx, chain.result_type.unwrap()).and_then(|d| d.operands.get(1).cloned())
        else {
            panic!("chain result type is not a pointer");
        };
        assert_eq!(
            chain_pointee, uint,
            "chain retyped to the uint element pointer"
        );

        let cast = find_inst(&ctx, val_id);
        assert_eq!(cast.class.opcode, Op::Bitcast, "reinterpret load → bitcast");
        assert_eq!(cast.result_type, Some(float));
        let Operand::IdRef(load_id) = cast.operands[0] else {
            panic!("bitcast source is not an id");
        };
        assert_eq!(
            find_inst(&ctx, load_id).result_type,
            Some(uint),
            "the split load reads the uint element"
        );
    }

    #[test]
    fn dynamic_struct_index_reinterpret_is_idempotent() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let float = ctx.ty_float();
        let rt = ctx.ty_runtime_array(uint);
        let struct_id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_id),
            vec![Operand::IdRef(rt)],
        ));
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let base = storage_buffer_var(&mut ctx, ptr_struct);
        let dyn_idx = ctx.module.fresh_id();
        let chain_id = ctx.module.fresh_id();
        let val_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(dyn_idx), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_float),
                    Some(chain_id),
                    vec![Operand::IdRef(base), Operand::IdRef(dyn_idx)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(val_id),
                    vec![Operand::IdRef(chain_id)],
                ),
            ],
        );

        rewrite_dynamic_struct_index_reinterpret(&mut ctx, 0).unwrap();
        let after_first = ctx.module.functions[0].blocks[0].instructions.clone();
        rewrite_dynamic_struct_index_reinterpret(&mut ctx, 0).unwrap();
        assert_eq!(
            after_first, ctx.module.functions[0].blocks[0].instructions,
            "second application is a no-op"
        );
    }

    #[test]
    fn dynamic_struct_index_reinterpret_skips_constant_index() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let float = ctx.ty_float();
        let rt = ctx.ty_runtime_array(uint);
        let struct_id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_id),
            vec![Operand::IdRef(rt)],
        ));
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let base = storage_buffer_var(&mut ctx, ptr_struct);
        let const_idx = ctx.const_uint(3);
        let chain_id = ctx.module.fresh_id();
        let val_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_float),
                    Some(chain_id),
                    vec![Operand::IdRef(base), Operand::IdRef(const_idx)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(val_id),
                    vec![Operand::IdRef(chain_id)],
                ),
            ],
        );

        rewrite_dynamic_struct_index_reinterpret(&mut ctx, 0).unwrap();

        let chain = find_inst(&ctx, chain_id);
        assert_eq!(
            chain.operands.len(),
            2,
            "constant-index chain is left untouched"
        );
    }

    #[test]
    fn dynamic_struct_index_subword_reinterpret_extracts_half_from_word_lane() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let half = ctx.ty_half();
        let rt = ctx.ty_runtime_array(uint);
        let struct_id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_id),
            vec![Operand::IdRef(rt)],
        ));
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_half = ctx.ty_ptr(StorageClass::StorageBuffer, half);
        let base = storage_buffer_var(&mut ctx, ptr_struct);
        let dyn_idx = ctx.module.fresh_id();
        let chain_id = ctx.module.fresh_id();
        let val_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(dyn_idx), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_half),
                    Some(chain_id),
                    vec![Operand::IdRef(base), Operand::IdRef(dyn_idx)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(half),
                    Some(val_id),
                    vec![Operand::IdRef(chain_id)],
                ),
            ],
        );

        rewrite_dynamic_struct_index_subword_reinterpret(&mut ctx, 0).unwrap();

        let insts = &ctx.module.functions[0].blocks[0].instructions;
        assert!(
            !insts.iter().any(|inst| inst.result_id == Some(chain_id)),
            "the invalid half pointer should be eliminated"
        );
        let word_chain = insts
            .iter()
            .find(|inst| {
                matches!(inst.class.opcode, Op::InBoundsAccessChain | Op::AccessChain)
                    && inst.operands.first() == Some(&Operand::IdRef(base))
            })
            .expect("replacement chain into the backing word array");
        assert_eq!(word_chain.operands.len(), 3);
        let Some(Operand::IdRef(word_pointee)) = type_def_of(&ctx, word_chain.result_type.unwrap())
            .and_then(|d| d.operands.get(1).cloned())
        else {
            panic!("replacement chain result is not a pointer");
        };
        assert_eq!(word_pointee, uint, "replacement chain reads uint words");
        let cast = find_inst(&ctx, val_id);
        assert_eq!(cast.class.opcode, Op::Bitcast);
        assert_eq!(cast.result_type, Some(half));
        assert!(
            insts.iter().any(|inst| inst.class.opcode == Op::UDiv),
            "half element index is divided by two to address the backing word"
        );
        assert!(
            insts
                .iter()
                .any(|inst| inst.class.opcode == Op::ShiftRightLogical),
            "selected half lane is shifted down before truncation"
        );
    }

    #[test]
    fn dynamic_struct_index_subword_reinterpret_extracts_uchar_from_word_lane() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let uchar = ctx.ty_int8();
        let rt = ctx.ty_runtime_array(uint);
        let struct_id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_id),
            vec![Operand::IdRef(rt)],
        ));
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_uchar = ctx.ty_ptr(StorageClass::StorageBuffer, uchar);
        let base = storage_buffer_var(&mut ctx, ptr_struct);
        let dyn_idx = ctx.module.fresh_id();
        let chain_id = ctx.module.fresh_id();
        let val_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(dyn_idx), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_uchar),
                    Some(chain_id),
                    vec![Operand::IdRef(base), Operand::IdRef(dyn_idx)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(uchar),
                    Some(val_id),
                    vec![Operand::IdRef(chain_id)],
                ),
            ],
        );

        rewrite_dynamic_struct_index_subword_reinterpret(&mut ctx, 0).unwrap();

        let insts = &ctx.module.functions[0].blocks[0].instructions;
        assert!(
            !insts.iter().any(|inst| inst.result_id == Some(chain_id)),
            "the invalid uchar pointer should be eliminated"
        );
        let div = insts
            .iter()
            .find(|inst| inst.class.opcode == Op::UDiv)
            .expect("byte element index is divided by four to address the backing word");
        let Operand::IdRef(divisor) = div.operands[1] else {
            panic!("divisor is not an id")
        };
        assert_eq!(const_u32(&ctx, divisor), Some(4));
        let lane = insts
            .iter()
            .find(|inst| inst.class.opcode == Op::BitwiseAnd && inst.result_type == Some(uint))
            .expect("byte lane is masked out of the dynamic index");
        let Operand::IdRef(mask) = lane.operands[1] else {
            panic!("lane mask is not an id")
        };
        assert_eq!(const_u32(&ctx, mask), Some(3));
        assert_eq!(find_inst(&ctx, val_id).result_type, Some(uchar));
    }

    #[test]
    fn dynamic_struct_index_subword_reinterpret_packs_ushort_store_into_word_lane() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let ushort = ctx.ty_int16();
        let rt = ctx.ty_runtime_array(uint);
        let struct_id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_id),
            vec![Operand::IdRef(rt)],
        ));
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_ushort = ctx.ty_ptr(StorageClass::StorageBuffer, ushort);
        let base = storage_buffer_var(&mut ctx, ptr_struct);
        let dyn_idx = ctx.module.fresh_id();
        let object = ctx.module.fresh_id();
        let chain_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(dyn_idx), vec![]),
                Instruction::new(Op::Undef, Some(ushort), Some(object), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_ushort),
                    Some(chain_id),
                    vec![Operand::IdRef(base), Operand::IdRef(dyn_idx)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(chain_id), Operand::IdRef(object)],
                ),
            ],
        );

        rewrite_dynamic_struct_index_subword_reinterpret(&mut ctx, 0).unwrap();

        let insts = &ctx.module.functions[0].blocks[0].instructions;
        assert!(
            !insts.iter().any(|inst| inst.result_id == Some(chain_id)),
            "the invalid ushort pointer should be eliminated"
        );
        assert!(
            insts.iter().any(|inst| inst.class.opcode == Op::Not),
            "store clears the selected 16-bit lane before OR-ing in new bits"
        );
        let opcodes: Vec<Op> = insts.iter().map(|inst| inst.class.opcode).collect();
        assert!(
            opcodes.contains(&Op::AtomicAnd),
            "store clears the selected 16-bit lane atomically: {opcodes:?}"
        );
        assert!(
            opcodes.contains(&Op::AtomicOr),
            "store sets the selected 16-bit lane atomically: {opcodes:?}"
        );
        assert!(
            !opcodes.contains(&Op::Store) && !opcodes.contains(&Op::Load),
            "no plain word load or store survives the rewrite: {opcodes:?}"
        );
        let atomics: Vec<&Instruction> = insts
            .iter()
            .filter(|inst| matches!(inst.class.opcode, Op::AtomicAnd | Op::AtomicOr))
            .collect();
        assert_eq!(atomics.len(), 2);
        assert_eq!(atomics[0].operands[0], atomics[1].operands[0]);
        assert_eq!(atomics[0].result_type, Some(uint));
    }

    #[test]
    fn dynamic_struct_index_wide_word_reinterpret_assembles_ulong_from_two_words() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let ulong = ctx.ty_ulong();
        let rt = ctx.ty_runtime_array(uint);
        let struct_id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_id),
            vec![Operand::IdRef(rt)],
        ));
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_ulong = ctx.ty_ptr(StorageClass::StorageBuffer, ulong);
        let base = storage_buffer_var(&mut ctx, ptr_struct);
        let dyn_idx = ctx.module.fresh_id();
        let chain_id = ctx.module.fresh_id();
        let val_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(dyn_idx), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_ulong),
                    Some(chain_id),
                    vec![Operand::IdRef(base), Operand::IdRef(dyn_idx)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(ulong),
                    Some(val_id),
                    vec![Operand::IdRef(chain_id)],
                ),
            ],
        );

        rewrite_dynamic_struct_index_wide_word_reinterpret(&mut ctx, 0).unwrap();

        let insts = &ctx.module.functions[0].blocks[0].instructions;
        assert!(
            !insts.iter().any(|inst| inst.result_id == Some(chain_id)),
            "the invalid ulong pointer should be eliminated"
        );
        let chains: Vec<&Instruction> = insts
            .iter()
            .filter(|inst| {
                matches!(inst.class.opcode, Op::InBoundsAccessChain | Op::AccessChain)
                    && inst.operands.first() == Some(&Operand::IdRef(base))
            })
            .collect();
        assert_eq!(chains.len(), 2, "one word pointer per half of the u64");
        assert!(
            insts
                .iter()
                .filter(|inst| inst.class.opcode == Op::Load && inst.result_type == Some(uint))
                .count()
                == 2,
            "wide load reads two uint words"
        );
        assert_eq!(find_inst(&ctx, val_id).result_type, Some(ulong));
        assert!(
            insts.iter().any(|inst| inst.class.opcode == Op::BitwiseOr),
            "low and high words are OR-assembled"
        );
    }

    #[test]
    fn dynamic_struct_index_wide_word_reinterpret_splits_ulong_store_into_two_words() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let ulong = ctx.ty_ulong();
        let rt = ctx.ty_runtime_array(uint);
        let struct_id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_id),
            vec![Operand::IdRef(rt)],
        ));
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_ulong = ctx.ty_ptr(StorageClass::StorageBuffer, ulong);
        let base = storage_buffer_var(&mut ctx, ptr_struct);
        let dyn_idx = ctx.module.fresh_id();
        let object = ctx.module.fresh_id();
        let chain_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(dyn_idx), vec![]),
                Instruction::new(Op::Undef, Some(ulong), Some(object), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_ulong),
                    Some(chain_id),
                    vec![Operand::IdRef(base), Operand::IdRef(dyn_idx)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(chain_id), Operand::IdRef(object)],
                ),
            ],
        );

        rewrite_dynamic_struct_index_wide_word_reinterpret(&mut ctx, 0).unwrap();

        let insts = &ctx.module.functions[0].blocks[0].instructions;
        assert!(
            !insts.iter().any(|inst| inst.result_id == Some(chain_id)),
            "the invalid ulong pointer should be eliminated"
        );
        let stores: Vec<&Instruction> = insts
            .iter()
            .filter(|inst| inst.class.opcode == Op::Store)
            .collect();
        assert_eq!(stores.len(), 2, "wide store writes two uint words");
        for store in stores {
            let Operand::IdRef(stored) = store.operands[1] else {
                panic!("store object is not an id");
            };
            assert_eq!(value_result_type(&ctx, stored), Some(uint));
        }
        assert!(
            insts
                .iter()
                .any(|inst| inst.class.opcode == Op::ShiftRightLogical),
            "high word is extracted by shifting the 64-bit object down"
        );
    }

    #[test]
    fn dynamic_struct_index_vector_reinterpret_replays_scalar_lanes() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let v2uint = ctx.ty_vec_uint(2);
        let runtime = ctx.ty_runtime_array(uint);
        let block = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(block),
            vec![Operand::IdRef(runtime)],
        ));
        let ptr_block = ctx.ty_ptr(StorageClass::StorageBuffer, block);
        let ptr_v2uint = ctx.ty_ptr(StorageClass::StorageBuffer, v2uint);
        let base = storage_buffer_var(&mut ctx, ptr_block);
        let dyn_idx = ctx.module.fresh_id();
        let chain = ctx.module.fresh_id();
        let value = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(dyn_idx), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_v2uint),
                    Some(chain),
                    vec![Operand::IdRef(base), Operand::IdRef(dyn_idx)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(v2uint),
                    Some(value),
                    vec![Operand::IdRef(chain)],
                ),
            ],
        );

        rewrite_dynamic_struct_index_vector_reinterpret(&mut ctx, 0).unwrap();

        let insts = &ctx.module.functions[0].blocks[0].instructions;
        assert!(
            !insts.iter().any(|inst| inst.result_id == Some(chain)),
            "the illegal vector pointer must be eliminated"
        );
        let mul = insts
            .iter()
            .find(|inst| inst.class.opcode == Op::IMul)
            .expect("vector index is scaled by lane count");
        assert_eq!(mul.operands.first(), Some(&Operand::IdRef(dyn_idx)));
        let Operand::IdRef(two) = mul.operands.get(1).expect("lane multiplier") else {
            panic!("lane multiplier is not an id")
        };
        assert_eq!(const_u32(&ctx, *two), Some(2));

        let chains: Vec<&Instruction> = insts
            .iter()
            .filter(|inst| inst.class.opcode == Op::InBoundsAccessChain)
            .collect();
        assert_eq!(chains.len(), 2, "one scalar pointer per vector lane");
        for chain in &chains {
            assert_eq!(chain.operands.first(), Some(&Operand::IdRef(base)));
            let Operand::IdRef(member0) = chain.operands.get(1).expect("member-0 index") else {
                panic!("member index is not an id")
            };
            assert_eq!(const_u32(&ctx, *member0), Some(0));
        }

        let rebuilt = find_inst(&ctx, value);
        assert_eq!(rebuilt.class.opcode, Op::CompositeConstruct);
        assert_eq!(rebuilt.result_type, Some(v2uint));
        assert_eq!(rebuilt.operands.len(), 2);
    }

    #[test]
    fn dynamic_homogeneous_function_struct_index_load_becomes_select() {
        let mut ctx = Ctx::new(Module::new());
        let v3u16 = ctx.ty_vec_u16(3);
        let inner_struct = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(inner_struct),
            vec![
                Operand::IdRef(v3u16),
                Operand::IdRef(v3u16),
                Operand::IdRef(v3u16),
            ],
        ));
        let struct_id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_id),
            vec![Operand::IdRef(inner_struct)],
        ));
        let ptr_struct = ctx.ty_ptr(StorageClass::Function, struct_id);
        let ptr_v3u16 = ctx.ty_ptr(StorageClass::Function, v3u16);
        let uint = ctx.ty_uint();
        let zero = ctx.const_uint(0);
        let dyn_idx = ctx.module.fresh_id();
        let base = ctx.module.fresh_id();
        let chain = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();

        install_entry(
            &mut ctx,
            vec![
                Instruction::new(
                    Op::Variable,
                    Some(ptr_struct),
                    Some(base),
                    vec![Operand::StorageClass(StorageClass::Function)],
                ),
                Instruction::new(
                    Op::Bitcast,
                    Some(uint),
                    Some(dyn_idx),
                    vec![Operand::IdRef(zero)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_v3u16),
                    Some(chain),
                    vec![Operand::IdRef(base), Operand::IdRef(dyn_idx)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(v3u16),
                    Some(loaded),
                    vec![Operand::IdRef(chain)],
                ),
            ],
        );

        rewrite_dynamic_homogeneous_struct_index_load(&mut ctx, 0).unwrap();

        let insts = &ctx.module.functions[0].blocks[0].instructions;
        assert!(
            !insts.iter().any(|inst| inst.result_id == Some(chain)),
            "invalid dynamic pointer should be deleted"
        );
        assert!(
            insts
                .iter()
                .any(|inst| inst.class.opcode == Op::Select && inst.result_id == Some(loaded)),
            "load result should be produced by the select cascade"
        );
        assert_eq!(
            insts
                .iter()
                .filter(|inst| {
                    matches!(inst.class.opcode, Op::AccessChain | Op::InBoundsAccessChain)
                        && inst.result_type == Some(ptr_v3u16)
                })
                .count(),
            3,
            "one constant-member chain per homogeneous field"
        );
        for chain in insts.iter().filter(|inst| {
            matches!(inst.class.opcode, Op::AccessChain | Op::InBoundsAccessChain)
                && inst.result_type == Some(ptr_v3u16)
        }) {
            let Operand::IdRef(wrapper_member) = chain.operands.get(1).unwrap() else {
                panic!("wrapper member is not an id")
            };
            assert_eq!(const_u32(&ctx, *wrapper_member), Some(0));
        }
        assert!(
            insts.iter().any(|inst| {
                inst.class.opcode == Op::CompositeConstruct
                    && inst.result_type.and_then(|ty| {
                        type_def_of(&ctx, ty).and_then(|def| match def.operands.get(1) {
                            Some(Operand::LiteralBit32(lanes)) => Some(*lanes),
                            _ => None,
                        })
                    }) == Some(3)
            }),
            "vector selects need a v3bool condition"
        );
    }

    struct ChainedFixture {
        buf: Word,
        dyn_f: Word,
        dyn_u: Word,
        out_id: Word,
        val_id: Word,
        uint: Word,
        float: Word,
    }

    fn build_chained_reinterpret(ctx: &mut Ctx) -> ChainedFixture {
        let uint = ctx.ty_uint();
        let float = ctx.ty_float();
        let rt = ctx.ty_runtime_array(uint);
        let struct_id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_id),
            vec![Operand::IdRef(rt)],
        ));
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_uint = ctx.ty_ptr(StorageClass::StorageBuffer, uint);
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let buf = storage_buffer_var(ctx, ptr_struct);
        let zero = ctx.const_uint(0);
        let dyn_f = ctx.module.fresh_id();
        let dyn_u = ctx.module.fresh_id();
        let inner_id = ctx.module.fresh_id();
        let out_id = ctx.module.fresh_id();
        let val_id = ctx.module.fresh_id();
        install_entry(
            ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(dyn_f), vec![]),
                Instruction::new(Op::Undef, Some(uint), Some(dyn_u), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_uint),
                    Some(inner_id),
                    vec![
                        Operand::IdRef(buf),
                        Operand::IdRef(zero),
                        Operand::IdRef(dyn_f),
                    ],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_float),
                    Some(out_id),
                    vec![
                        Operand::IdRef(inner_id),
                        Operand::IdRef(zero),
                        Operand::IdRef(dyn_u),
                    ],
                ),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(val_id),
                    vec![Operand::IdRef(out_id)],
                ),
            ],
        );
        ChainedFixture {
            buf,
            dyn_f,
            dyn_u,
            out_id,
            val_id,
            uint,
            float,
        }
    }

    #[test]
    fn chained_element_reinterpret_reroots_with_summed_index() {
        let mut ctx = Ctx::new(Module::new());
        let fx = build_chained_reinterpret(&mut ctx);

        rewrite_chained_element_reinterpret(&mut ctx, 0).unwrap();

        let out = find_inst(&ctx, fx.out_id);
        assert_eq!(out.class.opcode, Op::InBoundsAccessChain);
        assert_eq!(
            out.operands.len(),
            3,
            "outer chain keeps member-0 + summed index"
        );
        assert_eq!(
            out.operands[0],
            Operand::IdRef(fx.buf),
            "re-rooted onto the buffer"
        );
        let Operand::IdRef(member0) = out.operands[1] else {
            panic!("member index is not an id");
        };
        assert_eq!(const_u32(&ctx, member0), Some(0));
        let Operand::IdRef(sum) = out.operands[2] else {
            panic!("summed index is not an id");
        };
        let sum_inst = find_inst(&ctx, sum);
        assert_eq!(
            sum_inst.class.opcode,
            Op::IAdd,
            "index is the sum of the two dynamic indices"
        );
        assert_eq!(sum_inst.result_type, Some(fx.uint));
        assert!(
            sum_inst.operands.contains(&Operand::IdRef(fx.dyn_f))
                && sum_inst.operands.contains(&Operand::IdRef(fx.dyn_u)),
            "sum adds dynF and dynU"
        );

        let cast = find_inst(&ctx, fx.val_id);
        assert_eq!(cast.class.opcode, Op::Bitcast, "reinterpret load → bitcast");
        assert_eq!(cast.result_type, Some(fx.float));
        let Operand::IdRef(load_id) = cast.operands[0] else {
            panic!("bitcast source is not an id");
        };
        assert_eq!(
            find_inst(&ctx, load_id).result_type,
            Some(fx.uint),
            "split load reads uint"
        );
    }

    #[test]
    fn chained_element_reinterpret_is_idempotent() {
        let mut ctx = Ctx::new(Module::new());
        build_chained_reinterpret(&mut ctx);

        rewrite_chained_element_reinterpret(&mut ctx, 0).unwrap();
        let after_first = ctx.module.functions[0].blocks[0].instructions.clone();
        rewrite_chained_element_reinterpret(&mut ctx, 0).unwrap();
        assert_eq!(
            after_first, ctx.module.functions[0].blocks[0].instructions,
            "second application is a no-op"
        );
    }

    fn ty_int16(ctx: &mut Ctx) -> Word {
        let id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeInt,
            None,
            Some(id),
            vec![Operand::LiteralBit32(16), Operand::LiteralBit32(0)],
        ));
        id
    }

    struct ByteBufFixture {
        out_id: Word,
        val_id: Word,
        u16: Word,
        u32: Word,
    }

    fn build_byte_buffer_widen(ctx: &mut Ctx) -> ByteBufFixture {
        let u16 = ty_int16(ctx);
        let u32 = ctx.ty_uint();
        let rt = ctx.ty_runtime_array(u16);
        decorate_array_stride(ctx, rt, 2);
        let struct_id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_id),
            vec![Operand::IdRef(rt)],
        ));
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_u16 = ctx.ty_ptr(StorageClass::StorageBuffer, u16);
        let ptr_u32 = ctx.ty_ptr(StorageClass::StorageBuffer, u32);
        let buf = storage_buffer_var(ctx, ptr_struct);
        let zero = ctx.const_uint(0);
        let byte_idx = ctx.module.fresh_id();
        let k = ctx.module.fresh_id();
        let inner_id = ctx.module.fresh_id();
        let out_id = ctx.module.fresh_id();
        let val_id = ctx.module.fresh_id();
        install_entry(
            ctx,
            vec![
                Instruction::new(Op::Undef, Some(u32), Some(byte_idx), vec![]),
                Instruction::new(Op::Undef, Some(u32), Some(k), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_u16),
                    Some(inner_id),
                    vec![
                        Operand::IdRef(buf),
                        Operand::IdRef(zero),
                        Operand::IdRef(byte_idx),
                    ],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_u32),
                    Some(out_id),
                    vec![Operand::IdRef(inner_id), Operand::IdRef(k)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(u32),
                    Some(val_id),
                    vec![Operand::IdRef(out_id)],
                ),
            ],
        );
        ByteBufFixture {
            out_id,
            val_id,
            u16,
            u32,
        }
    }

    #[test]
    fn half_array_chained_reinterpret_widens_through_nested_member() {
        let mut ctx = Ctx::new(Module::new());
        let half = ctx.ty_half();
        let uint = ctx.ty_uint();
        let array = ctx.ty_array(half, 8);
        decorate_array_stride(&mut ctx, array, 2);
        let pair = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeStruct,
            None,
            Some(pair),
            vec![Operand::IdRef(array), Operand::IdRef(array)],
        ));
        let ptr_pair = ctx.ty_ptr(StorageClass::StorageBuffer, pair);
        let ptr_half = ctx.ty_ptr(StorageClass::StorageBuffer, half);
        let ptr_uint = ctx.ty_ptr(StorageClass::StorageBuffer, uint);
        let buffer = storage_buffer_var(&mut ctx, ptr_pair);
        let zero = ctx.const_uint(0);
        let element = ctx.const_uint(1);
        let outer_index = ctx.module.fresh_id();
        let inner = ctx.module.fresh_id();
        let outer = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(outer_index), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_half),
                    Some(inner),
                    vec![
                        Operand::IdRef(buffer),
                        Operand::IdRef(zero),
                        Operand::IdRef(element),
                    ],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_uint),
                    Some(outer),
                    vec![
                        Operand::IdRef(inner),
                        Operand::IdRef(zero),
                        Operand::IdRef(outer_index),
                    ],
                ),
                Instruction::new(
                    Op::Load,
                    Some(uint),
                    Some(loaded),
                    vec![Operand::IdRef(outer)],
                ),
            ],
        );

        rewrite_byte_buffer_chained_reinterpret(&mut ctx, 0).unwrap();

        let body = &ctx.module.functions[0].blocks[0].instructions;
        assert_eq!(find_inst(&ctx, loaded).class.opcode, Op::BitwiseOr);
        assert_eq!(
            body.iter()
                .filter(|inst| inst.class.opcode == Op::Load && inst.result_type == Some(half))
                .count(),
            2
        );
        assert_eq!(
            body.iter()
                .filter(|inst| inst.class.opcode == Op::Bitcast)
                .count(),
            2,
            "each half slot is reinterpreted as its 16-bit integer payload"
        );
        assert!(body.iter().all(|inst| inst.result_id != Some(outer)));
    }

    #[test]
    fn byte_buffer_chained_reinterpret_expands_widening_load_into_slots() {
        let mut ctx = Ctx::new(Module::new());
        let fx = build_byte_buffer_widen(&mut ctx);

        rewrite_byte_buffer_chained_reinterpret(&mut ctx, 0).unwrap();

        let block = &ctx.module.functions[0].blocks[0].instructions;
        let val = find_inst(&ctx, fx.val_id);
        assert_eq!(
            val.class.opcode,
            Op::BitwiseOr,
            "little-endian OR-assembled result"
        );
        assert_eq!(val.result_type, Some(fx.u32));
        let slot_loads = block
            .iter()
            .filter(|i| i.class.opcode == Op::Load && i.result_type == Some(fx.u16))
            .count();
        assert_eq!(slot_loads, 2, "two little-endian narrow slot loads");
        assert!(
            block.iter().any(|i| i.class.opcode == Op::IMul),
            "out_idx * ratio"
        );
        assert!(
            block.iter().any(|i| i.class.opcode == Op::IAdd),
            "byteIdx + out_idx*ratio"
        );
        assert!(
            block.iter().all(|i| i.result_id != Some(fx.out_id)),
            "outer widening chain replaced"
        );
    }

    #[test]
    fn byte_buffer_chained_reinterpret_is_idempotent() {
        let mut ctx = Ctx::new(Module::new());
        build_byte_buffer_widen(&mut ctx);

        rewrite_byte_buffer_chained_reinterpret(&mut ctx, 0).unwrap();
        let after_first = ctx.module.functions[0].blocks[0].instructions.clone();
        rewrite_byte_buffer_chained_reinterpret(&mut ctx, 0).unwrap();
        assert_eq!(
            after_first, ctx.module.functions[0].blocks[0].instructions,
            "second application is a no-op"
        );
    }

    fn ty_uint8(ctx: &mut Ctx) -> Word {
        let id = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::TypeInt,
            None,
            Some(id),
            vec![
                Operand::LiteralBit32(RAW_BYTE_POINTER_ELEMENT_BITS),
                Operand::LiteralBit32(0),
            ],
        ));
        id
    }

    struct RawByteWideFixture {
        chain_id: Word,
        load_id: Word,
        byte: Word,
        v4float: Word,
        base_phi: Word,
        index: Word,
        ulong: Word,
    }

    fn build_raw_byte_phi_wide_load(ctx: &mut Ctx) -> RawByteWideFixture {
        let byte = ty_uint8(ctx);
        let ulong = ctx.ty_ulong();
        let v4float = ctx.ty_vecf(4);
        let ptr_byte = ctx.ty_ptr(StorageClass::StorageBuffer, byte);
        let ptr_v4float = ctx.ty_ptr(StorageClass::StorageBuffer, v4float);
        let left = ctx.module.fresh_id();
        let right = ctx.module.fresh_id();
        let base_phi = ctx.module.fresh_id();
        let pred_left = ctx.module.fresh_id();
        let pred_right = ctx.module.fresh_id();
        let index = ctx.module.fresh_id();
        let chain_id = ctx.module.fresh_id();
        let load_id = ctx.module.fresh_id();
        install_entry(
            ctx,
            vec![
                Instruction::new(Op::Undef, Some(ptr_byte), Some(left), vec![]),
                Instruction::new(Op::Undef, Some(ptr_byte), Some(right), vec![]),
                Instruction::new(
                    Op::Phi,
                    Some(ptr_byte),
                    Some(base_phi),
                    vec![
                        Operand::IdRef(left),
                        Operand::IdRef(pred_left),
                        Operand::IdRef(right),
                        Operand::IdRef(pred_right),
                    ],
                ),
                Instruction::new(Op::Undef, Some(ulong), Some(index), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_v4float),
                    Some(chain_id),
                    vec![Operand::IdRef(base_phi), Operand::IdRef(index)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(v4float),
                    Some(load_id),
                    vec![Operand::IdRef(chain_id)],
                ),
            ],
        );
        RawByteWideFixture {
            chain_id,
            load_id,
            byte,
            v4float,
            base_phi,
            index,
            ulong,
        }
    }

    #[test]
    fn raw_byte_pointer_wide_load_replays_vector_through_phi() {
        let mut ctx = Ctx::new(Module::new());
        let fx = build_raw_byte_phi_wide_load(&mut ctx);

        rewrite_raw_byte_pointer_wide_loads(&mut ctx, 0);

        let block = &ctx.module.functions[0].blocks[0].instructions;
        assert!(
            block.iter().all(|inst| inst.result_id != Some(fx.chain_id)),
            "the invalid wide pointer is removed"
        );
        let result = find_inst(&ctx, fx.load_id);
        assert_eq!(result.class.opcode, Op::CompositeConstruct);
        assert_eq!(result.result_type, Some(fx.v4float));
        assert_eq!(
            block
                .iter()
                .filter(|inst| inst.class.opcode == Op::PtrAccessChain)
                .count(),
            16,
            "four 32-bit lanes replay through sixteen byte pointers"
        );
        assert_eq!(
            block
                .iter()
                .filter(|inst| inst.class.opcode == Op::Load && inst.result_type == Some(fx.byte))
                .count(),
            16,
            "one byte load per byte of the float4"
        );
        assert!(
            block.iter().any(|inst| {
                inst.class.opcode == Op::IMul
                    && inst.result_type == Some(fx.ulong)
                    && inst.operands.contains(&Operand::IdRef(fx.index))
            }),
            "the original 64-bit GEP index scales the whole float4"
        );
        assert!(
            block
                .iter()
                .filter(|inst| inst.class.opcode == Op::PtrAccessChain)
                .all(|inst| { inst.operands.first() == Some(&Operand::IdRef(fx.base_phi)) }),
            "all byte accesses preserve the selected/phi raw base"
        );
    }

    #[test]
    fn raw_byte_pointer_half_load_replays_two_bytes() {
        let mut ctx = Ctx::new(Module::new());
        let byte = ty_uint8(&mut ctx);
        let half = ctx.ty_half();
        let ulong = ctx.ty_ulong();
        let ptr_byte = ctx.ty_ptr(StorageClass::StorageBuffer, byte);
        let ptr_half = ctx.ty_ptr(StorageClass::StorageBuffer, half);
        let base = ctx.module.fresh_id();
        let index = ctx.module.fresh_id();
        let chain = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(ptr_byte), Some(base), vec![]),
                Instruction::new(Op::Undef, Some(ulong), Some(index), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_half),
                    Some(chain),
                    vec![Operand::IdRef(base), Operand::IdRef(index)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(half),
                    Some(loaded),
                    vec![Operand::IdRef(chain)],
                ),
            ],
        );

        rewrite_raw_byte_pointer_wide_loads(&mut ctx, 0);

        let body = &ctx.module.functions[0].blocks[0].instructions;
        assert!(body
            .iter()
            .all(|instruction| instruction.result_id != Some(chain)));
        assert_eq!(find_inst(&ctx, loaded).class.opcode, Op::CopyObject);
        assert_eq!(
            body.iter()
                .filter(|instruction| instruction.class.opcode == Op::PtrAccessChain)
                .count(),
            2
        );
        assert_eq!(
            body.iter()
                .filter(|instruction| {
                    instruction.class.opcode == Op::Load && instruction.result_type == Some(byte)
                })
                .count(),
            2
        );
    }

    #[test]
    fn raw_byte_block_half_load_descends_through_member_zero() {
        let mut ctx = Ctx::new(Module::new());
        let byte = ty_uint8(&mut ctx);
        let half = ctx.ty_half();
        let ulong = ctx.ty_ulong();
        let byte_array = ctx.get_or_create(Op::TypeRuntimeArray, None, vec![Operand::IdRef(byte)]);
        let byte_block = ctx.get_or_create(Op::TypeStruct, None, vec![Operand::IdRef(byte_array)]);
        let ptr_block = ctx.ty_ptr(StorageClass::StorageBuffer, byte_block);
        let ptr_half = ctx.ty_ptr(StorageClass::StorageBuffer, half);
        let base = ctx.module.fresh_id();
        let index = ctx.module.fresh_id();
        let chain = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(ptr_block), Some(base), vec![]),
                Instruction::new(Op::Undef, Some(ulong), Some(index), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_half),
                    Some(chain),
                    vec![Operand::IdRef(base), Operand::IdRef(index)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(half),
                    Some(loaded),
                    vec![Operand::IdRef(chain)],
                ),
            ],
        );

        rewrite_raw_byte_pointer_wide_loads(&mut ctx, 0);

        let body = &ctx.module.functions[0].blocks[0].instructions;
        assert!(body
            .iter()
            .all(|instruction| instruction.result_id != Some(chain)));
        let byte_base = body
            .iter()
            .find(|instruction| {
                instruction.class.opcode == Op::InBoundsAccessChain
                    && instruction.operands.len() == 3
                    && instruction.operands.first() == Some(&Operand::IdRef(base))
            })
            .and_then(|instruction| instruction.result_id)
            .expect("member-zero byte base");
        assert_eq!(find_inst(&ctx, loaded).class.opcode, Op::CopyObject);
        assert_eq!(
            body.iter()
                .filter(|instruction| instruction.class.opcode == Op::PtrAccessChain)
                .count(),
            2
        );
        assert!(body
            .iter()
            .filter(|instruction| instruction.class.opcode == Op::PtrAccessChain)
            .all(|instruction| {
                instruction.operands.first() == Some(&Operand::IdRef(byte_base))
            }));
    }

    #[test]
    fn exact_raw_byte_block_fact_replays_typed_leaf_load() {
        let mut ctx = Ctx::new(Module::new());
        let byte = ty_uint8(&mut ctx);
        let half = ctx.ty_half();
        let half2 = ctx.ty_vech(2);
        let typed_struct = ctx.get_or_create(
            Op::TypeStruct,
            None,
            vec![Operand::IdRef(half), Operand::IdRef(half2)],
        );
        let byte_array = ctx.get_or_create(Op::TypeRuntimeArray, None, vec![Operand::IdRef(byte)]);
        let byte_block = ctx.get_or_create(Op::TypeStruct, None, vec![Operand::IdRef(byte_array)]);
        let ptr_block = ctx.ty_ptr(StorageClass::StorageBuffer, byte_block);
        let ptr_typed_struct = ctx.ty_ptr(StorageClass::StorageBuffer, typed_struct);
        let ptr_half2 = ctx.ty_ptr(StorageClass::StorageBuffer, half2);
        let root = storage_buffer_var(&mut ctx, ptr_block);
        let stale_aggregate = ctx.module.fresh_id();
        let leaf_pointer = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        let offset10 = ctx.const_uint(10);
        let member1 = ctx.const_uint(1);
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_typed_struct),
                    Some(stale_aggregate),
                    vec![Operand::IdRef(root), Operand::IdRef(offset10)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_half2),
                    Some(leaf_pointer),
                    vec![Operand::IdRef(stale_aggregate), Operand::IdRef(member1)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(half2),
                    Some(loaded),
                    vec![Operand::IdRef(leaf_pointer)],
                ),
            ],
        );
        ctx.emit_sidecar
            .buffer_access_offsets
            .push(crate::emit_sidecar::BufferAccessOffset {
                id: stale_aggregate,
                root,
                byte_offset: 10,
            });

        rewrite_exact_raw_byte_block_memory(&mut ctx, 0);

        let body = &ctx.module.functions[0].blocks[0].instructions;
        assert_eq!(find_inst(&ctx, loaded).class.opcode, Op::CompositeConstruct);
        assert_eq!(
            body.iter()
                .filter(|instruction| instruction.class.opcode == Op::Load)
                .count(),
            4,
            "two half lanes are reconstructed from four exact bytes"
        );
        let byte_offsets = body
            .iter()
            .filter(|instruction| instruction.class.opcode == Op::PtrAccessChain)
            .filter_map(|instruction| match instruction.operands.get(1) {
                Some(Operand::IdRef(id)) => const_u32(&ctx, *id),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            byte_offsets,
            vec![14, 16],
            "each half lane starts at its exact two-byte boundary"
        );
    }

    #[test]
    fn exact_raw_byte_block_replays_unsigned_byte_vectors() {
        for lanes in [2, 4] {
            let mut ctx = Ctx::new(Module::new());
            let byte = ty_uint8(&mut ctx);
            let vector = ctx.get_or_create(
                Op::TypeVector,
                None,
                vec![Operand::IdRef(byte), Operand::LiteralBit32(lanes)],
            );
            let array = ctx.get_or_create(Op::TypeRuntimeArray, None, vec![Operand::IdRef(byte)]);
            let block = ctx.get_or_create(Op::TypeStruct, None, vec![Operand::IdRef(array)]);
            let root_ptr = ctx.ty_ptr(StorageClass::StorageBuffer, block);
            let vector_ptr = ctx.ty_ptr(StorageClass::StorageBuffer, vector);
            let root = storage_buffer_var(&mut ctx, root_ptr);
            let pointer = ctx.module.fresh_id();
            let loaded = ctx.module.fresh_id();
            let zero = ctx.const_uint(0);
            let offset = ctx.const_uint(7);
            install_entry(
                &mut ctx,
                vec![
                    Instruction::new(
                        Op::InBoundsAccessChain,
                        Some(vector_ptr),
                        Some(pointer),
                        vec![
                            Operand::IdRef(root),
                            Operand::IdRef(zero),
                            Operand::IdRef(offset),
                        ],
                    ),
                    Instruction::new(
                        Op::Load,
                        Some(vector),
                        Some(loaded),
                        vec![Operand::IdRef(pointer)],
                    ),
                ],
            );
            ctx.emit_sidecar
                .buffer_access_offsets
                .push(crate::emit_sidecar::BufferAccessOffset {
                    id: pointer,
                    root,
                    byte_offset: 7,
                });
            rewrite_exact_raw_byte_block_memory(&mut ctx, 0);
            let construct = find_inst(&ctx, loaded);
            assert_eq!(construct.class.opcode, Op::CompositeConstruct);
            assert_eq!(construct.operands.len(), lanes as usize);
            let body = &ctx.module.functions[0].blocks[0].instructions;
            let offsets = body
                .iter()
                .filter(|inst| inst.class.opcode == Op::PtrAccessChain)
                .map(|inst| match inst.operands.get(1) {
                    Some(Operand::IdRef(id)) => const_u32(&ctx, *id).unwrap(),
                    _ => panic!("missing exact byte offset"),
                })
                .collect::<Vec<_>>();
            assert_eq!(offsets, (7..7 + lanes).collect::<Vec<_>>());
            assert_eq!(
                body.iter()
                    .filter(|inst| inst.class.opcode == Op::Load)
                    .count(),
                lanes as usize
            );
            for operand in &construct.operands {
                let Operand::IdRef(id) = operand else {
                    panic!("expected lane value")
                };
                assert_eq!(find_inst(&ctx, *id).result_type, Some(byte));
            }
        }
    }

    #[test]
    fn exact_raw_word_fact_inherits_function_root_storage() {
        let mut ctx = Ctx::new(Module::new());
        let byte = ctx.ty_int8();
        let uint = ctx.ty_uint();
        let word_count = ctx.const_uint(64);
        let words = ctx.get_or_create(
            Op::TypeArray,
            None,
            vec![Operand::IdRef(uint), Operand::IdRef(word_count)],
        );
        let block = ctx.get_or_create(Op::TypeStruct, None, vec![Operand::IdRef(words)]);
        let ptr_block = ctx.ty_ptr(StorageClass::Function, block);
        let ptr_uint = ctx.ty_ptr(StorageClass::Function, uint);
        let ptr_byte = ctx.ty_ptr(StorageClass::Function, byte);
        let float = ctx.ty_float();
        let float4 = ctx.ty_vecf(4);
        let root = ctx.module.fresh_id();
        let remapped_root = ctx.module.fresh_id();
        let stale_pointer = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        let stale_vector_pointer = ctx.module.fresh_id();
        let loaded_vector = ctx.module.fresh_id();
        let stale_float_pointer = ctx.module.fresh_id();
        let loaded_float = ctx.module.fresh_id();
        let zero = ctx.const_uint(0);
        let one = ctx.const_uint(1);
        let offset16 = ctx.const_uint(16);
        let offset149 = ctx.const_uint(149);
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(
                    Op::Variable,
                    Some(ptr_block),
                    Some(root),
                    vec![Operand::StorageClass(StorageClass::Function)],
                ),
                Instruction::new(
                    Op::AccessChain,
                    Some(ptr_uint),
                    Some(remapped_root),
                    vec![
                        Operand::IdRef(root),
                        Operand::IdRef(zero),
                        Operand::IdRef(zero),
                    ],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_byte),
                    Some(stale_pointer),
                    vec![
                        Operand::IdRef(remapped_root),
                        Operand::IdRef(zero),
                        Operand::IdRef(one),
                        Operand::IdRef(offset149),
                    ],
                ),
                Instruction::new(
                    Op::Load,
                    Some(byte),
                    Some(loaded),
                    vec![Operand::IdRef(stale_pointer)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_byte),
                    Some(stale_vector_pointer),
                    vec![
                        Operand::IdRef(remapped_root),
                        Operand::IdRef(zero),
                        Operand::IdRef(one),
                        Operand::IdRef(offset16),
                    ],
                ),
                Instruction::new(
                    Op::Load,
                    Some(float4),
                    Some(loaded_vector),
                    vec![Operand::IdRef(stale_vector_pointer)],
                ),
                Instruction::new(Op::Undef, Some(ptr_byte), Some(stale_float_pointer), vec![]),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(loaded_float),
                    vec![Operand::IdRef(stale_float_pointer)],
                ),
            ],
        );
        ctx.emit_sidecar.buffer_access_offsets.extend([
            crate::emit_sidecar::BufferAccessOffset {
                id: stale_pointer,
                root: remapped_root,
                byte_offset: 149,
            },
            crate::emit_sidecar::BufferAccessOffset {
                id: stale_vector_pointer,
                root: remapped_root,
                byte_offset: 16,
            },
            crate::emit_sidecar::BufferAccessOffset {
                id: stale_float_pointer,
                root: remapped_root,
                byte_offset: 20,
            },
        ]);
        ctx.module.types_global_values.append(&mut ctx.new_globals);

        crate::passes::resources::rewrites::rewrite_exact_raw_word_loads(&mut ctx, 0);

        let body = &ctx.module.functions[0].blocks[0].instructions;
        let uint_load = body
            .iter()
            .find(|instruction| {
                instruction.class.opcode == Op::Load && instruction.result_type == Some(uint)
            })
            .expect("containing uint word load");
        let word_pointer = uint_load
            .operands
            .first()
            .and_then(|operand| match operand {
                Operand::IdRef(id) => Some(*id),
                _ => None,
            })
            .expect("word pointer");
        let word_chain = body
            .iter()
            .find(|instruction| instruction.result_id == Some(word_pointer))
            .expect("word access chain");
        assert_eq!(word_chain.class.opcode, Op::AccessChain);
        assert_eq!(word_chain.result_type, Some(ptr_uint));
        assert_eq!(
            word_chain
                .operands
                .get(2)
                .and_then(|operand| match operand {
                    Operand::IdRef(id) => Some(*id),
                    _ => None,
                })
                .and_then(|id| const_u32(&ctx, id)),
            Some(37)
        );
        let shift = body
            .iter()
            .find(|instruction| instruction.class.opcode == Op::ShiftRightLogical)
            .expect("byte lane shift");
        assert_eq!(
            shift
                .operands
                .get(1)
                .and_then(|operand| match operand {
                    Operand::IdRef(id) => Some(*id),
                    _ => None,
                })
                .and_then(|id| const_u32(&ctx, id)),
            Some(8)
        );
        let reconstructed = body
            .iter()
            .find(|instruction| instruction.result_id == Some(loaded))
            .expect("preserved byte result");
        assert_eq!(reconstructed.class.opcode, Op::UConvert);
        assert_eq!(reconstructed.result_type, Some(byte));
        let reconstructed_vector = body
            .iter()
            .find(|instruction| instruction.result_id == Some(loaded_vector))
            .expect("preserved vector result");
        assert_eq!(reconstructed_vector.class.opcode, Op::CompositeConstruct);
        assert_eq!(reconstructed_vector.result_type, Some(float4));
        assert_eq!(reconstructed_vector.operands.len(), 4);
        let reconstructed_float = body
            .iter()
            .find(|instruction| instruction.result_id == Some(loaded_float))
            .expect("preserved float result");
        assert_eq!(reconstructed_float.class.opcode, Op::Bitcast);
        assert_eq!(reconstructed_float.result_type, Some(float));
    }

    #[test]
    fn affine_raw_word_fact_replays_dynamic_float_vector_load() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let byte = ty_uint8(&mut ctx);
        let float3 = ctx.ty_vecf(3);
        let words = ctx.ty_runtime_array(uint);
        let block = ctx.get_or_create(Op::TypeStruct, None, vec![Operand::IdRef(words)]);
        let ptr_block = ctx.ty_ptr(StorageClass::StorageBuffer, block);
        let ptr_float3 = ctx.ty_ptr(StorageClass::StorageBuffer, float3);
        let local_struct = ctx.get_or_create(Op::TypeStruct, None, vec![Operand::IdRef(float3)]);
        let ptr_local_struct = ctx.ty_ptr(StorageClass::Function, local_struct);
        let ptr_local_float3 = ctx.ty_ptr(StorageClass::Function, float3);
        let ptr_byte = ctx.ty_ptr(StorageClass::StorageBuffer, byte);
        let root = storage_buffer_var(&mut ctx, ptr_block);
        let carrier = ctx.module.fresh_id();
        let local_root = ctx.module.fresh_id();
        let index = ctx.module.fresh_id();
        let stale_pointer = ctx.module.fresh_id();
        let parallel_stale_pointer = ctx.module.fresh_id();
        let stale_byte_pointer = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        let loaded_byte = ctx.module.fresh_id();
        let zero = ctx.const_uint(0);
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(ptr_block), Some(carrier), vec![]),
                Instruction::new(
                    Op::Variable,
                    Some(ptr_local_struct),
                    Some(local_root),
                    vec![Operand::StorageClass(StorageClass::Function)],
                ),
                Instruction::new(Op::Undef, Some(uint), Some(index), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_float3),
                    Some(stale_pointer),
                    vec![
                        Operand::IdRef(root),
                        Operand::IdRef(zero),
                        Operand::IdRef(index),
                    ],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_local_float3),
                    Some(parallel_stale_pointer),
                    vec![
                        Operand::IdRef(local_root),
                        Operand::IdRef(zero),
                        Operand::IdRef(index),
                    ],
                ),
                Instruction::new(Op::Undef, Some(ptr_byte), Some(stale_byte_pointer), vec![]),
                Instruction::new(
                    Op::Load,
                    Some(float3),
                    Some(loaded),
                    vec![Operand::IdRef(stale_pointer)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(byte),
                    Some(loaded_byte),
                    vec![Operand::IdRef(stale_byte_pointer)],
                ),
            ],
        );
        ctx.emit_sidecar.buffer_access_affine_offsets.push(
            crate::emit_sidecar::BufferAccessAffineOffset {
                id: stale_byte_pointer,
                root: carrier,
                constant: 11,
                terms: vec![(index, 16)],
            },
        );
        ctx.emit_sidecar.buffer_access_affine_offsets.push(
            crate::emit_sidecar::BufferAccessAffineOffset {
                id: stale_pointer,
                root: carrier,
                constant: 8,
                terms: vec![(index, 16)],
            },
        );
        ctx.emit_sidecar.buffer_access_affine_offsets.push(
            crate::emit_sidecar::BufferAccessAffineOffset {
                id: parallel_stale_pointer,
                root: carrier,
                constant: 8,
                terms: vec![(index, 16)],
            },
        );
        ctx.emit_sidecar
            .buffer_access_offsets
            .push(crate::emit_sidecar::BufferAccessOffset {
                id: carrier,
                root,
                byte_offset: 16,
            });
        ctx.module.types_global_values.append(&mut ctx.new_globals);

        crate::passes::resources::rewrites::rewrite_affine_raw_word_loads(&mut ctx, 0);

        let reconstructed = find_inst(&ctx, loaded);
        assert_eq!(reconstructed.class.opcode, Op::CompositeConstruct);
        assert_eq!(reconstructed.result_type, Some(float3));
        assert_eq!(reconstructed.operands.len(), 3);
        let reconstructed_byte = find_inst(&ctx, loaded_byte);
        assert_eq!(reconstructed_byte.class.opcode, Op::UConvert);
        assert_eq!(reconstructed_byte.result_type, Some(byte));
        let body = &ctx.module.functions[0].blocks[0].instructions;
        assert!(body
            .iter()
            .all(|instruction| instruction.result_id != Some(stale_pointer)));
        assert!(body
            .iter()
            .all(|instruction| instruction.result_id != Some(parallel_stale_pointer)));
        assert!(body
            .iter()
            .any(|instruction| instruction.class.opcode == Op::IMul));
        assert_eq!(
            body.iter()
                .filter(|instruction| {
                    instruction.class.opcode == Op::Load && instruction.result_type == Some(uint)
                })
                .count(),
            4
        );
    }

    #[test]
    fn direct_float_load_from_raw_byte_pointer_phi_replays_four_bytes() {
        let mut ctx = Ctx::new(Module::new());
        let byte = ty_uint8(&mut ctx);
        let float = ctx.ty_float();
        let ptr_byte = ctx.ty_ptr(StorageClass::StorageBuffer, byte);
        let left = ctx.module.fresh_id();
        let right = ctx.module.fresh_id();
        let pred_left = ctx.module.fresh_id();
        let pred_right = ctx.module.fresh_id();
        let pointer_phi = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(ptr_byte), Some(left), vec![]),
                Instruction::new(Op::Undef, Some(ptr_byte), Some(right), vec![]),
                Instruction::new(
                    Op::Phi,
                    Some(ptr_byte),
                    Some(pointer_phi),
                    vec![
                        Operand::IdRef(left),
                        Operand::IdRef(pred_left),
                        Operand::IdRef(right),
                        Operand::IdRef(pred_right),
                    ],
                ),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(loaded),
                    vec![Operand::IdRef(pointer_phi)],
                ),
            ],
        );

        rewrite_raw_byte_pointer_direct_loads(&mut ctx, 0);

        let body = &ctx.module.functions[0].blocks[0].instructions;
        assert_eq!(find_inst(&ctx, loaded).class.opcode, Op::CopyObject);
        assert!(body.iter().all(|instruction| {
            instruction.class.opcode != Op::Load || instruction.result_type == Some(byte)
        }));
        assert_eq!(
            body.iter()
                .filter(|instruction| instruction.class.opcode == Op::PtrAccessChain)
                .count(),
            4
        );
        assert!(body
            .iter()
            .filter(|instruction| instruction.class.opcode == Op::PtrAccessChain)
            .all(|instruction| {
                instruction.operands.first() == Some(&Operand::IdRef(pointer_phi))
            }));
    }

    #[test]
    fn direct_float_vector_load_from_raw_byte_pointer_replays_each_lane() {
        let mut ctx = Ctx::new(Module::new());
        let byte = ty_uint8(&mut ctx);
        let vector = ctx.ty_vecf(4);
        let ptr_byte = ctx.ty_ptr(StorageClass::StorageBuffer, byte);
        let pointer = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(ptr_byte), Some(pointer), vec![]),
                Instruction::new(
                    Op::Load,
                    Some(vector),
                    Some(loaded),
                    vec![Operand::IdRef(pointer)],
                ),
            ],
        );

        rewrite_raw_byte_pointer_direct_loads(&mut ctx, 0);

        let body = &ctx.module.functions[0].blocks[0].instructions;
        assert_eq!(find_inst(&ctx, loaded).class.opcode, Op::CompositeConstruct);
        assert_eq!(
            body.iter()
                .filter(|instruction| {
                    instruction.class.opcode == Op::Load && instruction.result_type == Some(byte)
                })
                .count(),
            16
        );
        assert_eq!(
            body.iter()
                .filter(|instruction| instruction.class.opcode == Op::PtrAccessChain)
                .count(),
            16
        );
    }

    #[test]
    fn direct_float_load_from_raw_byte_array_pointer_descends_to_byte_zero() {
        let mut ctx = Ctx::new(Module::new());
        let byte = ty_uint8(&mut ctx);
        let float = ctx.ty_float();
        let byte_array = ctx.get_or_create(Op::TypeRuntimeArray, None, vec![Operand::IdRef(byte)]);
        let ptr_array = ctx.ty_ptr(StorageClass::StorageBuffer, byte_array);
        let pointer = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(ptr_array), Some(pointer), vec![]),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(loaded),
                    vec![Operand::IdRef(pointer)],
                ),
            ],
        );

        rewrite_raw_byte_pointer_direct_loads(&mut ctx, 0);

        let body = &ctx.module.functions[0].blocks[0].instructions;
        assert_eq!(find_inst(&ctx, loaded).class.opcode, Op::CopyObject);
        assert!(body.iter().any(|instruction| {
            instruction.class.opcode == Op::InBoundsAccessChain
                && instruction.operands.first() == Some(&Operand::IdRef(pointer))
        }));
        assert_eq!(
            body.iter()
                .filter(|instruction| {
                    instruction.class.opcode == Op::Load && instruction.result_type == Some(byte)
                })
                .count(),
            4
        );
    }

    #[test]
    fn raw_byte_pointer_wide_load_is_idempotent() {
        let mut ctx = Ctx::new(Module::new());
        build_raw_byte_phi_wide_load(&mut ctx);

        rewrite_raw_byte_pointer_wide_loads(&mut ctx, 0);
        let after_first = ctx.module.functions[0].blocks[0].instructions.clone();
        rewrite_raw_byte_pointer_wide_loads(&mut ctx, 0);
        assert_eq!(after_first, ctx.module.functions[0].blocks[0].instructions);
    }

    #[test]
    fn raw_byte_pointer_wide_load_declines_pointer_escape() {
        let mut ctx = Ctx::new(Module::new());
        let fx = build_raw_byte_phi_wide_load(&mut ctx);
        let pointer_ty = find_inst(&ctx, fx.chain_id)
            .result_type
            .expect("fixture chain has a pointer result type");
        let escape = ctx.module.fresh_id();
        let block = &mut ctx.module.functions[0].blocks[0].instructions;
        let ret = block.pop().expect("fixture ends in return");
        block.push(Instruction::new(
            Op::CopyObject,
            Some(pointer_ty),
            Some(escape),
            vec![Operand::IdRef(fx.chain_id)],
        ));
        block.push(ret);

        rewrite_raw_byte_pointer_wide_loads(&mut ctx, 0);

        assert_eq!(
            find_inst(&ctx, fx.chain_id).class.opcode,
            Op::InBoundsAccessChain,
            "a pointer escape disqualifies the entire byte replay"
        );
        assert_eq!(
            find_inst(&ctx, fx.load_id).class.opcode,
            Op::Load,
            "the original load remains paired with the untouched pointer"
        );
    }

    #[test]
    fn workgroup_byte_pointer_half_load_and_store_replay_two_bytes() {
        let mut ctx = Ctx::new(Module::new());
        let byte = ty_uint8(&mut ctx);
        let half = ctx.ty_half();
        let bytes = ctx.ty_array(byte, 8);
        let ptr_bytes = ctx.ty_ptr(StorageClass::Workgroup, bytes);
        let ptr_byte = ctx.ty_ptr(StorageClass::Workgroup, byte);
        let base = ctx.module.fresh_id();
        let index = ctx.const_uint(3);
        let pointer = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(ptr_bytes), Some(base), vec![]),
                Instruction::new(
                    Op::AccessChain,
                    Some(ptr_byte),
                    Some(pointer),
                    vec![Operand::IdRef(base), Operand::IdRef(index)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(half),
                    Some(loaded),
                    vec![Operand::IdRef(pointer)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(pointer), Operand::IdRef(loaded)],
                ),
            ],
        );

        rewrite_reinterpret_scalar_loads(&mut ctx, 0);
        rewrite_raw_byte_pointer_wide_stores(&mut ctx, 0);

        let body = &ctx.module.functions[0].blocks[0].instructions;
        assert_eq!(find_inst(&ctx, loaded).class.opcode, Op::Bitcast);
        assert_eq!(
            body.iter()
                .filter(|instruction| {
                    instruction.class.opcode == Op::Load && instruction.result_type == Some(byte)
                })
                .count(),
            2
        );
        assert_eq!(
            body.iter()
                .filter(|instruction| instruction.class.opcode == Op::Store)
                .count(),
            2
        );
        assert!(
            body.iter()
                .filter(|instruction| {
                    instruction.class.opcode == Op::Store
                        && instruction.operands.first() == Some(&Operand::IdRef(pointer))
                })
                .count()
                == 1
        );
    }

    fn build_samewidth_reinterpret_load(ctx: &mut Ctx) -> (Word, Word, Word, Word) {
        let float = ctx.ty_float();
        let uint = ctx.ty_uint();
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let ptr = storage_buffer_var(ctx, ptr_float);
        let val_id = ctx.module.fresh_id();
        install_entry(
            ctx,
            vec![Instruction::new(
                Op::Load,
                Some(uint),
                Some(val_id),
                vec![Operand::IdRef(ptr)],
            )],
        );
        (ptr, val_id, uint, float)
    }

    #[test]
    fn reinterpret_scalar_load_samewidth_becomes_typed_load_plus_bitcast() {
        let mut ctx = Ctx::new(Module::new());
        let (_ptr, val_id, uint, float) = build_samewidth_reinterpret_load(&mut ctx);

        rewrite_reinterpret_scalar_loads(&mut ctx, 0);

        let cast = find_inst(&ctx, val_id);
        assert_eq!(
            cast.class.opcode,
            Op::Bitcast,
            "same-width reinterpret → bitcast"
        );
        assert_eq!(
            cast.result_type,
            Some(uint),
            "result keeps the loaded (uint) type"
        );
        let Operand::IdRef(lo) = cast.operands[0] else {
            panic!("bitcast source is not an id");
        };
        let lo_inst = find_inst(&ctx, lo);
        assert_eq!(
            lo_inst.class.opcode,
            Op::Load,
            "slot is loaded in its declared type"
        );
        assert_eq!(
            lo_inst.result_type,
            Some(float),
            "the declared-type load reads float"
        );
    }

    #[test]
    fn reinterpret_scalar_load_is_idempotent() {
        let mut ctx = Ctx::new(Module::new());
        build_samewidth_reinterpret_load(&mut ctx);

        rewrite_reinterpret_scalar_loads(&mut ctx, 0);
        let after_first = ctx.module.functions[0].blocks[0].instructions.clone();
        rewrite_reinterpret_scalar_loads(&mut ctx, 0);
        assert_eq!(
            after_first, ctx.module.functions[0].blocks[0].instructions,
            "second application is a no-op"
        );
    }

    #[test]
    fn scalar_pointer_arithmetic_flips_samewidth_chain_to_ptr_access_chain() {
        let mut ctx = Ctx::new(Module::new());
        let float = ctx.ty_float();
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let base = storage_buffer_var(&mut ctx, ptr_float);
        let idx = ctx.const_uint(3);
        let chain_id = ctx.module.fresh_id();
        let ops = vec![Operand::IdRef(base), Operand::IdRef(idx)];
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_float),
                Some(chain_id),
                ops.clone(),
            )],
        );

        rewrite_scalar_pointer_arithmetic_access_chains(&mut ctx, 0);

        let inst = only_inst(&ctx);
        assert_eq!(inst.class.opcode, Op::PtrAccessChain, "opcode flipped");
        assert_eq!(inst.result_type, Some(ptr_float), "result type preserved");
        assert_eq!(inst.result_id, Some(chain_id), "result id preserved");
        assert_eq!(inst.operands, ops, "base + index operands unchanged");
    }

    #[test]
    fn scalar_pointer_arithmetic_is_idempotent() {
        let mut ctx = Ctx::new(Module::new());
        let float = ctx.ty_float();
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let base = storage_buffer_var(&mut ctx, ptr_float);
        let idx = ctx.const_uint(3);
        let chain_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_float),
                Some(chain_id),
                vec![Operand::IdRef(base), Operand::IdRef(idx)],
            )],
        );

        rewrite_scalar_pointer_arithmetic_access_chains(&mut ctx, 0);
        let after_first = ctx.module.functions[0].blocks[0].instructions.clone();
        rewrite_scalar_pointer_arithmetic_access_chains(&mut ctx, 0);
        assert_eq!(
            after_first, ctx.module.functions[0].blocks[0].instructions,
            "second application leaves the block byte-identical"
        );
    }

    #[test]
    fn scalar_pointer_arithmetic_leaves_composite_descent_untouched() {
        let mut ctx = Ctx::new(Module::new());
        let float = ctx.ty_float();
        let rt = ctx.ty_runtime_array(float);
        let ptr_rt = ctx.ty_ptr(StorageClass::StorageBuffer, rt);
        let ptr_float = ctx.ty_ptr(StorageClass::StorageBuffer, float);
        let base = storage_buffer_var(&mut ctx, ptr_rt);
        let idx = ctx.const_uint(0);
        let chain_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_float),
                Some(chain_id),
                vec![Operand::IdRef(base), Operand::IdRef(idx)],
            )],
        );

        rewrite_scalar_pointer_arithmetic_access_chains(&mut ctx, 0);

        assert_eq!(
            only_inst(&ctx).class.opcode,
            Op::InBoundsAccessChain,
            "an aggregate-indexing chain (base type != result type) is not rewritten"
        );
    }

    #[test]
    fn scalar_pointer_arithmetic_leaves_same_typed_struct_chain_untouched() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let structure = ctx.get_or_create(Op::TypeStruct, None, vec![Operand::IdRef(uint)]);
        let ptr_structure = ctx.ty_ptr(StorageClass::StorageBuffer, structure);
        let base = storage_buffer_var(&mut ctx, ptr_structure);
        let provisional_index = ctx.const_uint(4096);
        let chain_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_structure),
                Some(chain_id),
                vec![Operand::IdRef(base), Operand::IdRef(provisional_index)],
            )],
        );

        rewrite_scalar_pointer_arithmetic_access_chains(&mut ctx, 0);

        assert_eq!(
            only_inst(&ctx).class.opcode,
            Op::InBoundsAccessChain,
            "aggregate indexing must remain available to later layout remapping"
        );
    }

    #[test]
    fn scalar_pointer_arithmetic_declines_private_storage() {
        let mut ctx = Ctx::new(Module::new());
        let float = ctx.ty_float();
        let ptr_float_priv = ctx.ty_ptr(StorageClass::Private, float);
        let base = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::Variable,
            Some(ptr_float_priv),
            Some(base),
            vec![Operand::StorageClass(StorageClass::Private)],
        ));
        let idx = ctx.const_uint(3);
        let chain_id = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_float_priv),
                Some(chain_id),
                vec![Operand::IdRef(base), Operand::IdRef(idx)],
            )],
        );

        rewrite_scalar_pointer_arithmetic_access_chains(&mut ctx, 0);

        assert_eq!(
            only_inst(&ctx).class.opcode,
            Op::InBoundsAccessChain,
            "a Private-storage scalar chain is not turned into OpPtrAccessChain"
        );
    }

    #[test]
    fn nullable_select_access_chain_uses_concrete_arm() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let ptr = ctx.ty_ptr(StorageClass::StorageBuffer, uint);
        let concrete = storage_buffer_var(&mut ctx, ptr);
        let null = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::ConstantNull,
            Some(ptr),
            Some(null),
            vec![],
        ));
        let cond = ctx.module.fresh_id();
        let selected = ctx.module.fresh_id();
        let chain = ctx.module.fresh_id();
        let index = ctx.const_uint(2);
        let bool_ty = ctx.ty_bool();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(bool_ty), Some(cond), vec![]),
                Instruction::new(
                    Op::Select,
                    Some(ptr),
                    Some(selected),
                    vec![
                        Operand::IdRef(cond),
                        Operand::IdRef(null),
                        Operand::IdRef(concrete),
                    ],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr),
                    Some(chain),
                    vec![Operand::IdRef(selected), Operand::IdRef(index)],
                ),
            ],
        );

        expose_nullable_memory_bases(&mut ctx, 0);

        assert_eq!(
            find_inst(&ctx, chain).operands.first(),
            Some(&Operand::IdRef(concrete))
        );
    }

    #[test]
    fn nullable_phi_direct_load_uses_its_only_concrete_arm() {
        let mut ctx = Ctx::new(Module::new());
        let byte = ctx.ty_int8();
        let ptr = ctx.ty_ptr(StorageClass::StorageBuffer, byte);
        let concrete = storage_buffer_var(&mut ctx, ptr);
        let null = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::ConstantNull,
            Some(ptr),
            Some(null),
            vec![],
        ));
        let merged = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(
                    Op::Phi,
                    Some(ptr),
                    Some(merged),
                    vec![
                        Operand::IdRef(concrete),
                        Operand::IdRef(90),
                        Operand::IdRef(null),
                        Operand::IdRef(91),
                    ],
                ),
                Instruction::new(
                    Op::Load,
                    Some(byte),
                    Some(loaded),
                    vec![Operand::IdRef(merged)],
                ),
            ],
        );

        expose_nullable_memory_bases(&mut ctx, 0);

        assert_eq!(
            find_inst(&ctx, loaded).operands.first(),
            Some(&Operand::IdRef(concrete))
        );
        assert!(
            ctx.module.functions[0].blocks[0]
                .instructions
                .iter()
                .all(|instruction| instruction.result_id != Some(merged)),
            "the unused pointer phi must not retain its null arm"
        );
    }

    #[test]
    fn a_nullable_arm_that_does_not_dominate_the_use_keeps_its_merge() {
        let mut ctx = Ctx::new(Module::new());
        let byte = ctx.ty_int8();
        let ptr = ctx.ty_ptr(StorageClass::StorageBuffer, byte);
        let base = storage_buffer_var(&mut ctx, ptr);
        let bool_ty = ctx.ty_bool();
        let index = ctx.const_uint(2);
        let null = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::ConstantNull,
            Some(ptr),
            Some(null),
            vec![],
        ));
        let cond = ctx.module.fresh_id();
        let concrete = ctx.module.fresh_id();
        let merged = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        let (entry, left, right, join) = (
            ctx.module.fresh_id(),
            ctx.module.fresh_id(),
            ctx.module.fresh_id(),
            ctx.module.fresh_id(),
        );
        let func_id = ctx.module.fresh_id();
        let block = |label: Word, instructions: Vec<Instruction>| Block {
            label: Some(Instruction::new(Op::Label, None, Some(label), vec![])),
            instructions,
        };
        ctx.module.functions.push(Function {
            def: Some(Instruction::new(Op::Function, None, Some(func_id), vec![])),
            end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
            parameters: vec![],
            blocks: vec![
                block(
                    entry,
                    vec![
                        Instruction::new(Op::Undef, Some(bool_ty), Some(cond), vec![]),
                        Instruction::new(
                            Op::SelectionMerge,
                            None,
                            None,
                            vec![
                                Operand::IdRef(join),
                                Operand::SelectionControl(spirv::SelectionControl::NONE),
                            ],
                        ),
                        Instruction::new(
                            Op::BranchConditional,
                            None,
                            None,
                            vec![
                                Operand::IdRef(cond),
                                Operand::IdRef(left),
                                Operand::IdRef(right),
                            ],
                        ),
                    ],
                ),
                block(
                    left,
                    vec![
                        Instruction::new(
                            Op::InBoundsAccessChain,
                            Some(ptr),
                            Some(concrete),
                            vec![Operand::IdRef(base), Operand::IdRef(index)],
                        ),
                        Instruction::new(Op::Branch, None, None, vec![Operand::IdRef(join)]),
                    ],
                ),
                block(
                    right,
                    vec![Instruction::new(
                        Op::Branch,
                        None,
                        None,
                        vec![Operand::IdRef(join)],
                    )],
                ),
                block(
                    join,
                    vec![
                        Instruction::new(
                            Op::Phi,
                            Some(ptr),
                            Some(merged),
                            vec![
                                Operand::IdRef(concrete),
                                Operand::IdRef(left),
                                Operand::IdRef(null),
                                Operand::IdRef(right),
                            ],
                        ),
                        Instruction::new(
                            Op::Load,
                            Some(byte),
                            Some(loaded),
                            vec![Operand::IdRef(merged)],
                        ),
                        Instruction::new(Op::Return, None, None, vec![]),
                    ],
                ),
            ],
        });

        expose_nullable_memory_bases(&mut ctx, 0);

        assert_eq!(
            find_inst_anywhere(&ctx, loaded).operands.first(),
            Some(&Operand::IdRef(merged)),
            "the load must keep reading the merge, which is the only operand defined on both paths"
        );
        assert!(
            ctx.module.functions[0].blocks[3]
                .instructions
                .iter()
                .any(|instruction| instruction.result_id == Some(merged)),
            "a merge that is still read must not be retired"
        );
    }

    #[test]
    fn a_nullable_arm_that_dominates_the_use_is_still_substituted() {
        let mut ctx = Ctx::new(Module::new());
        let byte = ctx.ty_int8();
        let ptr = ctx.ty_ptr(StorageClass::StorageBuffer, byte);
        let base = storage_buffer_var(&mut ctx, ptr);
        let index = ctx.const_uint(2);
        let null = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::ConstantNull,
            Some(ptr),
            Some(null),
            vec![],
        ));
        let concrete = ctx.module.fresh_id();
        let merged = ctx.module.fresh_id();
        let loaded = ctx.module.fresh_id();
        let (entry, join) = (ctx.module.fresh_id(), ctx.module.fresh_id());
        let func_id = ctx.module.fresh_id();
        let block = |label: Word, instructions: Vec<Instruction>| Block {
            label: Some(Instruction::new(Op::Label, None, Some(label), vec![])),
            instructions,
        };
        ctx.module.functions.push(Function {
            def: Some(Instruction::new(Op::Function, None, Some(func_id), vec![])),
            end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
            parameters: vec![],
            blocks: vec![
                block(
                    entry,
                    vec![
                        Instruction::new(
                            Op::InBoundsAccessChain,
                            Some(ptr),
                            Some(concrete),
                            vec![Operand::IdRef(base), Operand::IdRef(index)],
                        ),
                        Instruction::new(Op::Branch, None, None, vec![Operand::IdRef(join)]),
                    ],
                ),
                block(
                    join,
                    vec![
                        Instruction::new(
                            Op::Phi,
                            Some(ptr),
                            Some(merged),
                            vec![
                                Operand::IdRef(concrete),
                                Operand::IdRef(entry),
                                Operand::IdRef(null),
                                Operand::IdRef(entry),
                            ],
                        ),
                        Instruction::new(
                            Op::Load,
                            Some(byte),
                            Some(loaded),
                            vec![Operand::IdRef(merged)],
                        ),
                        Instruction::new(Op::Return, None, None, vec![]),
                    ],
                ),
            ],
        });

        expose_nullable_memory_bases(&mut ctx, 0);

        assert_eq!(
            find_inst_anywhere(&ctx, loaded).operands.first(),
            Some(&Operand::IdRef(concrete)),
            "the entry block dominates the join, so the arm is available at the load"
        );
    }

    #[test]
    fn thread_local_aggregate_prefix_store_descends_and_reinterprets_scalar() {
        let mut ctx = Ctx::new(Module::new());
        let half = ctx.ty_half();
        let ushort = ctx.ty_int16();
        let pair = ctx.get_or_create(
            Op::TypeStruct,
            None,
            vec![Operand::IdRef(half), Operand::IdRef(half)],
        );
        let ptr_pair = ctx.ty_ptr(StorageClass::Private, pair);
        let source = ctx.module.fresh_id();
        ctx.new_globals.push(Instruction::new(
            Op::Variable,
            Some(ptr_pair),
            Some(source),
            vec![Operand::StorageClass(StorageClass::Private)],
        ));
        let alias = ctx.module.fresh_id();
        let object = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(
                    Op::CopyObject,
                    Some(ptr_pair),
                    Some(alias),
                    vec![Operand::IdRef(source)],
                ),
                Instruction::new(Op::Undef, Some(ushort), Some(object), vec![]),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(alias), Operand::IdRef(object)],
                ),
            ],
        );

        rewrite_thread_local_aggregate_prefix_stores(&mut ctx, 0);

        let body = &ctx.module.functions[0].blocks[0].instructions;
        let leaf_pointer = body
            .iter()
            .find(|instruction| instruction.class.opcode == Op::InBoundsAccessChain)
            .and_then(|instruction| instruction.result_id)
            .expect("first-field pointer");
        let cast = body
            .iter()
            .find(|instruction| instruction.class.opcode == Op::Bitcast)
            .expect("equal-width integer-to-half bitcast");
        assert_eq!(cast.result_type, Some(half));
        assert!(body.iter().any(|instruction| {
            instruction.class.opcode == Op::Store
                && instruction.operands.first() == Some(&Operand::IdRef(leaf_pointer))
                && instruction.operands.get(1) == cast.result_id.map(Operand::IdRef).as_ref()
        }));
    }

    #[test]
    fn raw_byte_wide_store_splits_a_vector_into_one_store_per_byte() {
        let mut ctx = Ctx::new(Module::new());
        let uchar = ctx.ty_int8();
        let float3 = ctx.ty_vecf(3);
        let ptr_uchar = ctx.ty_ptr(StorageClass::StorageBuffer, uchar);
        let pointer = storage_buffer_var(&mut ctx, ptr_uchar);
        let value = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(float3), Some(value), vec![]),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(pointer), Operand::IdRef(value)],
                ),
            ],
        );

        rewrite_raw_byte_pointer_wide_stores(&mut ctx, 0);

        let insts = &ctx.module.functions[0].blocks[0].instructions;
        let stores = insts
            .iter()
            .filter(|inst| inst.class.opcode == Op::Store)
            .count();
        assert_eq!(stores, 12, "three float lanes are twelve bytes");
        assert_eq!(
            insts
                .iter()
                .filter(|inst| inst.class.opcode == Op::CompositeExtract)
                .count(),
            3,
            "one extract per lane"
        );
        let offsets = insts
            .iter()
            .filter(|inst| inst.class.opcode == Op::PtrAccessChain)
            .filter_map(|inst| match inst.operands.get(1) {
                Some(Operand::IdRef(offset)) => const_u32(&ctx, *offset),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            offsets,
            (1..12).collect::<Vec<_>>(),
            "byte 0 reuses the pointer; the rest walk 1..11 with no gap between lanes"
        );
    }

    #[test]
    fn block_view_offset_fold_adds_the_element_index_into_the_view() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let uchar = ctx.ty_int8();
        let rt = ctx.ty_runtime_array(uint);
        let struct_id = ctx.get_or_create(Op::TypeStruct, None, vec![Operand::IdRef(rt)]);
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_uchar = ctx.ty_ptr(StorageClass::StorageBuffer, uchar);
        let base = storage_buffer_var(&mut ctx, ptr_struct);
        let dyn_idx = ctx.module.fresh_id();
        let view = ctx.module.fresh_id();
        let byte3 = ctx.const_uint(3);
        let offset_chain = ctx.module.fresh_id();
        let byte0 = ctx.module.fresh_id();
        let byte3_value = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(dyn_idx), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_uchar),
                    Some(view),
                    vec![Operand::IdRef(base), Operand::IdRef(dyn_idx)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(uchar),
                    Some(byte0),
                    vec![Operand::IdRef(view)],
                ),
                Instruction::new(
                    Op::PtrAccessChain,
                    Some(ptr_uchar),
                    Some(offset_chain),
                    vec![Operand::IdRef(view), Operand::IdRef(byte3)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(uchar),
                    Some(byte3_value),
                    vec![Operand::IdRef(offset_chain)],
                ),
            ],
        );

        fold_block_view_element_offsets(&mut ctx, 0);

        let folded = find_inst(&ctx, offset_chain);
        assert_eq!(folded.class.opcode, Op::InBoundsAccessChain);
        assert_eq!(
            folded.operands[0],
            Operand::IdRef(base),
            "rooted at the buffer"
        );
        let Operand::IdRef(index) = folded.operands[1] else {
            panic!("folded chain index is not an id");
        };
        let sum = find_inst(&ctx, index);
        assert_eq!(sum.class.opcode, Op::IAdd);
        assert_eq!(sum.operands[0], Operand::IdRef(dyn_idx));
        assert_eq!(sum.operands[1], Operand::IdRef(byte3));

        rewrite_dynamic_struct_index_subword_reinterpret(&mut ctx, 0).unwrap();
        let insts = &ctx.module.functions[0].blocks[0].instructions;
        assert!(
            !insts
                .iter()
                .any(|inst| inst.class.opcode == Op::PtrAccessChain),
            "no byte pointer survives"
        );
        assert!(
            insts.iter().all(|inst| !matches!(
                inst.class.opcode,
                Op::InBoundsAccessChain | Op::AccessChain
            ) || inst.operands.len() == 3),
            "every surviving chain descends member-0 into the word array"
        );
    }

    #[test]
    fn block_view_offset_fold_leaves_a_valid_base_alone() {
        let mut ctx = Ctx::new(Module::new());
        let uint = ctx.ty_uint();
        let uchar = ctx.ty_int8();
        let rt = ctx.ty_runtime_array(uint);
        let struct_id = ctx.get_or_create(Op::TypeStruct, None, vec![Operand::IdRef(rt)]);
        let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
        let ptr_uchar = ctx.ty_ptr(StorageClass::StorageBuffer, uchar);
        let base = storage_buffer_var(&mut ctx, ptr_struct);
        let zero = ctx.const_uint(0);
        let dyn_idx = ctx.module.fresh_id();
        let valid = ctx.module.fresh_id();
        let byte1 = ctx.const_uint(1);
        let offset_chain = ctx.module.fresh_id();
        install_entry(
            &mut ctx,
            vec![
                Instruction::new(Op::Undef, Some(uint), Some(dyn_idx), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_uchar),
                    Some(valid),
                    vec![
                        Operand::IdRef(base),
                        Operand::IdRef(zero),
                        Operand::IdRef(dyn_idx),
                    ],
                ),
                Instruction::new(
                    Op::PtrAccessChain,
                    Some(ptr_uchar),
                    Some(offset_chain),
                    vec![Operand::IdRef(valid), Operand::IdRef(byte1)],
                ),
            ],
        );

        fold_block_view_element_offsets(&mut ctx, 0);

        let unchanged = find_inst(&ctx, offset_chain);
        assert_eq!(unchanged.class.opcode, Op::PtrAccessChain);
        assert_eq!(unchanged.operands[0], Operand::IdRef(valid));
    }

    #[test]
    fn a_view_rewrite_drops_an_alignment_hint_but_not_a_volatile_access() {
        fn load_with(memory: Vec<Operand>) -> (Ctx, Word, Word) {
            let mut ctx = Ctx::new(Module::new());
            let uint = ctx.ty_uint();
            let uchar = ctx.ty_int8();
            let rt = ctx.ty_runtime_array(uint);
            let struct_id = ctx.get_or_create(Op::TypeStruct, None, vec![Operand::IdRef(rt)]);
            let ptr_struct = ctx.ty_ptr(StorageClass::StorageBuffer, struct_id);
            let ptr_uchar = ctx.ty_ptr(StorageClass::StorageBuffer, uchar);
            let base = storage_buffer_var(&mut ctx, ptr_struct);
            let dyn_idx = ctx.module.fresh_id();
            let view = ctx.module.fresh_id();
            let loaded = ctx.module.fresh_id();
            let mut load_operands = vec![Operand::IdRef(view)];
            load_operands.extend(memory);
            install_entry(
                &mut ctx,
                vec![
                    Instruction::new(Op::Undef, Some(uint), Some(dyn_idx), vec![]),
                    Instruction::new(
                        Op::InBoundsAccessChain,
                        Some(ptr_uchar),
                        Some(view),
                        vec![Operand::IdRef(base), Operand::IdRef(dyn_idx)],
                    ),
                    Instruction::new(Op::Load, Some(uchar), Some(loaded), load_operands),
                ],
            );
            rewrite_dynamic_struct_index_subword_reinterpret(&mut ctx, 0).unwrap();
            (ctx, view, loaded)
        }

        let (aligned, view, _) = load_with(vec![
            Operand::MemoryAccess(spirv::MemoryAccess::ALIGNED),
            Operand::LiteralBit32(1),
        ]);
        assert!(
            !aligned.module.functions[0].blocks[0]
                .instructions
                .iter()
                .any(|inst| inst.result_id == Some(view)),
            "an Aligned hint does not stop the replay"
        );

        let (volatile, view, _) =
            load_with(vec![Operand::MemoryAccess(spirv::MemoryAccess::VOLATILE)]);
        assert!(
            volatile.module.functions[0].blocks[0]
                .instructions
                .iter()
                .any(|inst| inst.result_id == Some(view)),
            "a Volatile access keeps the view out of the rewrite"
        );
    }
}
