use crate::buffer_addresses::TABLE_BINDING;
use rspirv::{binary::Assemble, dr::{Instruction, Operand}, spirv::*};
use std::collections::{HashMap, HashSet};

pub const TABLE_ENTRIES: u32 = 64;

fn id(o: &Operand) -> Option<u32> { if let Operand::IdRef(v) = o { Some(*v) } else { None } }

pub fn convert(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut m = rspirv::dr::load_bytes(bytes).map_err(|e| e.to_string())?;
    let Some(table) = m.annotations.iter().find_map(|a| match a.operands.as_slice() {
        [Operand::IdRef(n), Operand::Decoration(Decoration::Binding), Operand::LiteralBit32(b)] if *b == TABLE_BINDING => Some(*n),
        _ => None,
    }) else { return Ok(bytes.to_vec()) };
    let defs: HashMap<u32, Instruction> = m.types_global_values.iter().filter_map(|i| Some((i.result_id?, i.clone()))).collect();
    let tv = defs.get(&table).filter(|i| i.class.opcode == Op::Variable).ok_or("binding 640 is not a global variable")?;
    match tv.operands.first() {
        Some(Operand::StorageClass(StorageClass::Uniform)) => return Ok(bytes.to_vec()),
        Some(Operand::StorageClass(StorageClass::StorageBuffer)) => {}
        other => return Err(format!("binding 640 has storage class {other:?}")),
    }
    let ptr_ty = tv.result_type.ok_or("table without a type")?;
    let block = defs.get(&ptr_ty).and_then(|p| p.operands.get(1)).and_then(id).ok_or("table pointer without a pointee")?;
    let arr = defs.get(&block).filter(|s| s.class.opcode == Op::TypeStruct && s.operands.len() == 1)
        .and_then(|s| id(&s.operands[0])).ok_or("table block is not a one-member struct")?;
    let arr_def = defs.get(&arr).ok_or("table array missing")?;
    if !matches!(arr_def.class.opcode, Op::TypeRuntimeArray | Op::TypeArray) { return Err("table member is not an array".into()); }
    let elem = arr_def.operands.first().and_then(id).ok_or("table array without an element type")?;
    let stride = m.annotations.iter().find_map(|a| match a.operands.as_slice() {
        [Operand::IdRef(n), Operand::Decoration(Decoration::ArrayStride), Operand::LiteralBit32(s)] if *n == arr => Some(*s),
        _ => None,
    }).ok_or("table array has no ArrayStride")?;
    let mut next = m.header.as_ref().ok_or("no SPIR-V header")?.bound;
    let mut fresh = || { let n = next; next += 1; n };
    let uint = defs.iter().find_map(|(&n, i)| (i.class.opcode == Op::TypeInt && i.operands == vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)]).then_some(n));
    let uint = match uint { Some(u) => u, None => { let n = fresh(); m.types_global_values.push(Instruction::new(Op::TypeInt, None, Some(n), vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)])); n } };
    let (count, arr2, block2, ptr2) = (fresh(), fresh(), fresh(), fresh());
    let tpos = m.types_global_values.iter().position(|i| i.result_id == Some(table)).unwrap();
    let mut tvar = m.types_global_values.remove(tpos);
    m.types_global_values.extend([
        Instruction::new(Op::Constant, Some(uint), Some(count), vec![Operand::LiteralBit32(TABLE_ENTRIES)]),
        Instruction::new(Op::TypeArray, None, Some(arr2), vec![Operand::IdRef(elem), Operand::IdRef(count)]),
        Instruction::new(Op::TypeStruct, None, Some(block2), vec![Operand::IdRef(arr2)]),
        Instruction::new(Op::TypePointer, None, Some(ptr2), vec![Operand::StorageClass(StorageClass::Uniform), Operand::IdRef(block2)]),
    ]);
    tvar.result_type = Some(ptr2);
    tvar.operands[0] = Operand::StorageClass(StorageClass::Uniform);
    m.types_global_values.push(tvar);
    m.annotations.retain(|a| !(a.class.opcode == Op::Decorate && a.operands.first() == Some(&Operand::IdRef(table))
        && matches!(a.operands.get(1), Some(Operand::Decoration(Decoration::NonWritable | Decoration::Restrict | Decoration::Aliased)))));
    m.annotations.extend([
        Instruction::new(Op::Decorate, None, None, vec![Operand::IdRef(arr2), Operand::Decoration(Decoration::ArrayStride), Operand::LiteralBit32(stride)]),
        Instruction::new(Op::Decorate, None, None, vec![Operand::IdRef(block2), Operand::Decoration(Decoration::Block)]),
        Instruction::new(Op::MemberDecorate, None, None, vec![Operand::IdRef(block2), Operand::LiteralBit32(0), Operand::Decoration(Decoration::Offset), Operand::LiteralBit32(0)]),
    ]);
    let mut uptr: HashMap<u32, u32> = HashMap::new();
    let mut derived: HashSet<u32> = HashSet::from([table]);
    let mut new_types = Vec::new();
    for f in &mut m.functions {
        for b in &mut f.blocks {
            for i in &mut b.instructions {
                let uses: Vec<u32> = i.operands.iter().filter_map(id).filter(|v| derived.contains(v)).collect();
                if uses.is_empty() { continue; }
                match i.class.opcode {
                    Op::AccessChain | Op::InBoundsAccessChain if i.operands.first().and_then(id).is_some_and(|v| derived.contains(&v))
                        && uses.len() == 1 => {
                        let old = i.result_type.ok_or("access chain without a type")?;
                        let pointee = defs.get(&old).filter(|p| p.class.opcode == Op::TypePointer).and_then(|p| p.operands.get(1)).and_then(id)
                            .ok_or("table access chain type is not a pointer")?;
                        let t = *uptr.entry(old).or_insert_with(|| {
                            let n = next; next += 1;
                            new_types.push(Instruction::new(Op::TypePointer, None, Some(n), vec![Operand::StorageClass(StorageClass::Uniform), Operand::IdRef(pointee)]));
                            n
                        });
                        i.result_type = Some(t);
                        derived.insert(i.result_id.ok_or("access chain without a result")?);
                    }
                    Op::Load if uses.len() == 1 && i.operands.first().and_then(id).is_some_and(|v| derived.contains(&v)) => {}
                    op => return Err(format!("the table is used by {op:?} - only access chains and loads can move to a Uniform block")),
                }
            }
        }
    }
    let vpos = m.types_global_values.iter().position(|i| i.result_id == Some(table)).unwrap();
    for (k, t) in new_types.into_iter().enumerate() { m.types_global_values.insert(vpos + k, t); }
    m.header.as_mut().unwrap().bound = next;
    Ok(m.assemble().iter().flat_map(|w| w.to_le_bytes()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer_addresses::lower;
    use spirv_tools::{assembler::Assembler, val::{Validator, ValidatorOptions}};
    const K: &str = r#"
OpCapability Shader
OpMemoryModel Logical GLSL450
OpEntryPoint GLCompute %main "main" %buf %gid
OpExecutionMode %main LocalSize 1 1 1
OpDecorate %gid BuiltIn GlobalInvocationId
OpDecorate %arr ArrayStride 4
OpDecorate %block Block
OpMemberDecorate %block 0 Offset 0
OpDecorate %buf DescriptorSet 0
OpDecorate %buf Binding 3
%void = OpTypeVoid
%uint = OpTypeInt 32 0
%v3 = OpTypeVector %uint 3
%pin = OpTypePointer Input %v3
%gid = OpVariable %pin Input
%zero = OpConstant %uint 0
%arr = OpTypeRuntimeArray %uint
%block = OpTypeStruct %arr
%pb = OpTypePointer StorageBuffer %block
%pu = OpTypePointer StorageBuffer %uint
%buf = OpVariable %pb StorageBuffer
%fn = OpTypeFunction %void
%main = OpFunction %void None %fn
%label = OpLabel
%g3 = OpLoad %v3 %gid
%g = OpCompositeExtract %uint %g3 0
%p = OpAccessChain %pu %buf %zero %g
%x = OpLoad %uint %p
OpStore %p %x
OpReturn
OpFunctionEnd
"#;
    fn validate_scalar(b: &[u8]) {
        let w: Vec<u32> = b.chunks_exact(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect();
        let mut o = ValidatorOptions::default(); o.scalar_block_layout = true;
        spirv_tools::val::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).validate(&w, Some(o)).unwrap();
    }
    fn lowered() -> Vec<u8> {
        let b = spirv_tools::assembler::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).assemble(K, Default::default()).unwrap().as_bytes().to_vec();
        lower(&b).unwrap()
    }
    #[test] fn a_lowered_kernels_table_becomes_a_valid_uniform_block_at_640() {
        let out = convert(&lowered()).unwrap();
        validate_scalar(&out);
        let m = rspirv::dr::load_bytes(&out).unwrap();
        let sb_vars = m.types_global_values.iter().filter(|i| i.class.opcode == Op::Variable
            && i.operands.first() == Some(&Operand::StorageClass(StorageClass::StorageBuffer))).count();
        let u_vars = m.types_global_values.iter().filter(|i| i.class.opcode == Op::Variable
            && i.operands.first() == Some(&Operand::StorageClass(StorageClass::Uniform))).count();
        assert_eq!((sb_vars, u_vars), (0, 1), "the table is the one Uniform variable, no storage buffer is left");
    }
    #[test] fn converting_twice_changes_nothing_and_a_tableless_module_is_untouched() {
        let once = convert(&lowered()).unwrap();
        assert_eq!(convert(&once).unwrap(), once);
        let plain = spirv_tools::assembler::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).assemble(K, Default::default()).unwrap().as_bytes().to_vec();
        assert_eq!(convert(&plain).unwrap(), plain, "binding 3 is not the table");
    }
}
