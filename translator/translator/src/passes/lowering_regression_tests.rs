use crate::float16::f32_to_f16_bits;
use crate::passes::access::*;
use crate::passes::air_calls::conversions::is_i1_type_token;
use crate::passes::resources::rewrites::{
    rewrite_private_pointer_atomics, rewrite_private_zero_root_loads,
};
use crate::passes::value_result_type;
use crate::spirv_module::Operand;
use crate::spirv_module::{Block, Function, Instruction, Module, ModuleHeader};
use spirv::{Decoration, FunctionControl, Op, StorageClass};

fn ctx_state(ctx: &crate::passes::Ctx) -> String {
    format!(
        "{:?}",
        (
            &ctx.module.functions,
            &ctx.module.types_global_values,
            &ctx.module.annotations,
            &ctx.new_globals,
        )
    )
}

fn run_idempotent(ctx: &mut crate::passes::Ctx, mut pass: impl FnMut(&mut crate::passes::Ctx)) {
    pass(ctx);
    let after_first = ctx_state(ctx);
    pass(ctx);
    assert_eq!(
        after_first,
        ctx_state(ctx),
        "pass is not idempotent: a second run mutated the module"
    );
}

#[test]
fn i1_token_detection() {
    assert!(is_i1_type_token("i1"));
    assert!(is_i1_type_token("v3i1"));
    assert!(!is_i1_type_token("v2i16"));
    assert!(!is_i1_type_token("i12"));
    assert!(!is_i1_type_token("f32"));
    assert!(!is_i1_type_token("i32"));
}

#[test]
fn f16_encoding() {
    assert_eq!(f32_to_f16_bits(0.0), 0x0000);
    assert_eq!(f32_to_f16_bits(1.0), 0x3c00);
}

#[test]
fn remap_word_index_to_struct_member_rewrites_oob_word_to_member() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let float = 2;
    let v4float = 3;
    let struct_inner = 4;
    let struct_outer = 5;
    let ptr_sb_outer = 6;
    let ptr_sb_uint = 7;
    let buf = 8;
    let uint_0 = 9;
    let uint_10 = 10;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeVector,
            None,
            Some(v4float),
            vec![Operand::IdRef(float), Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_inner),
            vec![
                Operand::IdRef(v4float),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_outer),
            vec![
                Operand::IdRef(struct_inner),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_outer),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(struct_outer),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_uint),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_sb_outer),
            Some(buf),
            vec![Operand::StorageClass(StorageClass::StorageBuffer)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_10),
            vec![Operand::LiteralBit32(10)],
        ),
    ];

    let offsets = [0u32, 16, 20, 24, 28, 32, 36, 40];
    for (m, off) in offsets.iter().enumerate() {
        module.annotations.push(Instruction::new(
            Op::MemberDecorate,
            None,
            None,
            vec![
                Operand::IdRef(struct_inner),
                Operand::LiteralBit32(m as u32),
                Operand::Decoration(Decoration::Offset),
                Operand::LiteralBit32(*off),
            ],
        ));
    }

    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_sb_uint),
                Some(60),
                vec![
                    Operand::IdRef(buf),
                    Operand::IdRef(uint_0),
                    Operand::IdRef(uint_10),
                ],
            )],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| remap_word_index_to_struct_member(c, 0));

    let chain = &ctx.module.functions[0].blocks[0].instructions[0];
    let Some(Operand::IdRef(last)) = chain.operands.last() else {
        panic!("chain lost its trailing index");
    };
    let cval = ctx
        .new_globals
        .iter()
        .chain(ctx.module.types_global_values.iter())
        .find(|g| g.result_id == Some(*last) && g.class.opcode == Op::Constant)
        .and_then(|g| match g.operands.first() {
            Some(Operand::LiteralBit32(v)) => Some(*v),
            _ => None,
        });
    assert_eq!(
        cval,
        Some(7),
        "word index 10 should remap to member index 7"
    );
}

#[test]
fn remap_overflow_word_index_collapses_to_outer_sibling_member() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let float = 2;
    let v4float = 3;
    let struct_inner = 4;
    let struct_outer = 5;
    let ptr_sb_outer = 6;
    let ptr_sb_uint = 7;
    let buf = 8;
    let uint_0 = 9;
    let uint_14 = 10;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeVector,
            None,
            Some(v4float),
            vec![Operand::IdRef(float), Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_inner),
            vec![
                Operand::IdRef(v4float),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_outer),
            vec![
                Operand::IdRef(struct_inner),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_outer),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(struct_outer),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_uint),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_sb_outer),
            Some(buf),
            vec![Operand::StorageClass(StorageClass::StorageBuffer)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_14),
            vec![Operand::LiteralBit32(14)],
        ),
    ];

    let inner_offsets = [0u32, 16, 20, 24, 28, 32, 36, 40];
    for (m, off) in inner_offsets.iter().enumerate() {
        module.annotations.push(Instruction::new(
            Op::MemberDecorate,
            None,
            None,
            vec![
                Operand::IdRef(struct_inner),
                Operand::LiteralBit32(m as u32),
                Operand::Decoration(Decoration::Offset),
                Operand::LiteralBit32(*off),
            ],
        ));
    }
    let outer_offsets = [0u32, 48, 52, 56];
    for (m, off) in outer_offsets.iter().enumerate() {
        module.annotations.push(Instruction::new(
            Op::MemberDecorate,
            None,
            None,
            vec![
                Operand::IdRef(struct_outer),
                Operand::LiteralBit32(m as u32),
                Operand::Decoration(Decoration::Offset),
                Operand::LiteralBit32(*off),
            ],
        ));
    }

    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_sb_uint),
                Some(60),
                vec![
                    Operand::IdRef(buf),
                    Operand::IdRef(uint_0),
                    Operand::IdRef(uint_14),
                ],
            )],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| {
        remap_overflow_word_index_to_outer_member(c, 0)
    });

    let chain = &ctx.module.functions[0].blocks[0].instructions[0];
    assert_eq!(
        chain.operands.len(),
        2,
        "chain should collapse to base + single member index"
    );
    let Some(Operand::IdRef(last)) = chain.operands.last() else {
        panic!("chain lost its trailing index");
    };
    let cval = ctx
        .new_globals
        .iter()
        .chain(ctx.module.types_global_values.iter())
        .find(|g| g.result_id == Some(*last) && g.class.opcode == Op::Constant)
        .and_then(|g| match g.operands.first() {
            Some(Operand::LiteralBit32(v)) => Some(*v),
            _ => None,
        });
    assert_eq!(
        cval,
        Some(3),
        "byte 56 (word 14) should collapse to outer member index 3"
    );
}

#[test]
fn remodel_workgroup_flatword_aggregate_flattens_to_uint_array() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let elem = 2;
    let arr = 3;
    let ptr_wg_arr = 4;
    let ptr_wg_elem = 5;
    let wgvar = 6;
    let uint_1 = 7;
    let uint_2 = 8;
    let arr_len = 9;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(elem),
            vec![Operand::IdRef(uint), Operand::IdRef(uint)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(arr_len),
            vec![Operand::LiteralBit32(3)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(arr),
            vec![Operand::IdRef(elem), Operand::IdRef(arr_len)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_wg_arr),
            vec![
                Operand::StorageClass(StorageClass::Workgroup),
                Operand::IdRef(arr),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_wg_elem),
            vec![
                Operand::StorageClass(StorageClass::Workgroup),
                Operand::IdRef(elem),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_wg_arr),
            Some(wgvar),
            vec![Operand::StorageClass(StorageClass::Workgroup)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_1),
            vec![Operand::LiteralBit32(1)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_2),
            vec![Operand::LiteralBit32(2)],
        ),
    ];

    let param = 20;
    let mul = 21;
    let word = 22;
    let chain = 23;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![Instruction::new(
            Op::FunctionParameter,
            Some(uint),
            Some(param),
            vec![],
        )],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::IMul,
                    Some(uint),
                    Some(mul),
                    vec![Operand::IdRef(param), Operand::IdRef(uint_2)],
                ),
                Instruction::new(
                    Op::IAdd,
                    Some(uint),
                    Some(word),
                    vec![Operand::IdRef(uint_1), Operand::IdRef(mul)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_wg_elem),
                    Some(chain),
                    vec![Operand::IdRef(wgvar), Operand::IdRef(word)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(chain), Operand::IdRef(param)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    remodel_workgroup_flatword_aggregate(&mut ctx, 0);

    let all_globals: Vec<&Instruction> = ctx
        .new_globals
        .iter()
        .chain(ctx.module.types_global_values.iter())
        .collect();
    let pointee_of = |ptr: u32| -> Option<u32> {
        all_globals.iter().find_map(|g| {
            if g.result_id == Some(ptr) && g.class.opcode == Op::TypePointer {
                match g.operands.get(1) {
                    Some(Operand::IdRef(p)) => Some(*p),
                    _ => None,
                }
            } else {
                None
            }
        })
    };
    let chain_inst = &ctx.module.functions[0].blocks[0].instructions[2];
    let chain_ptr = chain_inst.result_type.expect("chain has a result type");
    assert_eq!(
        pointee_of(chain_ptr),
        Some(uint),
        "chain result must point at uint after remodel"
    );
    let var_inst = all_globals
        .iter()
        .find(|g| g.class.opcode == Op::Variable && g.result_id == Some(wgvar))
        .expect("variable still present");
    let var_arr = pointee_of(var_inst.result_type.unwrap()).expect("var points at an array");
    let arr_def = all_globals
        .iter()
        .find(|g| g.result_id == Some(var_arr) && g.class.opcode == Op::TypeArray)
        .expect("variable pointee is an array");
    let Some(Operand::IdRef(arr_elem)) = arr_def.operands.first() else {
        panic!("array missing element type");
    };
    assert_eq!(*arr_elem, uint, "flat array element must be uint");
    let Some(Operand::IdRef(len_c)) = arr_def.operands.get(1) else {
        panic!("array missing length");
    };
    let len_val = all_globals
        .iter()
        .find(|g| g.result_id == Some(*len_c) && g.class.opcode == Op::Constant)
        .and_then(|g| match g.operands.first() {
            Some(Operand::LiteralBit32(v)) => Some(*v),
            _ => None,
        });
    assert_eq!(len_val, Some(6), "flat array length must be 3*2 = 6 words");
}

#[test]
fn remodel_workgroup_floatarray_atomic_as_uint_retypes_and_repoints() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let float = 2;
    let uint_ptr_wg = 3;
    let float_arr = 4;
    let ptr_wg_arr = 5;
    let ptr_wg_float = 6;
    let wgvar = 7;
    let arr_len = 8;
    let idx = 9;
    let scope = 10;
    let sem = 11;
    let val = 12;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(arr_len),
            vec![Operand::LiteralBit32(3)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(float_arr),
            vec![Operand::IdRef(float), Operand::IdRef(arr_len)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_wg_arr),
            vec![
                Operand::StorageClass(StorageClass::Workgroup),
                Operand::IdRef(float_arr),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_wg_float),
            vec![
                Operand::StorageClass(StorageClass::Workgroup),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(uint_ptr_wg),
            vec![
                Operand::StorageClass(StorageClass::Workgroup),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_wg_arr),
            Some(wgvar),
            vec![Operand::StorageClass(StorageClass::Workgroup)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(scope),
            vec![Operand::LiteralBit32(1)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(sem),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(val),
            vec![Operand::LiteralBit32(7)],
        ),
    ];

    let chain = 20;
    let cu = 21;
    let atomic = 22;
    let loaded = 23;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_wg_float),
                    Some(chain),
                    vec![Operand::IdRef(wgvar), Operand::IdRef(idx)],
                ),
                Instruction::new(
                    Op::Bitcast,
                    Some(uint_ptr_wg),
                    Some(cu),
                    vec![Operand::IdRef(chain)],
                ),
                Instruction::new(
                    Op::AtomicSMin,
                    Some(uint),
                    Some(atomic),
                    vec![
                        Operand::IdRef(cu),
                        Operand::IdRef(scope),
                        Operand::IdRef(sem),
                        Operand::IdRef(val),
                    ],
                ),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(loaded),
                    vec![Operand::IdRef(chain)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    remodel_workgroup_floatarray_atomic_as_uint(&mut ctx, 0).unwrap();
    ctx.module.types_global_values.append(&mut ctx.new_globals);
    assert!(
        !crate::native::construct_workgroup_atomic_floats_module(&mut ctx.module),
        "the memory-phase owner must leave no flat Workgroup atomic graph for a later sweep"
    );

    let all_globals: Vec<&Instruction> = ctx
        .new_globals
        .iter()
        .chain(ctx.module.types_global_values.iter())
        .collect();
    let pointee_of = |ptr: u32| -> Option<u32> {
        all_globals.iter().find_map(|g| {
            if g.result_id == Some(ptr) && g.class.opcode == Op::TypePointer {
                match g.operands.get(1) {
                    Some(Operand::IdRef(p)) => Some(*p),
                    _ => None,
                }
            } else {
                None
            }
        })
    };
    let var_ptr = all_globals
        .iter()
        .find(|g| g.result_id == Some(wgvar))
        .and_then(|g| g.result_type)
        .expect("var has a result type");
    let new_arr = pointee_of(var_ptr).expect("var points at an array");
    let new_arr_def = all_globals
        .iter()
        .find(|g| g.result_id == Some(new_arr))
        .expect("array type def");
    assert_eq!(new_arr_def.class.opcode, Op::TypeArray);
    assert_eq!(
        new_arr_def.operands.first(),
        Some(&Operand::IdRef(uint)),
        "array element must be uint after retype"
    );

    let body = &ctx.module.functions[0].blocks[0].instructions;
    let chain_inst = body
        .iter()
        .find(|i| i.result_id == Some(chain))
        .expect("chain survives");
    assert_eq!(
        pointee_of(chain_inst.result_type.unwrap()),
        Some(uint),
        "chain must point at uint after retype"
    );
    assert!(
        !body.iter().any(|i| i.result_id == Some(cu)),
        "the pointer OpBitcast must be removed"
    );
    let atomic_inst = body
        .iter()
        .find(|i| i.result_id == Some(atomic))
        .expect("atomic survives");
    assert_eq!(
        atomic_inst.operands.first(),
        Some(&Operand::IdRef(chain)),
        "atomic must point at the uint chain, not the dropped bitcast"
    );
    let load_bitcast = body
        .iter()
        .find(|i| i.class.opcode == Op::Bitcast && i.result_id == Some(loaded))
        .expect("the float load became a value bitcast");
    assert_eq!(load_bitcast.result_type, Some(float));
    let uint_load_src = match load_bitcast.operands.first() {
        Some(Operand::IdRef(s)) => *s,
        _ => panic!("bitcast source"),
    };
    let uint_load = body
        .iter()
        .find(|i| i.class.opcode == Op::Load && i.result_id == Some(uint_load_src))
        .expect("a uint load feeds the value bitcast");
    assert_eq!(uint_load.result_type, Some(uint));
    assert_eq!(uint_load.operands.first(), Some(&Operand::IdRef(chain)));
}

#[test]
fn remodel_workgroup_floatarray_atomic_as_signed_int_add_retypes_and_repoints() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let float = 2;
    let int_ptr_wg = 3;
    let float_arr = 4;
    let ptr_wg_arr = 5;
    let ptr_wg_float = 6;
    let wgvar = 7;
    let arr_len = 8;
    let idx = 9;
    let scope = 10;
    let sem = 11;
    let val = 12;
    let int_s = 13;
    let f0 = 14;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(int_s),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(1)],
        ),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(arr_len),
            vec![Operand::LiteralBit32(3)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(float_arr),
            vec![Operand::IdRef(float), Operand::IdRef(arr_len)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_wg_arr),
            vec![
                Operand::StorageClass(StorageClass::Workgroup),
                Operand::IdRef(float_arr),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_wg_float),
            vec![
                Operand::StorageClass(StorageClass::Workgroup),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(int_ptr_wg),
            vec![
                Operand::StorageClass(StorageClass::Workgroup),
                Operand::IdRef(int_s),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_wg_arr),
            Some(wgvar),
            vec![Operand::StorageClass(StorageClass::Workgroup)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(scope),
            vec![Operand::LiteralBit32(1)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(sem),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(int_s),
            Some(val),
            vec![Operand::LiteralBit32(7)],
        ),
        Instruction::new(
            Op::Constant,
            Some(float),
            Some(f0),
            vec![Operand::LiteralBit32(0)],
        ),
    ];

    let chain = 20;
    let cu = 21;
    let atomic = 22;
    let histload = 23;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_wg_float),
                    Some(chain),
                    vec![Operand::IdRef(wgvar), Operand::IdRef(idx)],
                ),
                Instruction::new(
                    Op::Bitcast,
                    Some(int_ptr_wg),
                    Some(cu),
                    vec![Operand::IdRef(chain)],
                ),
                Instruction::new(
                    Op::AtomicIAdd,
                    Some(int_s),
                    Some(atomic),
                    vec![
                        Operand::IdRef(cu),
                        Operand::IdRef(scope),
                        Operand::IdRef(sem),
                        Operand::IdRef(val),
                    ],
                ),
                Instruction::new(
                    Op::Load,
                    Some(int_s),
                    Some(histload),
                    vec![Operand::IdRef(cu)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(chain), Operand::IdRef(f0)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    remodel_workgroup_floatarray_atomic_as_uint(&mut ctx, 0).unwrap();

    let all_globals: Vec<&Instruction> = ctx
        .new_globals
        .iter()
        .chain(ctx.module.types_global_values.iter())
        .collect();
    let pointee_of = |ptr: u32| -> Option<u32> {
        all_globals.iter().find_map(|g| {
            if g.result_id == Some(ptr) && g.class.opcode == Op::TypePointer {
                match g.operands.get(1) {
                    Some(Operand::IdRef(p)) => Some(*p),
                    _ => None,
                }
            } else {
                None
            }
        })
    };
    let var_ptr = all_globals
        .iter()
        .find(|g| g.result_id == Some(wgvar))
        .and_then(|g| g.result_type)
        .expect("var has a result type");
    let new_arr = pointee_of(var_ptr).expect("var points at an array");
    let new_arr_def = all_globals
        .iter()
        .find(|g| g.result_id == Some(new_arr))
        .expect("array type def");
    assert_eq!(new_arr_def.class.opcode, Op::TypeArray);
    assert_eq!(
        new_arr_def.operands.first(),
        Some(&Operand::IdRef(int_s)),
        "array element must be the SIGNED int after retype"
    );

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert!(
        !body.iter().any(|i| i.result_id == Some(cu)),
        "the pointer OpBitcast must be removed"
    );
    let atomic_inst = body
        .iter()
        .find(|i| i.result_id == Some(atomic))
        .expect("atomic survives");
    assert_eq!(
        atomic_inst.operands.first(),
        Some(&Operand::IdRef(chain)),
        "atomic must point at the int chain"
    );
    let hist_inst = body
        .iter()
        .find(|i| i.result_id == Some(histload))
        .expect("histogram load survives");
    assert_eq!(hist_inst.class.opcode, Op::Load);
    assert_eq!(hist_inst.result_type, Some(int_s));
    assert_eq!(
        hist_inst.operands.first(),
        Some(&Operand::IdRef(chain)),
        "the plain int load must be repointed natively at the int chain"
    );
    let store = body
        .iter()
        .find(|i| i.class.opcode == Op::Store)
        .expect("the store survives");
    let stored = match store.operands.get(1) {
        Some(Operand::IdRef(s)) => *s,
        _ => panic!("store object"),
    };
    let store_bitcast = body
        .iter()
        .find(|i| i.class.opcode == Op::Bitcast && i.result_id == Some(stored))
        .expect("the float store object became an int value bitcast");
    assert_eq!(store_bitcast.result_type, Some(int_s));
    assert_eq!(store_bitcast.operands.first(), Some(&Operand::IdRef(f0)));
}

#[test]
fn rewrite_scalar_slot_array_overindex_lowers_union_element_load() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let float = 2;
    let ulong = 3;
    let ptr_func_ulong = 4;
    let ptr_func_float = 5;
    let uint_1 = 6;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(ulong),
            vec![Operand::LiteralBit32(64), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_func_ulong),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(ulong),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_func_float),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_1),
            vec![Operand::LiteralBit32(1)],
        ),
    ];

    let slot = 60;
    let chain = 61;
    let loaded = 62;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::Variable,
                    Some(ptr_func_ulong),
                    Some(slot),
                    vec![Operand::StorageClass(StorageClass::Function)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_func_float),
                    Some(chain),
                    vec![Operand::IdRef(slot), Operand::IdRef(uint_1)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(loaded),
                    vec![Operand::IdRef(chain)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| {
        rewrite_scalar_slot_array_overindex(c, 0).unwrap();
    });

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert!(
        !body
            .iter()
            .any(|i| matches!(i.class.opcode, Op::InBoundsAccessChain | Op::AccessChain)),
        "the scalar over-index chain should be removed"
    );
    assert!(
        body.iter().any(|i| i.class.opcode == Op::ShiftRightLogical),
        "element 1 should be read via a right shift of the whole slot"
    );
    assert!(
        body.iter().any(|i| i.class.opcode == Op::UConvert),
        "the shifted slot should be truncated to 32 bits"
    );
    assert!(
        body.iter().any(|i| i.result_id == Some(loaded)),
        "the load result id should survive the rewrite"
    );
}

#[test]
fn rewrite_scalar_slot_array_overindex_scales_in_dynamic_index_type() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let float = 2;
    let ulong = 3;
    let ptr_func_ulong = 4;
    let ptr_func_float = 5;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(ulong),
            vec![Operand::LiteralBit32(64), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_func_ulong),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(ulong),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_func_float),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(float),
            ],
        ),
    ];

    let dynamic_ulong_index = 60;
    let slot = 61;
    let chain = 62;
    let loaded = 63;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![Instruction::new(
            Op::FunctionParameter,
            Some(ulong),
            Some(dynamic_ulong_index),
            vec![],
        )],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::Variable,
                    Some(ptr_func_ulong),
                    Some(slot),
                    vec![Operand::StorageClass(StorageClass::Function)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_func_float),
                    Some(chain),
                    vec![Operand::IdRef(slot), Operand::IdRef(dynamic_ulong_index)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(loaded),
                    vec![Operand::IdRef(chain)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| {
        rewrite_scalar_slot_array_overindex(c, 0).unwrap();
    });

    let body = &ctx.module.functions[0].blocks[0].instructions;
    let scale = body
        .iter()
        .find(|instruction| instruction.class.opcode == Op::IMul)
        .expect("dynamic packed-slot index is scaled");
    assert_eq!(
        scale.result_type,
        Some(ulong),
        "the scale must retain the dynamic access-chain index width"
    );
    assert_eq!(scale.operands[0], Operand::IdRef(dynamic_ulong_index));
    let factor = operand_id(scale, 1).expect("scale factor");
    assert_eq!(value_result_type(&ctx, factor), Some(ulong));
    assert_eq!(const_i64_value(&ctx, factor), Some(32));
}

#[test]
fn rewrite_scalar_pointer_arithmetic_promotes_self_typed_inbounds_to_ptr_access_chain() {
    let float = 1;
    let ptr_sb_float = 2;
    let uint = 3;
    let idx = 4;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_float),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx),
            vec![Operand::LiteralBit32(5)],
        ),
    ];

    let base = 50;
    let chain = 60;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![Instruction::new(
            Op::FunctionParameter,
            Some(ptr_sb_float),
            Some(base),
            vec![],
        )],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_sb_float),
                Some(chain),
                vec![Operand::IdRef(base), Operand::IdRef(idx)],
            )],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| {
        rewrite_scalar_pointer_arithmetic_access_chains(c, 0)
    });

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert!(
        !body
            .iter()
            .any(|i| i.class.opcode == Op::InBoundsAccessChain),
        "the self-typed scalar InBoundsAccessChain should be promoted"
    );
    let promoted = body
        .iter()
        .find(|i| i.result_id == Some(chain))
        .expect("the chain result id survives the rewrite");
    assert_eq!(
        promoted.class.opcode,
        Op::PtrAccessChain,
        "promoted to OpPtrAccessChain"
    );
    assert_eq!(
        promoted.result_type,
        Some(ptr_sb_float),
        "result pointer type is preserved"
    );
    assert_eq!(
        promoted.operands,
        vec![Operand::IdRef(base), Operand::IdRef(idx)],
        "base + index operands are preserved"
    );
}

#[test]
fn rewrite_scalar_pointer_arithmetic_leaves_genuine_aggregate_descent_alone() {
    let float = 1;
    let arr4 = 2;
    let ptr_sb_arr = 3;
    let ptr_sb_float = 4;
    let uint = 5;
    let idx = 6;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx),
            vec![Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(arr4),
            vec![Operand::IdRef(float), Operand::IdRef(idx)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_arr),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(arr4),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_float),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
    ];

    let base = 50;
    let chain = 60;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![Instruction::new(
            Op::FunctionParameter,
            Some(ptr_sb_arr),
            Some(base),
            vec![],
        )],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_sb_float),
                Some(chain),
                vec![Operand::IdRef(base), Operand::IdRef(idx)],
            )],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    rewrite_scalar_pointer_arithmetic_access_chains(&mut ctx, 0);

    let chain_inst = ctx.module.functions[0].blocks[0]
        .instructions
        .iter()
        .find(|i| i.result_id == Some(chain))
        .expect("the chain survives");
    assert_eq!(
        chain_inst.class.opcode,
        Op::InBoundsAccessChain,
        "genuine aggregate descent must NOT be promoted to OpPtrAccessChain"
    );
}

#[test]
fn rewrite_reinterpret_scalar_loads_same_width_splits_into_declared_load_plus_bitcast() {
    let float = 1;
    let uint = 2;
    let ptr_sb_float = 3;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_float),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
    ];

    let ptr = 50;
    let loaded = 60;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![Instruction::new(
            Op::FunctionParameter,
            Some(ptr_sb_float),
            Some(ptr),
            vec![],
        )],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::Load,
                Some(uint),
                Some(loaded),
                vec![Operand::IdRef(ptr)],
            )],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| rewrite_reinterpret_scalar_loads(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert!(
        !body
            .iter()
            .any(|i| i.class.opcode == Op::Load && i.result_type == Some(uint)),
        "the mistyped uint load should be gone"
    );
    let decl_load = body
        .iter()
        .find(|i| i.class.opcode == Op::Load && i.result_type == Some(float))
        .expect("a load in the declared float pointee type is emitted");
    assert_ne!(
        decl_load.result_id,
        Some(loaded),
        "the declared-type load gets a fresh id, not the original load id"
    );
    let bitcast = body
        .iter()
        .find(|i| i.result_id == Some(loaded))
        .expect("the original load result id survives");
    assert_eq!(
        bitcast.class.opcode,
        Op::Bitcast,
        "the result id now names the reinterpret bitcast"
    );
    assert_eq!(
        bitcast.result_type,
        Some(uint),
        "bitcast yields the uint value"
    );
    assert_eq!(
        bitcast.operands,
        vec![Operand::IdRef(decl_load.result_id.unwrap())],
        "the bitcast reinterprets the declared-type load"
    );
}

#[test]
fn rewrite_strided_descent_promotes_overindexed_array_chain_to_ptr_access_chain() {
    let float = 1;
    let uint = 2;
    let len4 = 3;
    let arr = 4;
    let ptr_sb_arr = 5;
    let ptr_sb_float = 6;
    let idx0 = 7;
    let idx1 = 8;
    let make_module = || {
        let mut module = Module::new();
        module.header = Some(ModuleHeader::new(100));
        module.types_global_values = vec![
            Instruction::new(
                Op::TypeFloat,
                None,
                Some(float),
                vec![Operand::LiteralBit32(32)],
            ),
            Instruction::new(
                Op::TypeInt,
                None,
                Some(uint),
                vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
            ),
            Instruction::new(
                Op::Constant,
                Some(uint),
                Some(len4),
                vec![Operand::LiteralBit32(4)],
            ),
            Instruction::new(
                Op::TypeArray,
                None,
                Some(arr),
                vec![Operand::IdRef(float), Operand::IdRef(len4)],
            ),
            Instruction::new(
                Op::TypePointer,
                None,
                Some(ptr_sb_arr),
                vec![
                    Operand::StorageClass(StorageClass::StorageBuffer),
                    Operand::IdRef(arr),
                ],
            ),
            Instruction::new(
                Op::TypePointer,
                None,
                Some(ptr_sb_float),
                vec![
                    Operand::StorageClass(StorageClass::StorageBuffer),
                    Operand::IdRef(float),
                ],
            ),
            Instruction::new(
                Op::Constant,
                Some(uint),
                Some(idx0),
                vec![Operand::LiteralBit32(0)],
            ),
            Instruction::new(
                Op::Constant,
                Some(uint),
                Some(idx1),
                vec![Operand::LiteralBit32(2)],
            ),
        ];
        module
    };

    let base = 50;
    let chain = 60;
    let mut module = make_module();
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![Instruction::new(
            Op::FunctionParameter,
            Some(ptr_sb_arr),
            Some(base),
            vec![],
        )],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_sb_float),
                Some(chain),
                vec![
                    Operand::IdRef(base),
                    Operand::IdRef(idx0),
                    Operand::IdRef(idx1),
                ],
            )],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });
    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| rewrite_strided_descent_access_chains(c, 0));
    let promoted = ctx.module.functions[0].blocks[0]
        .instructions
        .iter()
        .find(|i| i.result_id == Some(chain))
        .expect("chain survives");
    assert_eq!(
        promoted.class.opcode,
        Op::PtrAccessChain,
        "over-indexed array chain promoted to OpPtrAccessChain"
    );
    assert_eq!(
        promoted.operands,
        vec![
            Operand::IdRef(base),
            Operand::IdRef(idx0),
            Operand::IdRef(idx1),
        ],
        "base + stride + descent operands preserved"
    );

    let mut module = make_module();
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![Instruction::new(
            Op::FunctionParameter,
            Some(ptr_sb_arr),
            Some(base),
            vec![],
        )],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_sb_float),
                Some(chain),
                vec![Operand::IdRef(base), Operand::IdRef(idx1)],
            )],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });
    let mut ctx = crate::passes::Ctx::new(module);
    rewrite_strided_descent_access_chains(&mut ctx, 0);
    let valid = ctx.module.functions[0].blocks[0]
        .instructions
        .iter()
        .find(|i| i.result_id == Some(chain))
        .expect("chain survives");
    assert_eq!(
        valid.class.opcode,
        Op::InBoundsAccessChain,
        "a valid single-index descent must NOT be promoted"
    );
}

#[test]
fn compose_derived_access_chains_rebases_linear_stream_offsets() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(1),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeVector,
            None,
            Some(2),
            vec![Operand::IdRef(1), Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(3),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(2),
            ],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(4),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(4),
            Some(5),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(4),
            Some(6),
            vec![Operand::LiteralBit32(7)],
        ),
        Instruction::new(
            Op::Constant,
            Some(4),
            Some(7),
            vec![Operand::LiteralBit32(1)],
        ),
        Instruction::new(
            Op::Variable,
            Some(3),
            Some(30),
            vec![Operand::StorageClass(StorageClass::StorageBuffer)],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(1),
            Some(10),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(11),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(12), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(3),
                    Some(20),
                    vec![Operand::IdRef(30), Operand::IdRef(5), Operand::IdRef(6)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(3),
                    Some(21),
                    vec![Operand::IdRef(20), Operand::IdRef(7)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| compose_derived_access_chains(c, 0));

    let inst = ctx.module.functions[0].blocks[0]
        .instructions
        .iter()
        .find(|inst| inst.result_id == Some(21))
        .expect("composed access chain");
    assert_eq!(inst.operands[0], Operand::IdRef(30));
    assert_eq!(inst.operands[1], Operand::IdRef(5));
    let Operand::IdRef(composed) = inst.operands[2] else {
        panic!("composed index should be an id");
    };
    assert_eq!(const_i64_value(&ctx, composed), Some(8));
}

#[test]
fn hoist_function_variables_moves_them_to_entry_front() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(1),
            Some(10),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(11),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![
            Block {
                label: Some(Instruction::new(Op::Label, None, Some(12), vec![])),
                instructions: vec![
                    Instruction::new(Op::Load, Some(1), Some(20), vec![Operand::IdRef(30)]),
                    Instruction::new(
                        Op::Variable,
                        Some(2),
                        Some(21),
                        vec![Operand::StorageClass(StorageClass::Function)],
                    ),
                ],
            },
            Block {
                label: Some(Instruction::new(Op::Label, None, Some(13), vec![])),
                instructions: vec![Instruction::new(
                    Op::Variable,
                    Some(2),
                    Some(22),
                    vec![Operand::StorageClass(StorageClass::Function)],
                )],
            },
        ],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| hoist_function_variables(c, 0));

    let first = &ctx.module.functions[0].blocks[0].instructions;
    assert_eq!(first[0].class.opcode, Op::Variable);
    assert_eq!(first[1].class.opcode, Op::Variable);
    assert_eq!(first[2].class.opcode, Op::Load);
    assert!(ctx.module.functions[0].blocks[1].instructions.is_empty());
}

#[test]
fn lower_private_memory_atomics_uses_plain_load_store() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(1),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(2),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(1),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(1),
            Some(3),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Variable,
            Some(2),
            Some(4),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(3),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(1),
            Some(5),
            vec![Operand::LiteralBit32(1)],
        ),
        Instruction::new(
            Op::Constant,
            Some(1),
            Some(6),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(1),
            Some(7),
            vec![Operand::LiteralBit32(0xff)],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(1),
            Some(10),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(11),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(12), vec![])),
            instructions: vec![Instruction::new(
                Op::AtomicAnd,
                Some(1),
                Some(20),
                vec![
                    Operand::IdRef(4),
                    Operand::IdScope(5),
                    Operand::IdMemorySemantics(6),
                    Operand::IdRef(7),
                ],
            )],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| lower_private_memory_atomics(c, 0));

    let insts = &ctx.module.functions[0].blocks[0].instructions;
    assert_eq!(insts.len(), 3);
    assert_eq!(insts[0].class.opcode, Op::Load);
    assert_eq!(insts[0].result_id, Some(20));
    assert_eq!(insts[1].class.opcode, Op::BitwiseAnd);
    assert_eq!(insts[2].class.opcode, Op::Store);
    assert!(!insts.iter().any(|inst| inst.class.opcode == Op::AtomicAnd));
}

#[test]
fn lower_reinterpreted_private_atomic_bitcasts_values_not_pointer() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    let uint = 1;
    let float = 2;
    let bool_ty = 3;
    let ptr_private_float = 4;
    let ptr_private_uint = 5;
    let null_float = 6;
    let private_float = 7;
    let scope = 8;
    let semantics = 9;
    let value = 10;
    let private_uint = 11;
    let null_uint = 12;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(Op::TypeBool, None, Some(bool_ty), vec![]),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_private_float),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_private_uint),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(Op::ConstantNull, Some(float), Some(null_float), vec![]),
        Instruction::new(
            Op::Variable,
            Some(ptr_private_float),
            Some(private_float),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(null_float),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(scope),
            vec![Operand::LiteralBit32(1)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(semantics),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(value),
            vec![Operand::LiteralBit32(7)],
        ),
        Instruction::new(Op::ConstantNull, Some(uint), Some(null_uint), vec![]),
        Instruction::new(
            Op::Variable,
            Some(ptr_private_uint),
            Some(private_uint),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(null_uint),
            ],
        ),
    ];
    let pointer_bitcast = 20;
    let atomic_result = 21;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::Bitcast,
                    Some(ptr_private_uint),
                    Some(pointer_bitcast),
                    vec![Operand::IdRef(private_float)],
                ),
                Instruction::new(
                    Op::AtomicSMin,
                    Some(uint),
                    Some(atomic_result),
                    vec![
                        Operand::IdRef(pointer_bitcast),
                        Operand::IdScope(scope),
                        Operand::IdMemorySemantics(semantics),
                        Operand::IdRef(value),
                    ],
                ),
            ],
        }],
    });
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(60),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(61), vec![])),
            instructions: vec![Instruction::new(
                Op::AtomicLoad,
                Some(uint),
                Some(62),
                vec![
                    Operand::IdRef(private_uint),
                    Operand::IdScope(scope),
                    Operand::IdMemorySemantics(semantics),
                ],
            )],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, rewrite_private_pointer_atomics);

    let instructions = &ctx.module.functions[0].blocks[0].instructions;
    assert!(!instructions
        .iter()
        .any(|inst| inst.class.opcode == Op::AtomicSMin));
    assert!(!instructions.iter().any(|inst| {
        inst.class.opcode == Op::Bitcast && inst.result_type == Some(ptr_private_uint)
    }));
    assert_eq!(instructions[0].class.opcode, Op::Load);
    assert_eq!(instructions[0].result_type, Some(float));
    assert_eq!(instructions[1].class.opcode, Op::Bitcast);
    assert_eq!(instructions[1].result_type, Some(uint));
    assert_eq!(instructions[1].result_id, Some(atomic_result));
    let store = instructions.last().expect("lowered store");
    assert_eq!(store.class.opcode, Op::Store);
    assert_eq!(store.operands.first(), Some(&Operand::IdRef(private_float)));
    assert_eq!(
        ctx.module.functions[1].blocks[0].instructions[0]
            .class
            .opcode,
        Op::Load,
        "a helper function sharing the Private root must be lowered too"
    );
}

#[test]
fn absent_private_aggregate_root_load_through_helper_becomes_typed_zero() {
    let uint = 1;
    let length = 2;
    let array = 3;
    let pointer = 4;
    let initializer = 5;
    let root = 6;
    let alias = 7;
    let result = 8;
    let entry_function = 9;
    let helper_function = 10;
    let helper_parameter = 11;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(64));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(length),
            vec![Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(array),
            vec![Operand::IdRef(uint), Operand::IdRef(length)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(pointer),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(array),
            ],
        ),
        Instruction::new(Op::ConstantNull, Some(array), Some(initializer), vec![]),
        Instruction::new(
            Op::Variable,
            Some(pointer),
            Some(root),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(initializer),
            ],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(entry_function),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(60),
            ],
        )),
        end: None,
        parameters: vec![],
        blocks: vec![Block {
            label: None,
            instructions: vec![Instruction::new(
                Op::FunctionCall,
                Some(uint),
                Some(61),
                vec![Operand::IdRef(helper_function), Operand::IdRef(root)],
            )],
        }],
    });
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(helper_function),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(62),
            ],
        )),
        end: None,
        parameters: vec![Instruction::new(
            Op::FunctionParameter,
            Some(pointer),
            Some(helper_parameter),
            vec![],
        )],
        blocks: vec![Block {
            label: None,
            instructions: vec![
                Instruction::new(
                    Op::CopyObject,
                    Some(pointer),
                    Some(alias),
                    vec![Operand::IdRef(helper_parameter)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(uint),
                    Some(result),
                    vec![Operand::IdRef(alias)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |ctx| {
        rewrite_private_zero_root_loads(ctx, &[root])
    });

    let rewritten = &ctx.module.functions[1].blocks[0].instructions[1];
    assert_eq!(rewritten.class.opcode, Op::CopyObject);
    assert_eq!(rewritten.result_type, Some(uint));
    assert_eq!(rewritten.result_id, Some(result));
    let Operand::IdRef(zero) = rewritten.operands[0] else {
        panic!("rewritten load must copy a null constant")
    };
    assert!(ctx.new_globals.iter().any(|instruction| {
        instruction.class.opcode == Op::ConstantNull
            && instruction.result_type == Some(uint)
            && instruction.result_id == Some(zero)
    }));
}

#[test]
fn decorate_ptr_access_chain_base_strides_adds_stride_per_pointee() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(Op::TypeVoid, None, Some(1), vec![]),
        Instruction::new(Op::TypeFunction, None, Some(2), vec![Operand::IdRef(1)]),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(3),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(4),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(4),
            Some(5),
            vec![Operand::LiteralBit32(7)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(6),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(3),
            ],
        ),
        Instruction::new(
            Op::TypeVector,
            None,
            Some(7),
            vec![Operand::IdRef(3), Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(8),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(7),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(9),
            vec![
                Operand::StorageClass(StorageClass::Workgroup),
                Operand::IdRef(3),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(11),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(3),
            ],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(1),
            Some(10),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(2),
            ],
        )),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(20), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::PtrAccessChain,
                    Some(6),
                    Some(30),
                    vec![Operand::IdRef(40), Operand::IdRef(5)],
                ),
                Instruction::new(
                    Op::PtrAccessChain,
                    Some(8),
                    Some(31),
                    vec![Operand::IdRef(41), Operand::IdRef(5)],
                ),
                Instruction::new(
                    Op::PtrAccessChain,
                    Some(9),
                    Some(32),
                    vec![Operand::IdRef(42), Operand::IdRef(5)],
                ),
                Instruction::new(
                    Op::PtrAccessChain,
                    Some(11),
                    Some(33),
                    vec![Operand::IdRef(43), Operand::IdRef(5)],
                ),
                Instruction::new(Op::Return, None, None, vec![]),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    decorate_ptr_access_chain_base_strides(&mut ctx);

    let stride_of = |ctx: &crate::passes::Ctx, ty: u32| -> Vec<u32> {
        ctx.module
            .annotations
            .iter()
            .filter(|a| {
                a.class.opcode == Op::Decorate
                    && a.operands.first() == Some(&Operand::IdRef(ty))
                    && a.operands.get(1) == Some(&Operand::Decoration(Decoration::ArrayStride))
            })
            .filter_map(|a| match a.operands.get(2) {
                Some(Operand::LiteralBit32(s)) => Some(*s),
                _ => None,
            })
            .collect()
    };
    assert_eq!(stride_of(&ctx, 6), vec![4], "float pointer stride");
    assert_eq!(stride_of(&ctx, 8), vec![16], "<4 x float> pointer stride");
    assert!(
        stride_of(&ctx, 9).is_empty(),
        "Workgroup pointer types cannot carry explicit layout decorations"
    );
    assert!(
        stride_of(&ctx, 11).is_empty(),
        "Function pointer types cannot carry explicit layout decorations"
    );

    decorate_ptr_access_chain_base_strides(&mut ctx);
    assert_eq!(stride_of(&ctx, 6), vec![4], "no duplicate after re-run");
    assert_eq!(stride_of(&ctx, 8), vec![16], "no duplicate after re-run");
}

#[test]
fn lower_cross_member_subword_load_assembles_spanned_bytes() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uchar = 1;
    let ushort = 2;
    let uint = 3;
    let struct_inner = 4;
    let struct_outer = 5;
    let ptr_sb_outer = 6;
    let ptr_sb_uchar = 7;
    let buf = 8;
    let uint_0 = 9;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uchar),
            vec![Operand::LiteralBit32(8), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(ushort),
            vec![Operand::LiteralBit32(16), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_inner),
            vec![Operand::IdRef(uchar), Operand::IdRef(uchar)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_outer),
            vec![Operand::IdRef(struct_inner)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_outer),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(struct_outer),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_uchar),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uchar),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_sb_outer),
            Some(buf),
            vec![Operand::StorageClass(StorageClass::StorageBuffer)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_0),
            vec![Operand::LiteralBit32(0)],
        ),
    ];
    for (m, off) in [0u32, 1].iter().enumerate() {
        module.annotations.push(Instruction::new(
            Op::MemberDecorate,
            None,
            None,
            vec![
                Operand::IdRef(struct_inner),
                Operand::LiteralBit32(m as u32),
                Operand::Decoration(Decoration::Offset),
                Operand::LiteralBit32(*off),
            ],
        ));
    }
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_sb_uchar),
                    Some(60),
                    vec![
                        Operand::IdRef(buf),
                        Operand::IdRef(uint_0),
                        Operand::IdRef(uint_0),
                    ],
                ),
                Instruction::new(Op::Load, Some(ushort), Some(61), vec![Operand::IdRef(60)]),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| {
        lower_cross_member_subword_load(c, 0).unwrap();
    });

    let insts = &ctx.module.functions[0].blocks[0].instructions;
    assert!(
        !insts
            .iter()
            .any(|i| i.class.opcode == Op::Load && i.result_type == Some(ushort)),
        "the ushort load through the uchar pointer must be replaced"
    );
    assert!(
        insts.iter().any(|i| i.class.opcode == Op::ShiftLeftLogical),
        "high byte must be shifted into place"
    );
    assert!(
        insts.iter().any(|i| i.class.opcode == Op::BitwiseOr),
        "the two bytes must be ORed"
    );
    assert_eq!(
        insts
            .iter()
            .filter(|i| i.class.opcode == Op::Load && i.result_type == Some(uchar))
            .count(),
        2,
        "one load per spanned member"
    );
    assert!(
        insts.iter().any(|i| i.result_id == Some(61)),
        "the load result id must survive as the assembled value"
    );
}

#[test]
fn lower_cross_member_subword_store_splits_into_members() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let ulong = 2;
    let struct_inner = 3;
    let struct_outer = 4;
    let ptr_sb_outer = 5;
    let ptr_sb_uint = 6;
    let buf = 7;
    let uint_0 = 8;
    let obj = 9;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(ulong),
            vec![Operand::LiteralBit32(64), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_inner),
            vec![Operand::IdRef(uint), Operand::IdRef(uint)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_outer),
            vec![Operand::IdRef(struct_inner)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_outer),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(struct_outer),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_uint),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_sb_outer),
            Some(buf),
            vec![Operand::StorageClass(StorageClass::StorageBuffer)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(ulong),
            Some(obj),
            vec![Operand::LiteralBit32(7), Operand::LiteralBit32(0)],
        ),
    ];
    for (m, off) in [0u32, 4].iter().enumerate() {
        module.annotations.push(Instruction::new(
            Op::MemberDecorate,
            None,
            None,
            vec![
                Operand::IdRef(struct_inner),
                Operand::LiteralBit32(m as u32),
                Operand::Decoration(Decoration::Offset),
                Operand::LiteralBit32(*off),
            ],
        ));
    }
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_sb_uint),
                    Some(60),
                    vec![
                        Operand::IdRef(buf),
                        Operand::IdRef(uint_0),
                        Operand::IdRef(uint_0),
                    ],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(60), Operand::IdRef(obj)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| lower_cross_member_subword_store(c, 0));

    let insts = &ctx.module.functions[0].blocks[0].instructions;
    assert_eq!(
        insts.iter().filter(|i| i.class.opcode == Op::Store).count(),
        2,
        "the ulong store splits into two uint stores"
    );
    assert!(
        insts
            .iter()
            .any(|i| i.class.opcode == Op::ShiftRightLogical),
        "the high word must be shifted down"
    );
    assert!(
        insts.iter().any(|i| i.class.opcode == Op::UConvert),
        "each 32-bit word must be narrowed from the 64-bit object"
    );
    assert!(
        !insts.iter().any(|i| matches!(
            i.operands.get(1),
            Some(Operand::IdRef(id)) if *id == obj
        ) && i.class.opcode == Op::Store),
        "the original ulong object is no longer stored directly"
    );
}

#[test]
fn lower_private_byte_aggregate_reinterpret_splits_v2half_store() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let half = 1;
    let uint = 2;
    let uchar = 3;
    let v2half = 4;
    let arr2 = 5;
    let ptr_priv_arr = 6;
    let ptr_priv_half = 7;
    let ptr_priv_uchar = 8;
    let uint_0 = 9;
    let uint_2 = 10;
    let uint_4 = 11;
    let var = 12;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(half),
            vec![Operand::LiteralBit32(16)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uchar),
            vec![Operand::LiteralBit32(8), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeVector,
            None,
            Some(v2half),
            vec![Operand::IdRef(half), Operand::LiteralBit32(2)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(arr2),
            vec![Operand::IdRef(half), Operand::IdRef(uint_2)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_priv_arr),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(arr2),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_priv_half),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(half),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_priv_uchar),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(uchar),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_2),
            vec![Operand::LiteralBit32(2)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_4),
            vec![Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_priv_arr),
            Some(var),
            vec![Operand::StorageClass(StorageClass::Private)],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(Op::Undef, Some(v2half), Some(60), vec![]),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_priv_half),
                    Some(61),
                    vec![Operand::IdRef(var), Operand::IdRef(uint_0)],
                ),
                Instruction::new(
                    Op::PtrAccessChain,
                    Some(ptr_priv_uchar),
                    Some(62),
                    vec![Operand::IdRef(61), Operand::IdRef(uint_4)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(62), Operand::IdRef(60)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    lower_private_byte_aggregate_reinterpret(&mut ctx, 0).unwrap();

    let insts = &ctx.module.functions[0].blocks[0].instructions;
    assert!(
        !insts.iter().any(|i| i.class.opcode == Op::PtrAccessChain),
        "the uchar byte PtrAccessChain must be removed"
    );
    let elem_store_indices: Vec<u32> = insts
        .iter()
        .filter(|i| {
            i.class.opcode == Op::InBoundsAccessChain
                && i.operands.first() == Some(&Operand::IdRef(var))
        })
        .filter_map(|i| match i.operands.get(1) {
            Some(Operand::IdRef(c)) => ctx
                .module
                .types_global_values
                .iter()
                .chain(ctx.new_globals.iter())
                .find(|g| g.result_id == Some(*c))
                .and_then(|g| match g.operands.first() {
                    Some(Operand::LiteralBit32(v)) => Some(*v),
                    _ => None,
                }),
            _ => None,
        })
        .collect();
    assert!(
        elem_store_indices.contains(&2) && elem_store_indices.contains(&3),
        "expected per-element accesses at indices 2 and 3, got {elem_store_indices:?}"
    );
    assert_eq!(
        insts.iter().filter(|i| i.class.opcode == Op::Store).count(),
        2,
        "the v2half store must split into two half stores"
    );

    let var_ptr = ctx
        .module
        .types_global_values
        .iter()
        .chain(ctx.new_globals.iter())
        .find(|g| g.result_id == Some(var))
        .and_then(|g| g.result_type)
        .expect("variable retyped");
    let new_arr = ctx
        .module
        .types_global_values
        .iter()
        .chain(ctx.new_globals.iter())
        .find(|g| g.result_id == Some(var_ptr))
        .and_then(|g| match g.operands.get(1) {
            Some(Operand::IdRef(p)) => Some(*p),
            _ => None,
        })
        .expect("pointer pointee");
    let arr_len_c = ctx
        .module
        .types_global_values
        .iter()
        .chain(ctx.new_globals.iter())
        .find(|g| g.result_id == Some(new_arr) && g.class.opcode == Op::TypeArray)
        .and_then(|g| match g.operands.get(1) {
            Some(Operand::IdRef(c)) => Some(*c),
            _ => None,
        })
        .expect("array length");
    let len_val = ctx
        .module
        .types_global_values
        .iter()
        .chain(ctx.new_globals.iter())
        .find(|g| g.result_id == Some(arr_len_c))
        .and_then(|g| match g.operands.first() {
            Some(Operand::LiteralBit32(v)) => Some(*v),
            _ => None,
        })
        .expect("length constant");
    assert_eq!(
        len_val, 4,
        "the array must be enlarged to hold element index 3"
    );
}

#[test]
fn retype_demoted_copymemory_placeholder_matches_source_struct() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uchar = 1;
    let float = 2;
    let v3float = 3;
    let inner = 4;
    let ptr_priv_uchar = 5;
    let ptr_func_inner = 6;
    let null_uchar = 7;
    let placeholder = 8;
    let src = 9;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uchar),
            vec![Operand::LiteralBit32(8), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeVector,
            None,
            Some(v3float),
            vec![Operand::IdRef(float), Operand::LiteralBit32(3)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(inner),
            vec![Operand::IdRef(v3float)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_priv_uchar),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(uchar),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_func_inner),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(inner),
            ],
        ),
        Instruction::new(Op::ConstantNull, Some(uchar), Some(null_uchar), vec![]),
        Instruction::new(
            Op::Variable,
            Some(ptr_priv_uchar),
            Some(placeholder),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(null_uchar),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_func_inner),
            Some(src),
            vec![Operand::StorageClass(StorageClass::Function)],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uchar),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::CopyMemory,
                None,
                None,
                vec![Operand::IdRef(placeholder), Operand::IdRef(src)],
            )],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    retype_demoted_copymemory_placeholder(&mut ctx, 0);

    let var = ctx
        .module
        .types_global_values
        .iter()
        .chain(ctx.new_globals.iter())
        .find(|g| g.result_id == Some(placeholder))
        .expect("placeholder var");
    let new_ptr = var.result_type.expect("retyped");
    let pointee = ctx
        .module
        .types_global_values
        .iter()
        .chain(ctx.new_globals.iter())
        .find(|g| g.result_id == Some(new_ptr) && g.class.opcode == Op::TypePointer)
        .and_then(|g| match g.operands.get(1) {
            Some(Operand::IdRef(p)) => Some(*p),
            _ => None,
        })
        .expect("pointer pointee");
    assert_eq!(
        pointee, inner,
        "the placeholder must point to the source struct type"
    );
    assert_eq!(
        var.operands.len(),
        1,
        "the mistyped scalar initializer must be dropped"
    );
}

#[test]
fn retype_private_direct_memory_placeholder_uses_complete_object_type() {
    let byte = 1;
    let float = 2;
    let vector = 3;
    let ptr_private_byte = 4;
    let null_byte = 5;
    let variable = 6;
    let loaded = 7;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(32));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(byte),
            vec![Operand::LiteralBit32(8), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeVector,
            None,
            Some(vector),
            vec![Operand::IdRef(float), Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_private_byte),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(byte),
            ],
        ),
        Instruction::new(Op::ConstantNull, Some(byte), Some(null_byte), vec![]),
        Instruction::new(
            Op::Variable,
            Some(ptr_private_byte),
            Some(variable),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(null_byte),
            ],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(Op::Function, None, Some(20), vec![])),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(21), vec![])),
            instructions: vec![Instruction::new(
                Op::Load,
                Some(vector),
                Some(loaded),
                vec![Operand::IdRef(variable)],
            )],
        }],
    });
    let mut ctx = crate::passes::Ctx::new(module);

    retype_private_direct_memory_placeholders(&mut ctx);

    let variable = ctx
        .module
        .types_global_values
        .iter()
        .find(|instruction| instruction.result_id == Some(variable))
        .expect("private variable");
    let pointer_ty = variable.result_type.expect("private pointer type");
    assert_eq!(
        ctx.module
            .types_global_values
            .iter()
            .find(|instruction| instruction.result_id == Some(pointer_ty))
            .and_then(|instruction| instruction.operands.get(1)),
        Some(&Operand::IdRef(vector))
    );
    let initializer = match variable.operands.get(1) {
        Some(Operand::IdRef(initializer)) => *initializer,
        _ => panic!("typed private initializer"),
    };
    assert!(ctx.module.types_global_values.iter().any(|instruction| {
        instruction.class.opcode == Op::ConstantNull
            && instruction.result_id == Some(initializer)
            && instruction.result_type == Some(vector)
    }));
    assert_eq!(
        ctx.module.functions[0].blocks[0].instructions[0].result_type,
        Some(vector)
    );
}

#[test]
fn reroot_demoted_array_element_overindex_direct() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let float = 1;
    let uint = 2;
    let uint_9 = 3;
    let arr9 = 4;
    let ptr_func_arr = 5;
    let ptr_func_float = 6;
    let arr_var = 7;
    let uint_0 = 8;
    let idx = 9;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_9),
            vec![Operand::LiteralBit32(9)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(arr9),
            vec![Operand::IdRef(float), Operand::IdRef(uint_9)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_func_arr),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(arr9),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_func_float),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx),
            vec![Operand::LiteralBit32(5)],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::Variable,
                    Some(ptr_func_arr),
                    Some(arr_var),
                    vec![Operand::StorageClass(StorageClass::Function)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_func_float),
                    Some(60),
                    vec![Operand::IdRef(arr_var), Operand::IdRef(uint_0)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_func_float),
                    Some(61),
                    vec![Operand::IdRef(60), Operand::IdRef(idx)],
                ),
                Instruction::new(Op::Load, Some(float), Some(62), vec![Operand::IdRef(61)]),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| reroot_demoted_array_element_overindex(c, 0));

    let r = ctx.module.functions[0].blocks[0]
        .instructions
        .iter()
        .find(|i| i.result_id == Some(61))
        .expect("the over-index chain must still exist");
    assert_eq!(
        r.operands,
        vec![Operand::IdRef(arr_var), Operand::IdRef(idx)],
        "the over-index must be re-rooted onto the array variable with the same index"
    );
}

#[test]
fn reroot_demoted_array_element_overindex_through_phi() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let float = 1;
    let uint = 2;
    let uint_4 = 3;
    let arr4 = 4;
    let ptr_func_arr = 5;
    let ptr_func_float = 6;
    let arr_var = 7;
    let uint_0 = 8;
    let idx = 9;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_4),
            vec![Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(arr4),
            vec![Operand::IdRef(float), Operand::IdRef(uint_4)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_func_arr),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(arr4),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_func_float),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx),
            vec![Operand::LiteralBit32(2)],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![
            Block {
                label: Some(Instruction::new(Op::Label, None, Some(70), vec![])),
                instructions: vec![
                    Instruction::new(
                        Op::Variable,
                        Some(ptr_func_arr),
                        Some(arr_var),
                        vec![Operand::StorageClass(StorageClass::Function)],
                    ),
                    Instruction::new(
                        Op::InBoundsAccessChain,
                        Some(ptr_func_float),
                        Some(80),
                        vec![Operand::IdRef(arr_var), Operand::IdRef(uint_0)],
                    ),
                    Instruction::new(Op::Branch, None, None, vec![Operand::IdRef(72)]),
                ],
            },
            Block {
                label: Some(Instruction::new(Op::Label, None, Some(71), vec![])),
                instructions: vec![
                    Instruction::new(
                        Op::InBoundsAccessChain,
                        Some(ptr_func_float),
                        Some(81),
                        vec![Operand::IdRef(arr_var), Operand::IdRef(uint_0)],
                    ),
                    Instruction::new(Op::Branch, None, None, vec![Operand::IdRef(72)]),
                ],
            },
            Block {
                label: Some(Instruction::new(Op::Label, None, Some(72), vec![])),
                instructions: vec![
                    Instruction::new(
                        Op::Phi,
                        Some(ptr_func_float),
                        Some(82),
                        vec![
                            Operand::IdRef(80),
                            Operand::IdRef(70),
                            Operand::IdRef(81),
                            Operand::IdRef(71),
                        ],
                    ),
                    Instruction::new(
                        Op::InBoundsAccessChain,
                        Some(ptr_func_float),
                        Some(83),
                        vec![Operand::IdRef(82), Operand::IdRef(idx)],
                    ),
                    Instruction::new(Op::Return, None, None, vec![]),
                ],
            },
        ],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| reroot_demoted_array_element_overindex(c, 0));

    let r = ctx.module.functions[0].blocks[2]
        .instructions
        .iter()
        .find(|i| i.result_id == Some(83))
        .expect("the over-index chain must still exist");
    assert_eq!(
        r.operands,
        vec![Operand::IdRef(arr_var), Operand::IdRef(idx)],
        "the phi-reached over-index must re-root onto the converged array variable"
    );
}

#[test]
fn remap_dynamic_word_index_collapses_to_array_member() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let inner = 2;
    let arr4 = 3;
    let outer = 4;
    let ptr_sb_outer = 5;
    let ptr_sb_uint = 6;
    let buf = 7;
    let uint_0 = 8;
    let uint_44 = 9;
    let uint_4 = 10;
    let dyn_id = 11;
    let iadd = 12;

    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(inner),
            vec![Operand::IdRef(uint)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_4),
            vec![Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(arr4),
            vec![Operand::IdRef(uint), Operand::IdRef(uint_4)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(outer),
            vec![Operand::IdRef(inner), Operand::IdRef(arr4)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_outer),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(outer),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_uint),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_sb_outer),
            Some(buf),
            vec![Operand::StorageClass(StorageClass::StorageBuffer)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_44),
            vec![Operand::LiteralBit32(44)],
        ),
    ];

    module.annotations = vec![
        Instruction::new(
            Op::MemberDecorate,
            None,
            None,
            vec![
                Operand::IdRef(outer),
                Operand::LiteralBit32(0),
                Operand::Decoration(Decoration::Offset),
                Operand::LiteralBit32(0),
            ],
        ),
        Instruction::new(
            Op::MemberDecorate,
            None,
            None,
            vec![
                Operand::IdRef(outer),
                Operand::LiteralBit32(1),
                Operand::Decoration(Decoration::Offset),
                Operand::LiteralBit32(176),
            ],
        ),
        Instruction::new(
            Op::Decorate,
            None,
            None,
            vec![
                Operand::IdRef(arr4),
                Operand::Decoration(Decoration::ArrayStride),
                Operand::LiteralBit32(4),
            ],
        ),
    ];

    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::Bitcast,
                    Some(uint),
                    Some(dyn_id),
                    vec![Operand::IdRef(uint_0)],
                ),
                Instruction::new(
                    Op::IAdd,
                    Some(uint),
                    Some(iadd),
                    vec![Operand::IdRef(uint_44), Operand::IdRef(dyn_id)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_sb_uint),
                    Some(60),
                    vec![
                        Operand::IdRef(buf),
                        Operand::IdRef(uint_0),
                        Operand::IdRef(iadd),
                    ],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| remap_dynamic_word_index_to_array_member(c, 0));

    let chain = &ctx.module.functions[0].blocks[0].instructions[2];
    assert_eq!(
        chain.operands,
        vec![
            Operand::IdRef(buf),
            chain.operands[1].clone(),
            Operand::IdRef(dyn_id),
        ],
        "chain should be %buf %uint_1 %dyn"
    );
    let Operand::IdRef(member_id) = chain.operands[1] else {
        panic!("member index not an id")
    };
    let mval = ctx
        .new_globals
        .iter()
        .chain(ctx.module.types_global_values.iter())
        .find(|g| g.result_id == Some(member_id) && g.class.opcode == Op::Constant)
        .and_then(|g| match g.operands.first() {
            Some(Operand::LiteralBit32(v)) => Some(*v),
            _ => None,
        });
    assert_eq!(mval, Some(1), "word 44 should map to array member index 1");
}

#[test]
fn remap_dynamic_word_index_to_array_struct_field_remaps_and_splits_load() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let float = 2;
    let inner = 3;
    let elem = 4;
    let arr = 5;
    let outer = 6;
    let ptr_sb_outer = 7;
    let ptr_sb_uint = 8;
    let buf = 9;
    let uint_0 = 10;
    let uint_12 = 11;
    let uint_2c = 12;
    let dyn_id = 13;
    let imul = 14;
    let iadd = 15;
    let chain = 16;
    let load = 17;

    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(inner),
            vec![Operand::IdRef(float)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(elem),
            vec![Operand::IdRef(float), Operand::IdRef(float)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_2c),
            vec![Operand::LiteralBit32(2)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(arr),
            vec![Operand::IdRef(elem), Operand::IdRef(uint_2c)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(outer),
            vec![Operand::IdRef(inner), Operand::IdRef(arr)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_outer),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(outer),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_uint),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_sb_outer),
            Some(buf),
            vec![Operand::StorageClass(StorageClass::StorageBuffer)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_12),
            vec![Operand::LiteralBit32(12)],
        ),
    ];

    module.annotations = vec![
        Instruction::new(
            Op::MemberDecorate,
            None,
            None,
            vec![
                Operand::IdRef(outer),
                Operand::LiteralBit32(0),
                Operand::Decoration(Decoration::Offset),
                Operand::LiteralBit32(0),
            ],
        ),
        Instruction::new(
            Op::MemberDecorate,
            None,
            None,
            vec![
                Operand::IdRef(outer),
                Operand::LiteralBit32(1),
                Operand::Decoration(Decoration::Offset),
                Operand::LiteralBit32(48),
            ],
        ),
        Instruction::new(
            Op::Decorate,
            None,
            None,
            vec![
                Operand::IdRef(arr),
                Operand::Decoration(Decoration::ArrayStride),
                Operand::LiteralBit32(8),
            ],
        ),
        Instruction::new(
            Op::MemberDecorate,
            None,
            None,
            vec![
                Operand::IdRef(elem),
                Operand::LiteralBit32(0),
                Operand::Decoration(Decoration::Offset),
                Operand::LiteralBit32(0),
            ],
        ),
        Instruction::new(
            Op::MemberDecorate,
            None,
            None,
            vec![
                Operand::IdRef(elem),
                Operand::LiteralBit32(1),
                Operand::Decoration(Decoration::Offset),
                Operand::LiteralBit32(4),
            ],
        ),
    ];

    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::Bitcast,
                    Some(uint),
                    Some(dyn_id),
                    vec![Operand::IdRef(uint_0)],
                ),
                Instruction::new(
                    Op::IMul,
                    Some(uint),
                    Some(imul),
                    vec![Operand::IdRef(dyn_id), Operand::IdRef(uint_2c)],
                ),
                Instruction::new(
                    Op::IAdd,
                    Some(uint),
                    Some(iadd),
                    vec![Operand::IdRef(uint_12), Operand::IdRef(imul)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_sb_uint),
                    Some(chain),
                    vec![
                        Operand::IdRef(buf),
                        Operand::IdRef(uint_0),
                        Operand::IdRef(iadd),
                    ],
                ),
                Instruction::new(
                    Op::Load,
                    Some(uint),
                    Some(load),
                    vec![Operand::IdRef(chain)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| {
        remap_dynamic_word_index_to_array_struct_field(c, 0)
    });

    let insts = &ctx.module.functions[0].blocks[0].instructions;
    let chain_inst = insts
        .iter()
        .find(|i| i.result_id == Some(chain))
        .expect("chain");
    let Some(rt) = chain_inst.result_type else {
        panic!("chain has no result type")
    };
    let pointee = ctx
        .new_globals
        .iter()
        .chain(ctx.module.types_global_values.iter())
        .find(|g| g.result_id == Some(rt) && g.class.opcode == Op::TypePointer)
        .and_then(|g| match g.operands.get(1) {
            Some(Operand::IdRef(p)) => Some(*p),
            _ => None,
        });
    assert_eq!(pointee, Some(float), "chain should be retyped to float*");
    assert_eq!(chain_inst.operands.len(), 4, "chain should be %buf M dyn F");
    assert_eq!(chain_inst.operands[0], Operand::IdRef(buf));
    assert_eq!(
        chain_inst.operands[2],
        Operand::IdRef(dyn_id),
        "element index is %dyn"
    );
    let const_val = |id: Operand| -> Option<u32> {
        let Operand::IdRef(cid) = id else { return None };
        ctx.new_globals
            .iter()
            .chain(ctx.module.types_global_values.iter())
            .find(|g| g.result_id == Some(cid) && g.class.opcode == Op::Constant)
            .and_then(|g| match g.operands.first() {
                Some(Operand::LiteralBit32(v)) => Some(*v),
                _ => None,
            })
    };
    assert_eq!(
        const_val(chain_inst.operands[1].clone()),
        Some(1),
        "member 1"
    );
    assert_eq!(
        const_val(chain_inst.operands[3].clone()),
        Some(0),
        "field 0"
    );

    let bc = insts
        .iter()
        .find(|i| i.result_id == Some(load))
        .expect("load id");
    assert_eq!(bc.class.opcode, Op::Bitcast, "uint load becomes a bitcast");
    assert_eq!(
        bc.result_type,
        Some(uint),
        "bitcast preserves uint result type"
    );
    let Operand::IdRef(src) = bc.operands[0] else {
        panic!("bitcast operand")
    };
    let fload = insts
        .iter()
        .find(|i| i.result_id == Some(src))
        .expect("float load");
    assert_eq!(fload.class.opcode, Op::Load, "inserted op is a load");
    assert_eq!(
        fload.result_type,
        Some(float),
        "inserted load reads the float field"
    );
    assert_eq!(
        fload.operands[0],
        Operand::IdRef(chain),
        "float load reads the chain"
    );
}

#[test]
fn drop_writeonly_dead_local_array_stores_removes_invalid_stores() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let ulong = 1;
    let uint = 2;
    let float = 3;
    let arr16 = 4;
    let ptr_fn_arr = 5;
    let ptr_fn_ulong = 6;
    let ptr_sb_float = 7;
    let arr_var = 8;
    let buf = 9;
    let uint_16 = 10;
    let idx0 = 11;
    let slot = 12;

    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(ulong),
            vec![Operand::LiteralBit32(64), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(uint_16),
            vec![Operand::LiteralBit32(16)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(arr16),
            vec![Operand::IdRef(ulong), Operand::IdRef(uint_16)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_fn_arr),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(arr16),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_fn_ulong),
            vec![
                Operand::StorageClass(StorageClass::Function),
                Operand::IdRef(ulong),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_float),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_sb_float),
            Some(buf),
            vec![Operand::StorageClass(StorageClass::StorageBuffer)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx0),
            vec![Operand::LiteralBit32(0)],
        ),
    ];

    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            None,
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::Variable,
                    Some(ptr_fn_arr),
                    Some(arr_var),
                    vec![Operand::StorageClass(StorageClass::Function)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_fn_ulong),
                    Some(slot),
                    vec![Operand::IdRef(arr_var), Operand::IdRef(idx0)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(slot), Operand::IdRef(buf)],
                ),
                Instruction::new(Op::Return, None, None, vec![]),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| drop_writeonly_dead_local_array_stores(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert!(
        !body.iter().any(|i| i.class.opcode == Op::Store),
        "the dead invalid store should be removed"
    );
    assert!(
        !body.iter().any(|i| i.result_id == Some(slot)),
        "the now-dead access chain should be removed"
    );
    assert!(
        body.iter().any(|i| i.result_id == Some(arr_var)),
        "the (now unused) Function variable may remain"
    );
}

#[test]
fn guard_integer_division_by_zero_inserts_denominator_guard() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let numer = 10;
    let denom = 11;
    let div_res = 20;
    module.types_global_values = vec![Instruction::new(
        Op::TypeInt,
        None,
        Some(uint),
        vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
    )];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![
            Instruction::new(Op::FunctionParameter, Some(uint), Some(numer), vec![]),
            Instruction::new(Op::FunctionParameter, Some(uint), Some(denom), vec![]),
        ],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::UDiv,
                Some(uint),
                Some(div_res),
                vec![Operand::IdRef(numer), Operand::IdRef(denom)],
            )],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    guard_integer_division_by_zero(&mut ctx, 0);

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert_eq!(
        body.len(),
        3,
        "an IEqual + Select guard is inserted before the divide"
    );
    assert_eq!(body[0].class.opcode, Op::IEqual, "denominator == 0 test");
    assert_eq!(
        body[0].operands[0],
        Operand::IdRef(denom),
        "the zero test reads the original denominator"
    );
    assert_eq!(body[1].class.opcode, Op::Select, "select 1 when denom == 0");
    let safe = body[1].result_id.expect("select has a result id");
    let div = &body[2];
    assert_eq!(div.class.opcode, Op::UDiv, "the divide itself is preserved");
    assert_eq!(div.result_id, Some(div_res));
    assert_eq!(
        div.operands[0],
        Operand::IdRef(numer),
        "numerator unchanged"
    );
    assert_eq!(
        div.operands[1],
        Operand::IdRef(safe),
        "the divide denominator rebinds to the guarded (select) value"
    );
}

#[test]
fn guard_integer_division_by_zero_skips_a_constant_nonzero_denominator() {
    let uint = 1;
    let numer = 10;
    let four = 11;
    let nil = 12;
    let vec2 = 13;
    let mixed = 14;
    let all_four = 15;

    let divide = |result: u32, denom: u32, ty: u32| {
        Instruction::new(
            Op::UDiv,
            Some(ty),
            Some(result),
            vec![Operand::IdRef(numer), Operand::IdRef(denom)],
        )
    };
    let composite = |result: u32, parts: [u32; 2]| {
        Instruction::new(
            Op::ConstantComposite,
            Some(vec2),
            Some(result),
            parts.map(Operand::IdRef).to_vec(),
        )
    };

    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeVector,
            None,
            Some(vec2),
            vec![Operand::IdRef(uint), Operand::LiteralBit32(2)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(four),
            vec![Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(nil),
            vec![Operand::LiteralBit32(0)],
        ),
        composite(mixed, [four, nil]),
        composite(all_four, [four, four]),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![Instruction::new(
            Op::FunctionParameter,
            Some(uint),
            Some(numer),
            vec![],
        )],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                divide(20, four, uint),
                divide(21, all_four, vec2),
                divide(22, nil, uint),
                divide(23, mixed, vec2),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    guard_integer_division_by_zero(&mut ctx, 0);

    let body = &ctx.module.functions[0].blocks[0].instructions;
    let guards = body
        .iter()
        .filter(|inst| inst.class.opcode == Op::Select)
        .count();
    assert_eq!(
        guards, 2,
        "only the zero scalar and the mixed vector keep a guard"
    );

    let denom_of = |result: u32| {
        body.iter()
            .find(|inst| inst.result_id == Some(result))
            .and_then(|inst| match inst.operands.get(1) {
                Some(Operand::IdRef(denom)) => Some(*denom),
                _ => None,
            })
            .expect("the divide survives with an id denominator")
    };
    assert_eq!(
        denom_of(20),
        four,
        "a non-zero scalar constant divides directly"
    );
    assert_eq!(
        denom_of(21),
        all_four,
        "a composite with no zero lane divides directly"
    );
    assert_ne!(denom_of(22), nil, "a zero constant is still guarded");
    assert_ne!(
        denom_of(23),
        mixed,
        "a composite with one zero lane is still guarded"
    );
}

#[test]
fn integer_constant_is_never_zero_refuses_a_float_negative_zero() {
    use crate::passes::integer_constant_is_never_zero;

    let float = 1;
    let uint = 2;
    let neg_zero = 10;
    let four = 11;

    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(float),
            Some(neg_zero),
            vec![Operand::LiteralBit32(0x8000_0000)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(four),
            vec![Operand::LiteralBit32(4)],
        ),
    ];

    let ctx = crate::passes::Ctx::new(module);
    assert!(
        !integer_constant_is_never_zero(&ctx, neg_zero),
        "float -0.0 has non-zero bits but is zero"
    );
    assert!(
        integer_constant_is_never_zero(&ctx, four),
        "a non-zero integer constant is still proved"
    );
}

#[test]
fn narrow_access_chain_indices_narrows_constant_i64_index() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let ulong = 1;
    let uint = 2;
    let ptr = 3;
    let base = 10;
    let c64 = 11;
    let c32 = 12;
    let chain_a = 20;
    let chain_b = 21;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(ulong),
            vec![Operand::LiteralBit32(64), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr),
            Some(base),
            vec![Operand::StorageClass(StorageClass::StorageBuffer)],
        ),
        Instruction::new(
            Op::Constant,
            Some(ulong),
            Some(c64),
            vec![Operand::LiteralBit64(5)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(c32),
            vec![Operand::LiteralBit32(3)],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr),
                    Some(chain_a),
                    vec![Operand::IdRef(base), Operand::IdRef(c64)],
                ),
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr),
                    Some(chain_b),
                    vec![Operand::IdRef(base), Operand::IdRef(c32)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| narrow_access_chain_indices(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    let Some(Operand::IdRef(new_idx)) = body[0].operands.get(1) else {
        panic!("chain A lost its index operand");
    };
    assert_ne!(*new_idx, c64, "the 64-bit constant index must be replaced");
    let narrowed = ctx
        .new_globals
        .iter()
        .chain(ctx.module.types_global_values.iter())
        .find(|g| g.result_id == Some(*new_idx))
        .expect("the narrowed index constant must exist");
    assert_eq!(narrowed.class.opcode, Op::Constant);
    assert_eq!(
        narrowed.result_type,
        Some(uint),
        "the narrowed index is a 32-bit uint constant"
    );
    assert_eq!(narrowed.operands, vec![Operand::LiteralBit32(5)]);
    assert_eq!(
        body[1].operands.get(1),
        Some(&Operand::IdRef(c32)),
        "an already-32-bit index must be left alone"
    );
}

#[test]
fn drop_overindexed_zero_tail_truncates_trailing_zero_overindex() {
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));

    let uint = 1;
    let struct_s = 2;
    let ptr_struct = 3;
    let ptr_uint = 4;
    let base = 10;
    let c0 = 11;
    let chain = 20;
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(struct_s),
            vec![Operand::IdRef(uint)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_struct),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(struct_s),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_uint),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_struct),
            Some(base),
            vec![Operand::StorageClass(StorageClass::StorageBuffer)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(c0),
            vec![Operand::LiteralBit32(0)],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(50),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(51),
            ],
        )),
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::InBoundsAccessChain,
                Some(ptr_uint),
                Some(chain),
                vec![Operand::IdRef(base), Operand::IdRef(c0), Operand::IdRef(c0)],
            )],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| drop_overindexed_zero_tail(c, 0));

    let chain_inst = &ctx.module.functions[0].blocks[0].instructions[0];
    assert_eq!(
        chain_inst.operands,
        vec![Operand::IdRef(base), Operand::IdRef(c0)],
        "the trailing zero over-index is dropped, leaving the valid single-index chain"
    );
}

#[test]
fn private_low_byte_word_load_reads_only_declared_byte() {
    let uchar = 1;
    let uint = 2;
    let uchar4 = 3;
    let ptr_uchar = 4;
    let ptr_uint = 5;
    let zero = 6;
    let base = 7;
    let chain = 8;
    let wide = 9;
    let bytes = 10;
    let low = 11;
    let sum = 12;
    let one = 13;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uchar),
            vec![Operand::LiteralBit32(8), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeVector,
            None,
            Some(uchar4),
            vec![Operand::IdRef(uchar), Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_uchar),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(uchar),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_uint),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(zero),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uchar),
            Some(one),
            vec![Operand::LiteralBit32(1)],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_uchar),
            Some(base),
            vec![Operand::StorageClass(StorageClass::Private)],
        ),
    ];
    module.functions.push(Function {
        def: None,
        end: None,
        parameters: vec![],
        blocks: vec![Block {
            label: None,
            instructions: vec![
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_uint),
                    Some(chain),
                    vec![Operand::IdRef(base), Operand::IdRef(zero)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(uint),
                    Some(wide),
                    vec![Operand::IdRef(chain)],
                ),
                Instruction::new(
                    Op::Bitcast,
                    Some(uchar4),
                    Some(bytes),
                    vec![Operand::IdRef(wide)],
                ),
                Instruction::new(
                    Op::CompositeExtract,
                    Some(uchar),
                    Some(low),
                    vec![Operand::IdRef(bytes), Operand::LiteralBit32(0)],
                ),
                Instruction::new(
                    Op::IAdd,
                    Some(uchar),
                    Some(sum),
                    vec![Operand::IdRef(low), Operand::IdRef(one)],
                ),
            ],
        }],
    });

    let mut ctx = crate::passes::Ctx::new(module);
    lower_private_low_byte_word_load(&mut ctx, 0);

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert_eq!(body.len(), 2);
    assert_eq!(body[0].class.opcode, Op::Load);
    assert_eq!(body[0].result_type, Some(uchar));
    assert_eq!(body[0].operands, vec![Operand::IdRef(base)]);
    assert_eq!(body[1].class.opcode, Op::IAdd);
    assert_eq!(
        body[1].operands[0],
        Operand::IdRef(body[0].result_id.unwrap())
    );
}

#[test]
fn lower_scalar_i64_arithmetic_to_u32_halves_preserves_result_id() {
    let uint = 1;
    let ulong = 2;
    let lhs = 10;
    let rhs = 11;
    let product = 20;
    let difference = 21;
    let sum = 22;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(ulong),
            vec![Operand::LiteralBit32(64), Operand::LiteralBit32(0)],
        ),
    ];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(ulong),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![
            Instruction::new(Op::FunctionParameter, Some(ulong), Some(lhs), vec![]),
            Instruction::new(Op::FunctionParameter, Some(ulong), Some(rhs), vec![]),
        ],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::IMul,
                    Some(ulong),
                    Some(product),
                    vec![Operand::IdRef(lhs), Operand::IdRef(rhs)],
                ),
                Instruction::new(
                    Op::ISub,
                    Some(ulong),
                    Some(difference),
                    vec![Operand::IdRef(lhs), Operand::IdRef(rhs)],
                ),
                Instruction::new(
                    Op::IAdd,
                    Some(ulong),
                    Some(sum),
                    vec![Operand::IdRef(lhs), Operand::IdRef(rhs)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, lower_scalar_i64_arithmetic_to_u32_halves);

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert!(
        !body
            .iter()
            .any(|i| matches!(i.class.opcode, Op::IAdd | Op::ISub | Op::IMul)
                && i.result_type == Some(ulong)),
        "native 64-bit scalar arithmetic should be decomposed"
    );
    assert!(
        !body.iter().any(|i| i.class.opcode == Op::UMulExtended),
        "extended multiply is deliberately avoided"
    );
    assert!(
        body.iter()
            .filter(|i| i.class.opcode == Op::IMul && i.result_type == Some(uint))
            .count()
            >= 6,
        "16-bit pieces and 32-bit cross-products rebuild the low 64 bits"
    );
    for result in [product, difference, sum] {
        let final_inst = body
            .iter()
            .find(|i| i.result_id == Some(result))
            .expect("original result id is preserved");
        assert_eq!(final_inst.class.opcode, Op::BitwiseOr);
        assert_eq!(final_inst.result_type, Some(ulong));
    }
}

#[test]
fn lower_subword_scalar_store_splits_wide_store_into_per_element_little_endian_stores() {
    let uchar = 1;
    let uint = 2;
    let ptr_sb_uchar = 3;
    let idx0 = 4;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uchar),
            vec![Operand::LiteralBit32(8), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_uchar),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uchar),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx0),
            vec![Operand::LiteralBit32(0)],
        ),
    ];

    let base = 50;
    let obj = 51;
    let elem = 60;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_uchar),
                Some(base),
                vec![],
            ),
            Instruction::new(Op::FunctionParameter, Some(uint), Some(obj), vec![]),
        ],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::PtrAccessChain,
                    Some(ptr_sb_uchar),
                    Some(elem),
                    vec![Operand::IdRef(base), Operand::IdRef(idx0)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(elem), Operand::IdRef(obj)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| lower_subword_scalar_store(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    let stores: Vec<&Instruction> = body
        .iter()
        .filter(|i| i.class.opcode == Op::Store)
        .collect();
    assert_eq!(
        stores.len(),
        4,
        "one store per 8-bit slot of the 32-bit object"
    );
    assert!(
        !stores
            .iter()
            .any(|s| s.operands.get(1) == Some(&Operand::IdRef(obj))),
        "the wide object is never stored directly through the byte pointer"
    );
    let truncations = body
        .iter()
        .filter(|i| i.class.opcode == Op::UConvert && i.result_type == Some(uchar))
        .count();
    assert_eq!(
        truncations, 4,
        "each slot narrows to the uchar pointee width"
    );
    let shifts = body
        .iter()
        .filter(|i| i.class.opcode == Op::ShiftRightLogical)
        .count();
    assert_eq!(
        shifts, 3,
        "slots 1,2,3 shift the object right; slot 0 does not"
    );
    let chains = body
        .iter()
        .filter(|i| i.class.opcode == Op::PtrAccessChain)
        .count();
    assert_eq!(
        chains, 4,
        "three sibling element pointers plus the original base chain"
    );
}

#[test]
fn lower_subword_scalar_store_splits_wide_store_through_array_element_access_chain() {
    let uchar = 1;
    let uint = 2;
    let runtime_uchar = 3;
    let block_ty = 4;
    let ptr_sb_block = 5;
    let ptr_sb_uchar = 6;
    let member0 = 7;
    let dyn_idx = 8;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uchar),
            vec![Operand::LiteralBit32(8), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeRuntimeArray,
            None,
            Some(runtime_uchar),
            vec![Operand::IdRef(uchar)],
        ),
        Instruction::new(
            Op::TypeStruct,
            None,
            Some(block_ty),
            vec![Operand::IdRef(runtime_uchar)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_block),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(block_ty),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_uchar),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uchar),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(member0),
            vec![Operand::LiteralBit32(0)],
        ),
    ];

    let base = 50;
    let obj = 51;
    let elem = 60;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_block),
                Some(base),
                vec![],
            ),
            Instruction::new(Op::FunctionParameter, Some(uint), Some(obj), vec![]),
            Instruction::new(Op::FunctionParameter, Some(uint), Some(dyn_idx), vec![]),
        ],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::InBoundsAccessChain,
                    Some(ptr_sb_uchar),
                    Some(elem),
                    vec![
                        Operand::IdRef(base),
                        Operand::IdRef(member0),
                        Operand::IdRef(dyn_idx),
                    ],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(elem), Operand::IdRef(obj)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| lower_subword_scalar_store(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert_eq!(
        body.iter().filter(|i| i.class.opcode == Op::Store).count(),
        4,
        "the wide store is split even when the element pointer is a composed access-chain"
    );
    assert!(
        !body.iter().any(|i| i.class.opcode == Op::Store
            && i.operands.get(1) == Some(&Operand::IdRef(obj))),
        "the original wide object is not stored directly through the uchar pointer"
    );
    assert_eq!(
        body.iter()
            .filter(|i| i.class.opcode == Op::PtrAccessChain)
            .count(),
        3,
        "slots 1..3 use sibling element pointers from the access-chain element"
    );
}

#[test]
fn lower_subword_scalar_store_splits_vector_into_scalar_element_stores() {
    let ushort = 1;
    let ulong = 2;
    let v4ushort = 3;
    let ptr_sb_ushort = 4;
    let idx0 = 5;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(ushort),
            vec![Operand::LiteralBit32(16), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(ulong),
            vec![Operand::LiteralBit32(64), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeVector,
            None,
            Some(v4ushort),
            vec![Operand::IdRef(ushort), Operand::LiteralBit32(4)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_ushort),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(ushort),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(ulong),
            Some(idx0),
            vec![Operand::LiteralBit64(0)],
        ),
    ];

    let base = 50;
    let object = 51;
    let element = 60;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(ulong),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_ushort),
                Some(base),
                vec![],
            ),
            Instruction::new(Op::FunctionParameter, Some(v4ushort), Some(object), vec![]),
        ],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::PtrAccessChain,
                    Some(ptr_sb_ushort),
                    Some(element),
                    vec![Operand::IdRef(base), Operand::IdRef(idx0)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(element), Operand::IdRef(object)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| lower_subword_scalar_store(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert_eq!(
        body.iter()
            .filter(|inst| inst.class.opcode == Op::Store)
            .count(),
        4,
        "one scalar store per vector lane"
    );
    assert!(
        !body.iter().any(|inst| inst.class.opcode == Op::Store
            && inst.operands.get(1) == Some(&Operand::IdRef(object))),
        "the mismatched vector store is removed"
    );
    assert!(
        body.iter()
            .any(|inst| inst.class.opcode == Op::Bitcast && inst.result_type == Some(ulong)),
        "the vector payload is reinterpreted once as its 64-bit bit pattern"
    );
}

#[test]
fn lower_subword_scalar_store_leaves_matched_and_non_element_stores_alone() {
    let uchar = 1;
    let uint = 2;
    let ptr_sb_uchar = 3;
    let ptr_sb_uint = 4;
    let idx0 = 5;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uchar),
            vec![Operand::LiteralBit32(8), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_uchar),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uchar),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_uint),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(uint),
            ],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx0),
            vec![Operand::LiteralBit32(0)],
        ),
    ];

    let base = 50;
    let byte_obj = 51;
    let wide_obj = 52;
    let wide_param_ptr = 53;
    let elem = 60;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(uint),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_uchar),
                Some(base),
                vec![],
            ),
            Instruction::new(Op::FunctionParameter, Some(uchar), Some(byte_obj), vec![]),
            Instruction::new(Op::FunctionParameter, Some(uint), Some(wide_obj), vec![]),
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_uint),
                Some(wide_param_ptr),
                vec![],
            ),
        ],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::PtrAccessChain,
                    Some(ptr_sb_uchar),
                    Some(elem),
                    vec![Operand::IdRef(base), Operand::IdRef(idx0)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(elem), Operand::IdRef(byte_obj)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(wide_param_ptr), Operand::IdRef(wide_obj)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| lower_subword_scalar_store(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert_eq!(
        body.iter().filter(|i| i.class.opcode == Op::Store).count(),
        2,
        "neither store is split"
    );
    assert!(
        body.iter().any(|i| i.class.opcode == Op::Store
            && i.operands == vec![Operand::IdRef(elem), Operand::IdRef(byte_obj)]),
        "the matched byte store is untouched"
    );
    assert!(
        body.iter().any(|i| i.class.opcode == Op::Store
            && i.operands == vec![Operand::IdRef(wide_param_ptr), Operand::IdRef(wide_obj)]),
        "the wide store through a non-element pointer is untouched"
    );
    assert!(
        !body.iter().any(|i| i.class.opcode == Op::ShiftRightLogical),
        "no slotting arithmetic is emitted"
    );
}

#[test]
fn neutralize_null_access_chains_poisons_null_derived_chain_load_and_store() {
    let float = 1;
    let ptr_sb_float = 2;
    let uint = 3;
    let idx0 = 4;
    let null_base = 5;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_float),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::ConstantNull,
            Some(ptr_sb_float),
            Some(null_base),
            vec![],
        ),
    ];

    let chain = 60;
    let val = 61;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::AccessChain,
                    Some(ptr_sb_float),
                    Some(chain),
                    vec![Operand::IdRef(null_base), Operand::IdRef(idx0)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(val),
                    vec![Operand::IdRef(chain)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(null_base), Operand::IdRef(val)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| neutralize_null_access_chains(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert!(
        !body.iter().any(|i| matches!(
            i.class.opcode,
            Op::AccessChain | Op::InBoundsAccessChain | Op::PtrAccessChain
        )),
        "the null-rooted chain is removed"
    );
    assert!(
        !body.iter().any(|i| i.class.opcode == Op::Load),
        "the load through the null pointer is removed"
    );
    assert!(
        !body.iter().any(|i| i.class.opcode == Op::Store),
        "the store through the null pointer is dropped entirely"
    );
    let undef = body
        .iter()
        .find(|i| i.result_id == Some(chain))
        .expect("the chain result id survives as a poisoned value");
    assert_eq!(undef.class.opcode, Op::Undef);
    assert_eq!(undef.result_type, Some(ptr_sb_float));
    let copy = body
        .iter()
        .find(|i| i.result_id == Some(val))
        .expect("the load result id survives as a null copy");
    assert_eq!(copy.class.opcode, Op::CopyObject);
    assert_eq!(copy.result_type, Some(float));
    let Some(Operand::IdRef(zero)) = copy.operands.first() else {
        panic!("copy sources a value id");
    };
    let zero_def = ctx
        .new_globals
        .iter()
        .find(|i| i.result_id == Some(*zero))
        .expect("the null source is a synthesized global");
    assert_eq!(zero_def.class.opcode, Op::ConstantNull);
    assert_eq!(zero_def.result_type, Some(float));
}

#[test]
fn neutralize_null_access_chains_names_one_null_per_type() {
    let float = 1;
    let ptr_sb_float = 2;
    let null_base = 5;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_float),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::ConstantNull,
            Some(ptr_sb_float),
            Some(null_base),
            vec![],
        ),
    ];
    let first = 60;
    let second = 61;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(first),
                    vec![Operand::IdRef(null_base)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(second),
                    vec![Operand::IdRef(null_base)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| neutralize_null_access_chains(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    let source_of = |result| {
        let copy = body
            .iter()
            .find(|i| i.result_id == Some(result))
            .expect("the load result id survives as a null copy");
        assert_eq!(copy.class.opcode, Op::CopyObject);
        match copy.operands.first() {
            Some(Operand::IdRef(zero)) => *zero,
            _ => panic!("copy sources a value id"),
        }
    };
    assert_eq!(
        source_of(first),
        source_of(second),
        "both copies name the same null"
    );
    let nulls = ctx
        .new_globals
        .iter()
        .filter(|i| i.class.opcode == Op::ConstantNull && i.result_type == Some(float))
        .count();
    assert_eq!(nulls, 1, "one float null is synthesized, not two");
}

#[test]
fn neutralize_null_access_chains_leaves_live_pointer_access_alone() {
    let float = 1;
    let ptr_sb_float = 2;
    let uint = 3;
    let idx0 = 4;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_float),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx0),
            vec![Operand::LiteralBit32(0)],
        ),
    ];

    let base = 50;
    let chain = 60;
    let val = 61;
    let chain_ops = vec![Operand::IdRef(base), Operand::IdRef(idx0)];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![Instruction::new(
            Op::FunctionParameter,
            Some(ptr_sb_float),
            Some(base),
            vec![],
        )],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::AccessChain,
                    Some(ptr_sb_float),
                    Some(chain),
                    chain_ops.clone(),
                ),
                Instruction::new(
                    Op::Load,
                    Some(float),
                    Some(val),
                    vec![Operand::IdRef(chain)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(chain), Operand::IdRef(val)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| neutralize_null_access_chains(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert_eq!(body.len(), 3, "no instruction is added or removed");
    let chain_inst = body.iter().find(|i| i.result_id == Some(chain)).unwrap();
    assert_eq!(chain_inst.class.opcode, Op::AccessChain);
    assert_eq!(chain_inst.operands, chain_ops);
    assert!(
        body.iter()
            .any(|i| i.class.opcode == Op::Load && i.result_id == Some(val)),
        "the load over the live pointer survives"
    );
    assert!(
        body.iter().any(|i| i.class.opcode == Op::Store),
        "the store over the live pointer survives"
    );
}

#[test]
fn neutralize_private_placeholder_access_chains_replaces_unnamed_null_private_chain() {
    let float = 1;
    let uint = 2;
    let len2 = 3;
    let arr = 4;
    let ptr_priv_arr = 5;
    let ptr_priv_float = 6;
    let null_arr = 7;
    let idx0 = 8;
    let var = 9;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(len2),
            vec![Operand::LiteralBit32(2)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(arr),
            vec![Operand::IdRef(float), Operand::IdRef(len2)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_priv_arr),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(arr),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_priv_float),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(Op::ConstantNull, Some(arr), Some(null_arr), vec![]),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_priv_arr),
            Some(var),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(null_arr),
            ],
        ),
    ];

    let chain = 60;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::AccessChain,
                Some(ptr_priv_float),
                Some(chain),
                vec![Operand::IdRef(var), Operand::IdRef(idx0)],
            )],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| {
        neutralize_private_placeholder_access_chains(c, 0).unwrap()
    });

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert!(
        !body.iter().any(|i| i.class.opcode == Op::AccessChain),
        "the placeholder-rooted chain is removed"
    );
    let copy = body
        .iter()
        .find(|i| i.result_id == Some(chain))
        .expect("the chain result id survives as a copy");
    assert_eq!(copy.class.opcode, Op::CopyObject);
    assert_eq!(copy.result_type, Some(ptr_priv_float));
    let Some(Operand::IdRef(placeholder)) = copy.operands.first() else {
        panic!("copy sources a placeholder pointer id");
    };
    let ph_def = ctx
        .new_globals
        .iter()
        .find(|i| i.result_id == Some(*placeholder))
        .expect("placeholder pointer is a synthesized global");
    assert_eq!(ph_def.class.opcode, Op::Variable);
    assert_eq!(ph_def.result_type, Some(ptr_priv_float));
    assert_eq!(
        ph_def.operands.first(),
        Some(&Operand::StorageClass(StorageClass::Private))
    );
}

#[test]
fn neutralize_private_placeholder_component_replaces_loop_phi_and_backedge() {
    let float = 1;
    let uint = 2;
    let len2 = 3;
    let arr = 4;
    let ptr_priv_arr = 5;
    let ptr_priv_float = 6;
    let null_arr = 7;
    let idx0 = 8;
    let var = 9;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(len2),
            vec![Operand::LiteralBit32(2)],
        ),
        Instruction::new(
            Op::TypeArray,
            None,
            Some(arr),
            vec![Operand::IdRef(float), Operand::IdRef(len2)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_priv_arr),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(arr),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_priv_float),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(Op::ConstantNull, Some(arr), Some(null_arr), vec![]),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_priv_arr),
            Some(var),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(null_arr),
            ],
        ),
    ];

    let entry = 50;
    let header = 51;
    let latch = 52;
    let first = 60;
    let phi = 61;
    let backedge = 62;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![],
        blocks: vec![
            Block {
                label: Some(Instruction::new(Op::Label, None, Some(entry), vec![])),
                instructions: vec![
                    Instruction::new(
                        Op::AccessChain,
                        Some(ptr_priv_float),
                        Some(first),
                        vec![Operand::IdRef(var), Operand::IdRef(idx0)],
                    ),
                    Instruction::new(Op::Branch, None, None, vec![Operand::IdRef(header)]),
                ],
            },
            Block {
                label: Some(Instruction::new(Op::Label, None, Some(header), vec![])),
                instructions: vec![
                    Instruction::new(
                        Op::Phi,
                        Some(ptr_priv_float),
                        Some(phi),
                        vec![
                            Operand::IdRef(first),
                            Operand::IdRef(entry),
                            Operand::IdRef(backedge),
                            Operand::IdRef(latch),
                        ],
                    ),
                    Instruction::new(Op::Branch, None, None, vec![Operand::IdRef(latch)]),
                ],
            },
            Block {
                label: Some(Instruction::new(Op::Label, None, Some(latch), vec![])),
                instructions: vec![
                    Instruction::new(
                        Op::AccessChain,
                        Some(ptr_priv_float),
                        Some(backedge),
                        vec![Operand::IdRef(var), Operand::IdRef(idx0)],
                    ),
                    Instruction::new(Op::Branch, None, None, vec![Operand::IdRef(header)]),
                ],
            },
        ],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| {
        neutralize_private_placeholder_access_chains(c, 0).unwrap()
    });

    for result in [first, phi, backedge] {
        let inst = ctx.module.functions[0]
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .find(|inst| inst.result_id == Some(result))
            .expect("placeholder-derived pointer result survives");
        assert_eq!(inst.class.opcode, Op::CopyObject);
        assert_eq!(inst.result_type, Some(ptr_priv_float));
    }
    assert!(!ctx.module.functions[0]
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .any(|inst| matches!(inst.class.opcode, Op::Phi | Op::AccessChain)));
}

#[test]
fn neutralize_private_placeholder_access_chains_spares_named_private_variable() {
    let float = 1;
    let uint = 2;
    let ptr_priv_float = 3;
    let null_float = 4;
    let idx0 = 5;
    let var = 6;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_priv_float),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(Op::ConstantNull, Some(float), Some(null_float), vec![]),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(idx0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Variable,
            Some(ptr_priv_float),
            Some(var),
            vec![
                Operand::StorageClass(StorageClass::Private),
                Operand::IdRef(null_float),
            ],
        ),
    ];
    module.debug_names = vec![Instruction::new(
        Op::Name,
        None,
        None,
        vec![
            Operand::IdRef(var),
            Operand::LiteralString("g_state".into()),
        ],
    )];

    let chain = 60;
    let chain_ops = vec![Operand::IdRef(var), Operand::IdRef(idx0)];
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![Instruction::new(
                Op::AccessChain,
                Some(ptr_priv_float),
                Some(chain),
                chain_ops.clone(),
            )],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });

    let mut ctx = crate::passes::Ctx::new(module);
    run_idempotent(&mut ctx, |c| {
        neutralize_private_placeholder_access_chains(c, 0).unwrap()
    });

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert_eq!(body.len(), 1, "no instruction is added or removed");
    assert_eq!(body[0].class.opcode, Op::AccessChain);
    assert_eq!(body[0].operands, chain_ops, "the named chain is untouched");
}

#[test]
fn recover_inlined_local_pointer_fields_forwards_stored_source_to_matching_load() {
    let float = 1;
    let ptr_sb_float = 2;
    let uint = 3;
    let c1 = 4;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_float),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(c1),
            vec![Operand::LiteralBit32(1)],
        ),
    ];

    let root = 50;
    let source = 80;
    let stored_val = 70;
    let chain = 60;
    let load = 90;
    let dynamic_load = 95;
    let consumer = 100;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_float),
                Some(root),
                vec![],
            ),
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_float),
                Some(source),
                vec![],
            ),
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_float),
                Some(stored_val),
                vec![],
            ),
        ],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::AccessChain,
                    Some(ptr_sb_float),
                    Some(chain),
                    vec![Operand::IdRef(root), Operand::IdRef(c1)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(chain), Operand::IdRef(stored_val)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(ptr_sb_float),
                    Some(load),
                    vec![Operand::IdRef(chain)],
                ),
                Instruction::new(
                    Op::CopyObject,
                    Some(ptr_sb_float),
                    Some(consumer),
                    vec![Operand::IdRef(load)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });
    let mut ctx = crate::passes::Ctx::new(module);
    ctx.emit_sidecar
        .local_pointer_field_stores
        .push(crate::emit_sidecar::LocalPointerFieldStore {
            id: stored_val,
            source,
            root,
            indices: vec![1],
        });
    ctx.emit_sidecar
        .local_pointer_field_loads
        .push(crate::emit_sidecar::LocalPointerFieldLoad {
            id: load,
            root,
            indices: vec![1],
        });
    ctx.emit_sidecar.local_pointer_dynamic_field_loads.push(
        crate::emit_sidecar::LocalPointerDynamicFieldLoad {
            id: dynamic_load,
            root: load,
            prefix: vec![],
            index: c1,
            suffix: vec![0],
        },
    );
    run_idempotent(&mut ctx, |c| recover_inlined_local_pointer_fields(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    let consumer_inst = body.iter().find(|i| i.result_id == Some(consumer)).unwrap();
    assert_eq!(
        consumer_inst.operands,
        vec![Operand::IdRef(source)],
        "the load use is repointed at the stored source pointer"
    );
    assert!(
        body.iter().any(|i| i.result_id == Some(load)),
        "the load def id is preserved; only its uses forward"
    );
    assert_eq!(
        ctx.emit_sidecar.local_pointer_dynamic_field_loads[0].root, source,
        "typed consumers observe the same forwarding as function operands"
    );
}

#[test]
fn recover_inlined_local_pointer_fields_leaves_key_mismatched_load_alone() {
    let float = 1;
    let ptr_sb_float = 2;
    let uint = 3;
    let c1 = 4;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_float),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(c1),
            vec![Operand::LiteralBit32(1)],
        ),
    ];

    let root = 50;
    let source = 80;
    let stored_val = 70;
    let chain = 60;
    let load = 90;
    let consumer = 100;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_float),
                Some(root),
                vec![],
            ),
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_float),
                Some(source),
                vec![],
            ),
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_float),
                Some(stored_val),
                vec![],
            ),
        ],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::AccessChain,
                    Some(ptr_sb_float),
                    Some(chain),
                    vec![Operand::IdRef(root), Operand::IdRef(c1)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(chain), Operand::IdRef(stored_val)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(ptr_sb_float),
                    Some(load),
                    vec![Operand::IdRef(chain)],
                ),
                Instruction::new(
                    Op::CopyObject,
                    Some(ptr_sb_float),
                    Some(consumer),
                    vec![Operand::IdRef(load)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });
    let mut ctx = crate::passes::Ctx::new(module);
    ctx.emit_sidecar
        .local_pointer_field_stores
        .push(crate::emit_sidecar::LocalPointerFieldStore {
            id: stored_val,
            source,
            root,
            indices: vec![2],
        });
    ctx.emit_sidecar
        .local_pointer_field_loads
        .push(crate::emit_sidecar::LocalPointerFieldLoad {
            id: load,
            root,
            indices: vec![2],
        });
    run_idempotent(&mut ctx, |c| recover_inlined_local_pointer_fields(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    let consumer_inst = body.iter().find(|i| i.result_id == Some(consumer)).unwrap();
    assert_eq!(
        consumer_inst.operands,
        vec![Operand::IdRef(load)],
        "the key-mismatched load is not forwarded"
    );
}

#[test]
fn recover_inlined_local_pointer_fields_leaves_non_pointer_source_alone() {
    let float = 1;
    let ptr_sb_float = 2;
    let uint = 3;
    let c1 = 4;
    let ulong = 5;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_float),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(c1),
            vec![Operand::LiteralBit32(1)],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(ulong),
            vec![Operand::LiteralBit32(64), Operand::LiteralBit32(0)],
        ),
    ];

    let root = 50;
    let source = 80;
    let stored_val = 70;
    let chain = 60;
    let load = 90;
    let consumer = 100;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_float),
                Some(root),
                vec![],
            ),
            Instruction::new(Op::FunctionParameter, Some(ulong), Some(source), vec![]),
            Instruction::new(Op::FunctionParameter, Some(ulong), Some(stored_val), vec![]),
        ],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::AccessChain,
                    Some(ptr_sb_float),
                    Some(chain),
                    vec![Operand::IdRef(root), Operand::IdRef(c1)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(chain), Operand::IdRef(stored_val)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(ptr_sb_float),
                    Some(load),
                    vec![Operand::IdRef(chain)],
                ),
                Instruction::new(
                    Op::AccessChain,
                    Some(ptr_sb_float),
                    Some(consumer),
                    vec![Operand::IdRef(load), Operand::IdRef(c1)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });
    let mut ctx = crate::passes::Ctx::new(module);
    ctx.emit_sidecar
        .local_pointer_field_stores
        .push(crate::emit_sidecar::LocalPointerFieldStore {
            id: stored_val,
            source,
            root,
            indices: vec![1],
        });
    ctx.emit_sidecar
        .local_pointer_field_loads
        .push(crate::emit_sidecar::LocalPointerFieldLoad {
            id: load,
            root,
            indices: vec![1],
        });
    run_idempotent(&mut ctx, |c| recover_inlined_local_pointer_fields(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    let consumer_inst = body.iter().find(|i| i.result_id == Some(consumer)).unwrap();
    assert_eq!(
        consumer_inst.operands,
        vec![Operand::IdRef(load), Operand::IdRef(c1)],
        "the access chain still descends the pointer load, not the ulong source"
    );
}

#[test]
fn recover_inlined_local_pointer_fields_forwards_a_loaded_image_into_a_handle_load() {
    let float = 1;
    let image = 2;
    let ptr_uc_image = 3;
    let uint = 4;
    let c1 = 5;
    let ptr_sb_float = 6;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(100));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypeImage,
            None,
            Some(image),
            vec![
                Operand::IdRef(float),
                Operand::Dim(spirv::Dim::Dim2D),
                Operand::LiteralBit32(0),
                Operand::LiteralBit32(0),
                Operand::LiteralBit32(0),
                Operand::LiteralBit32(1),
                Operand::ImageFormat(spirv::ImageFormat::Unknown),
            ],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_uc_image),
            vec![
                Operand::StorageClass(StorageClass::UniformConstant),
                Operand::IdRef(image),
            ],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(c1),
            vec![Operand::LiteralBit32(1)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr_sb_float),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
    ];

    let root = 50;
    let source = 80;
    let stored_val = 70;
    let chain = 60;
    let load = 90;
    let consumer = 100;
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(40),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(41),
            ],
        )),
        parameters: vec![
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_sb_float),
                Some(root),
                vec![],
            ),
            Instruction::new(Op::FunctionParameter, Some(image), Some(source), vec![]),
            Instruction::new(
                Op::FunctionParameter,
                Some(ptr_uc_image),
                Some(stored_val),
                vec![],
            ),
        ],
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(52), vec![])),
            instructions: vec![
                Instruction::new(
                    Op::AccessChain,
                    Some(ptr_sb_float),
                    Some(chain),
                    vec![Operand::IdRef(root), Operand::IdRef(c1)],
                ),
                Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(chain), Operand::IdRef(stored_val)],
                ),
                Instruction::new(
                    Op::Load,
                    Some(ptr_uc_image),
                    Some(load),
                    vec![Operand::IdRef(chain)],
                ),
                Instruction::new(
                    Op::CopyObject,
                    Some(ptr_uc_image),
                    Some(consumer),
                    vec![Operand::IdRef(load)],
                ),
            ],
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });
    let mut ctx = crate::passes::Ctx::new(module);
    ctx.emit_sidecar
        .local_pointer_field_stores
        .push(crate::emit_sidecar::LocalPointerFieldStore {
            id: stored_val,
            source,
            root,
            indices: vec![1],
        });
    ctx.emit_sidecar
        .local_pointer_field_loads
        .push(crate::emit_sidecar::LocalPointerFieldLoad {
            id: load,
            root,
            indices: vec![1],
        });
    run_idempotent(&mut ctx, |c| recover_inlined_local_pointer_fields(c, 0));

    let body = &ctx.module.functions[0].blocks[0].instructions;
    let consumer_inst = body.iter().find(|i| i.result_id == Some(consumer)).unwrap();
    assert_eq!(
        consumer_inst.operands,
        vec![Operand::IdRef(source)],
        "the loaded image reaches the consumer even though it is no longer pointer typed"
    );
}

#[test]
fn recover_inlined_dynamic_pointer_table_selects_exact_stored_sources() {
    let float = 1;
    let ptr = 2;
    let uint = 3;
    let c0 = 4;
    let c1 = 5;
    let c2 = 6;
    let mut module = Module::new();
    module.header = Some(ModuleHeader::new(200));
    module.types_global_values = vec![
        Instruction::new(
            Op::TypeFloat,
            None,
            Some(float),
            vec![Operand::LiteralBit32(32)],
        ),
        Instruction::new(
            Op::TypePointer,
            None,
            Some(ptr),
            vec![
                Operand::StorageClass(StorageClass::StorageBuffer),
                Operand::IdRef(float),
            ],
        ),
        Instruction::new(
            Op::TypeInt,
            None,
            Some(uint),
            vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(c0),
            vec![Operand::LiteralBit32(0)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(c1),
            vec![Operand::LiteralBit32(1)],
        ),
        Instruction::new(
            Op::Constant,
            Some(uint),
            Some(c2),
            vec![Operand::LiteralBit32(2)],
        ),
    ];

    let root = 20;
    let index = 21;
    let sources = [30, 31, 32];
    let markers = [40, 41, 42];
    let slots = [50, 51, 52];
    let base = 53;
    let dynamic_slot = 54;
    let load = 60;
    let consumer = 61;
    let mut instructions = Vec::new();
    for ((slot, marker), constant) in slots.into_iter().zip(markers).zip([c0, c1, c2]) {
        instructions.push(Instruction::new(
            Op::AccessChain,
            Some(ptr),
            Some(slot),
            vec![Operand::IdRef(root), Operand::IdRef(constant)],
        ));
        instructions.push(Instruction::new(
            Op::Store,
            None,
            None,
            vec![Operand::IdRef(slot), Operand::IdRef(marker)],
        ));
    }
    instructions.extend([
        Instruction::new(
            Op::AccessChain,
            Some(ptr),
            Some(base),
            vec![Operand::IdRef(root), Operand::IdRef(c0)],
        ),
        Instruction::new(
            Op::PtrAccessChain,
            Some(ptr),
            Some(dynamic_slot),
            vec![Operand::IdRef(base), Operand::IdRef(index)],
        ),
        Instruction::new(
            Op::Load,
            Some(ptr),
            Some(load),
            vec![Operand::IdRef(dynamic_slot)],
        ),
        Instruction::new(
            Op::CopyObject,
            Some(ptr),
            Some(consumer),
            vec![Operand::IdRef(load)],
        ),
    ]);
    module.functions.push(Function {
        def: Some(Instruction::new(
            Op::Function,
            Some(float),
            Some(10),
            vec![
                Operand::FunctionControl(FunctionControl::NONE),
                Operand::IdRef(11),
            ],
        )),
        parameters: std::iter::once((root, ptr))
            .chain(std::iter::once((index, uint)))
            .chain(sources.into_iter().map(|source| (source, ptr)))
            .chain(markers.into_iter().map(|marker| (marker, ptr)))
            .map(|(id, ty)| Instruction::new(Op::FunctionParameter, Some(ty), Some(id), vec![]))
            .collect(),
        blocks: vec![Block {
            label: Some(Instruction::new(Op::Label, None, Some(12), vec![])),
            instructions,
        }],
        end: Some(Instruction::new(Op::FunctionEnd, None, None, vec![])),
    });
    let mut ctx = crate::passes::Ctx::new(module);
    for (index, (id, source)) in markers.into_iter().zip(sources).enumerate() {
        ctx.emit_sidecar.local_pointer_field_stores.push(
            crate::emit_sidecar::LocalPointerFieldStore {
                id,
                source,
                root,
                indices: vec![index as u32],
            },
        );
    }
    recover_inlined_local_dynamic_pointer_fields(&mut ctx, 0).unwrap();

    let body = &ctx.module.functions[0].blocks[0].instructions;
    assert!(!body
        .iter()
        .any(|inst| inst.class.opcode == Op::Load && inst.result_id == Some(load)));
    let selected = body
        .iter()
        .find(|inst| inst.result_id == Some(load))
        .expect("final dynamic table selection");
    assert_eq!(selected.class.opcode, Op::Select);
    assert_eq!(selected.result_type, Some(ptr));
    assert_eq!(
        body.iter()
            .filter(|inst| inst.class.opcode == Op::Select)
            .count(),
        2
    );
}
