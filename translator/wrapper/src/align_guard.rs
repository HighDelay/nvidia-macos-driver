use crate::buffer_addresses::TABLE_BINDING;
use rspirv::{binary::Assemble, dr::{Instruction, Operand}, spirv::*};
use std::collections::HashMap;

fn id(o: &Operand) -> Option<u32> { if let Operand::IdRef(v) = o { Some(*v) } else { None } }
fn lit(o: &Operand) -> Option<u32> { if let Operand::LiteralBit32(v) = o { Some(*v) } else { None } }

fn scalar_align(ty: u32, defs: &HashMap<u32, Instruction>, depth: usize) -> Option<u32> {
    if depth > 32 { return None; }
    let t = defs.get(&ty)?;
    match t.class.opcode {
        Op::TypeInt | Op::TypeFloat => Some(lit(t.operands.first()?)? / 8),
        Op::TypePointer => Some(8),
        Op::TypeVector | Op::TypeMatrix | Op::TypeArray | Op::TypeRuntimeArray => scalar_align(id(t.operands.first()?)?, defs, depth + 1),
        Op::TypeStruct => t.operands.iter().try_fold(1, |a, o| Some(a.max(scalar_align(id(o)?, defs, depth + 1)?))),
        _ => None,
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report { pub mask: u32, pub req: u32, pub untraceable: bool, pub accesses: u32, pub clamped: u32 }

fn root_binding(mut p: u32, local: &HashMap<u32, Instruction>, table: u32, consts: &HashMap<u32, u32>) -> Option<u32> {
    for _ in 0..64 {
        let d = local.get(&p)?;
        let ops: Vec<u32> = d.operands.iter().filter_map(id).collect();
        match d.class.opcode {
            Op::AccessChain | Op::InBoundsAccessChain | Op::PtrAccessChain | Op::CopyObject | Op::Bitcast => p = *ops.first()?,
            Op::ConvertUToPtr => {
                let x = local.get(ops.first()?)?;
                let xo: Vec<u32> = x.operands.iter().filter_map(id).collect();
                match x.class.opcode {
                    Op::Load => {
                        let ac = local.get(xo.first()?)?;
                        let ao: Vec<u32> = ac.operands.iter().filter_map(id).collect();
                        return match (ac.class.opcode, ao.as_slice()) {
                            (Op::AccessChain | Op::InBoundsAccessChain, [t, z, b]) if *t == table && consts.get(z) == Some(&0) =>
                                consts.get(b).copied().filter(|b| *b < 32),
                            _ => None,
                        };
                    }
                    Op::ConvertPtrToU => p = *xo.first()?,
                    Op::IAdd => {
                        let base = xo.iter().find(|v| local.get(v).is_some_and(|d| d.class.opcode == Op::ConvertPtrToU))?;
                        p = local.get(base)?.operands.iter().find_map(id)?;
                    }
                    _ => return None,
                }
            }
            _ => return None,
        }
    }
    None
}

pub fn guard(bytes: &[u8], safe: bool) -> Result<(Vec<u8>, Report), String> {
    let mut m = rspirv::dr::load_bytes(bytes).map_err(|e| e.to_string())?;
    let defs: HashMap<u32, Instruction> = m.types_global_values.iter().filter_map(|i| Some((i.result_id?, i.clone()))).collect();
    let consts: HashMap<u32, u32> = m.types_global_values.iter().filter(|i| i.class.opcode == Op::Constant)
        .filter_map(|i| Some((i.result_id?, lit(i.operands.first()?)?))).collect();
    let table = m.annotations.iter().find_map(|a| match a.operands.as_slice() {
        [Operand::IdRef(n), Operand::Decoration(Decoration::Binding), Operand::LiteralBit32(b)] if *b == TABLE_BINDING => Some(*n),
        _ => None,
    }).unwrap_or(u32::MAX);
    let mut value_types = HashMap::new();
    for i in m.all_inst_iter() { if let (Some(n), Some(t)) = (i.result_id, i.result_type) { value_types.insert(n, t); } }
    let psb_pointee = |ptr: u32| -> Option<u32> {
        let t = defs.get(value_types.get(&ptr)?)?;
        (t.class.opcode == Op::TypePointer && t.operands.first() == Some(&Operand::StorageClass(StorageClass::PhysicalStorageBuffer)))
            .then(|| id(t.operands.get(1)?)).flatten()
    };
    let mut r = Report::default();
    for f in &mut m.functions {
        let local: HashMap<u32, Instruction> = f.blocks.iter().flat_map(|b| b.instructions.iter()).filter_map(|i| Some((i.result_id?, i.clone()))).collect();
        for b in &mut f.blocks {
            for i in &mut b.instructions {
                let at = match i.class.opcode { Op::Load => 1, Op::Store => 2, _ => continue };
                let Some(ptr) = i.operands.first().and_then(id) else { continue };
                let Some(pointee) = psb_pointee(ptr) else { continue };
                let (Some(Operand::MemoryAccess(flags)), Some(a)) = (i.operands.get(at), i.operands.get(at + 1).and_then(lit)) else { continue };
                if !flags.contains(MemoryAccess::ALIGNED) { continue; }
                let scalar = scalar_align(pointee, &defs, 0);
                if scalar.is_some_and(|s| a <= s) { continue; }
                r.accesses += 1;
                r.req = r.req.max(a);
                match root_binding(ptr, &local, table, &consts) {
                    Some(bnd) => r.mask |= 1 << bnd,
                    None => r.untraceable = true,
                }
                if safe {
                    if let Some(s) = scalar { i.operands[at + 1] = Operand::LiteralBit32(s); r.clamped += 1; }
                }
            }
        }
    }
    if safe && r.clamped > 0 {
        return Ok((m.assemble().iter().flat_map(|w| w.to_le_bytes()).collect(), r));
    }
    Ok((bytes.to_vec(), r))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer_addresses::lower;
    use spirv_tools::{assembler::Assembler, val::Validator};
    const PAIR: &str = r#"
OpCapability Shader
OpCapability Int64
OpMemoryModel Logical GLSL450
OpEntryPoint GLCompute %main "main" %buf %out %gid
OpExecutionMode %main LocalSize 1 1 1
OpDecorate %gid BuiltIn GlobalInvocationId
OpDecorate %arr ArrayStride 4
OpDecorate %block Block
OpMemberDecorate %block 0 Offset 0
OpDecorate %buf DescriptorSet 0
OpDecorate %buf Binding 0
OpDecorate %out DescriptorSet 0
OpDecorate %out Binding 1
%void = OpTypeVoid
%uint = OpTypeInt 32 0
%ulong = OpTypeInt 64 0
%v3 = OpTypeVector %uint 3
%pin = OpTypePointer Input %v3
%gid = OpVariable %pin Input
%zero = OpConstant %uint 0
%one = OpConstant %uint 1
%two = OpConstant %uint 2
%arr = OpTypeRuntimeArray %uint
%block = OpTypeStruct %arr
%pb = OpTypePointer StorageBuffer %block
%pu = OpTypePointer StorageBuffer %uint
%buf = OpVariable %pb StorageBuffer
%out = OpVariable %pb StorageBuffer
%fn = OpTypeFunction %void
%main = OpFunction %void None %fn
%label = OpLabel
%g3 = OpLoad %v3 %gid
%g = OpCompositeExtract %uint %g3 0
%gl = OpUConvert %ulong %g
%c0 = OpUConvert %uint %gl
%m0 = OpIMul %uint %c0 %two
%p0 = OpInBoundsAccessChain %pu %buf %zero %m0
%x0 = OpLoad %uint %p0 Aligned 8
%c1 = OpUConvert %uint %gl
%m1 = OpIMul %uint %c1 %two
%w1 = OpIAdd %uint %one %m1
%p1 = OpInBoundsAccessChain %pu %buf %zero %w1
%x1 = OpLoad %uint %p1
%s = OpIAdd %uint %x0 %x1
%po = OpAccessChain %pu %out %zero %g
OpStore %po %s
OpReturn
OpFunctionEnd
"#;
    fn lowered(text: &str) -> Vec<u8> {
        let b = spirv_tools::assembler::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).assemble(text, Default::default()).unwrap().as_bytes().to_vec();
        lower(&b).unwrap()
    }
    fn validate(b: &[u8]) {
        let w: Vec<u32> = b.chunks_exact(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect();
        spirv_tools::val::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).validate(&w, None).unwrap();
    }
    #[test] fn a_fused_pair_reports_its_binding_and_8_and_nothing_else() {
        let (_, r) = guard(&lowered(PAIR), false).unwrap();
        assert_eq!((r.mask, r.req, r.untraceable), (1, 8, false), "binding 0 needs 8; binding 1's Aligned-4 word is no promise: {r:?}");
    }
    #[test] fn the_safe_module_keeps_no_promise_and_validates() {
        let (safe, r) = guard(&lowered(PAIR), true).unwrap();
        validate(&safe);
        assert!(r.clamped >= 1);
        let (_, again) = guard(&safe, false).unwrap();
        assert_eq!((again.mask, again.accesses), (0, 0), "a safe module must report nothing: {again:?}");
    }
    #[test] fn an_unhinted_module_reports_no_promise() {
        let (bytes, r) = guard(&lowered(&PAIR.replace("%p0 Aligned 8", "%p0")), true).unwrap();
        assert_eq!((r.mask, r.accesses, r.clamped), (0, 0, 0));
        assert_eq!(bytes, lowered(&PAIR.replace("%p0 Aligned 8", "%p0")), "nothing to withdraw = the same bytes");
    }
}
