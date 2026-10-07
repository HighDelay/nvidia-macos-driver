use crate::buffer_addresses::TABLE_BINDING;
use rspirv::{binary::Assemble, dr::{Instruction, Operand}, spirv::*};
use std::collections::HashMap;

pub const SLOTS: u32 = 32;
pub const SLOT_BYTES: u32 = 16;
pub const INLINE_OFFSET: u32 = crate::table_ubo::TABLE_ENTRIES * 8;

fn id(o: &Operand) -> Option<u32> { if let Operand::IdRef(v) = o { Some(*v) } else { None } }
fn lit(o: &Operand) -> Option<u32> { if let Operand::LiteralBit32(v) = o { Some(*v) } else { None } }

#[derive(Clone, Debug)]
struct Site { f: usize, b: usize, i: usize, binding: u32, offset: u32, lanes: u32, lane_ty: u32, result_ty: u32 }

pub fn apply(bytes: &[u8], mask: u32) -> Result<(Vec<u8>, u32), String> {
    let mut m = rspirv::dr::load_bytes(bytes).map_err(|e| e.to_string())?;
    let defs: HashMap<u32, Instruction> = m.types_global_values.iter().filter_map(|i| Some((i.result_id?, i.clone()))).collect();
    let Some(table) = m.annotations.iter().find_map(|a| match a.operands.as_slice() {
        [Operand::IdRef(n), Operand::Decoration(Decoration::Binding), Operand::LiteralBit32(b)] if *b == TABLE_BINDING => Some(*n),
        _ => None,
    }) else { return Ok((bytes.to_vec(), 0)) };
    let Some(tv) = defs.get(&table).filter(|i| i.class.opcode == Op::Variable) else { return Ok((bytes.to_vec(), 0)) };
    if tv.operands.first() != Some(&Operand::StorageClass(StorageClass::Uniform)) { return Ok((bytes.to_vec(), 0)); }
    let consts: HashMap<u32, u32> = defs.iter().filter(|(_, i)| i.class.opcode == Op::Constant && i.operands.len() == 1)
        .filter_map(|(&n, i)| Some((n, lit(&i.operands[0])?))).collect();
    let deco = |target: u32, d: Decoration| -> Option<u32> {
        m.annotations.iter().find_map(|a| match a.operands.as_slice() {
            [Operand::IdRef(n), Operand::Decoration(x), Operand::LiteralBit32(v)] if *n == target && *x == d => Some(*v),
            _ => None,
        })
    };
    let member_offset = |st: u32, k: u32| -> Option<u32> {
        m.annotations.iter().find_map(|a| match a.operands.as_slice() {
            [Operand::IdRef(n), Operand::LiteralBit32(mk), Operand::Decoration(Decoration::Offset), Operand::LiteralBit32(v)]
                if *n == st && *mk == k => Some(*v),
            _ => None,
        })
    };
    let is32 = |t: u32| defs.get(&t).is_some_and(|d| matches!(d.class.opcode, Op::TypeInt | Op::TypeFloat) && d.operands.first().and_then(lit) == Some(32));
    let pointee = |ptr_ty: u32| -> Option<u32> {
        let d = defs.get(&ptr_ty)?;
        (d.class.opcode == Op::TypePointer && d.operands.first() == Some(&Operand::StorageClass(StorageClass::PhysicalStorageBuffer)))
            .then(|| d.operands.get(1).and_then(id)).flatten()
    };
    let step = |ty: u32, ix: u32| -> Option<(u32, u32)> {
        let d = defs.get(&ty)?;
        let k = *consts.get(&ix)?;
        match d.class.opcode {
            Op::TypeStruct => { let t = d.operands.get(k as usize).and_then(id)?; Some((t, member_offset(ty, k)?)) }
            Op::TypeArray | Op::TypeRuntimeArray => { let t = d.operands.first().and_then(id)?; Some((t, deco(ty, Decoration::ArrayStride)?.checked_mul(k)?)) }
            Op::TypeVector => { let t = d.operands.first().and_then(id)?; if !is32(t) { return None; } Some((t, 4 * k)) }
            _ => None,
        }
    };
    let mut sites: Vec<Site> = Vec::new();
    let mut bad: u32 = 0;
    let mut seen: u32 = 0;
    for (fi, f) in m.functions.iter().enumerate() {
        let local: HashMap<u32, &Instruction> = f.blocks.iter().flat_map(|b| b.instructions.iter()).filter_map(|i| Some((i.result_id?, i))).collect();
        let mut root: HashMap<u32, u32> = HashMap::new();
        for i in local.values() {
            if i.class.opcode != Op::ConvertUToPtr { continue; }
            let Some(x) = i.operands.first().and_then(id).and_then(|v| local.get(&v)) else { continue };
            if x.class.opcode != Op::Load { continue; }
            let Some(ac) = x.operands.first().and_then(id).and_then(|v| local.get(&v)) else { continue };
            let ao: Vec<u32> = ac.operands.iter().filter_map(id).collect();
            if matches!(ac.class.opcode, Op::AccessChain | Op::InBoundsAccessChain) && ao.len() == 3 && ao[0] == table
                && consts.get(&ao[1]) == Some(&0) {
                if let Some(&b) = consts.get(&ao[2]) { if b < SLOTS { root.insert(i.result_id.unwrap(), b); } }
            }
        }
        if root.is_empty() { continue; }
        for &b in root.values() { seen |= 1 << b; }
        let mut derived: HashMap<u32, (u32, u32, u32)> = HashMap::new();
        for (&p, &b) in &root {
            let t = local.get(&p).and_then(|i| i.result_type).and_then(pointee);
            match t { Some(t) => { derived.insert(p, (b, 0, t)); } None => { bad |= 1 << b; } }
        }
        loop {
            let mut grew = false;
            for i in f.blocks.iter().flat_map(|b| b.instructions.iter()) {
                let Some(n) = i.result_id else { continue };
                if derived.contains_key(&n) { continue; }
                if !matches!(i.class.opcode, Op::AccessChain | Op::InBoundsAccessChain) { continue; }
                let ops: Vec<u32> = i.operands.iter().filter_map(id).collect();
                let Some(&(b, off, ty)) = ops.first().and_then(|base| derived.get(base)) else { continue };
                let mut t = ty; let mut o = off; let mut ok = true;
                for ix in &ops[1..] { match step(t, *ix) { Some((nt, d)) => { t = nt; o += d; } None => { ok = false; break; } } }
                if !ok || i.result_type.and_then(pointee) != Some(t) { bad |= 1 << b; continue; }
                derived.insert(n, (b, o, t)); grew = true;
            }
            if !grew { break; }
        }
        for (bi, blk) in f.blocks.iter().enumerate() {
            for (ii, i) in blk.instructions.iter().enumerate() {
                let uses: Vec<u32> = i.operands.iter().filter_map(id).filter(|v| derived.contains_key(v)).collect();
                if uses.is_empty() { continue; }
                let (b, off, ty) = derived[&uses[0]];
                match i.class.opcode {
                    Op::AccessChain | Op::InBoundsAccessChain if i.result_id.is_some_and(|n| derived.contains_key(&n))
                        && uses.len() == 1 && i.operands.first().and_then(id) == Some(uses[0]) => {}
                    Op::Load if uses.len() == 1 && i.operands.first().and_then(id) == Some(uses[0]) => {
                        let volatile = matches!(i.operands.get(1), Some(Operand::MemoryAccess(a)) if a.contains(MemoryAccess::VOLATILE));
                        let (lanes, lane) = match defs.get(&ty) {
                            Some(d) if d.class.opcode == Op::TypeVector => (d.operands.get(1).and_then(lit).unwrap_or(0), d.operands.first().and_then(id).unwrap_or(0)),
                            _ => (1, ty),
                        };
                        if volatile || !is32(lane) || !(1..=4).contains(&lanes) || off % 4 != 0 || off + 4 * lanes > SLOT_BYTES
                            || i.result_type != Some(ty) {
                            bad |= 1 << b;
                        } else {
                            sites.push(Site { f: fi, b: bi, i: ii, binding: b, offset: off, lanes, lane_ty: lane, result_ty: ty });
                        }
                    }
                    _ => { for u in uses { bad |= 1 << derived[&u].0; } }
                }
            }
        }
    }
    let eligible = seen & !bad & sites.iter().fold(0, |a, s| a | 1 << s.binding);
    let want = mask & eligible;
    if want == 0 { return Ok((bytes.to_vec(), eligible)); }
    let ptr_ty = tv.result_type.ok_or("table without a type")?;
    let block = defs.get(&ptr_ty).and_then(|p| p.operands.get(1)).and_then(id).ok_or("table pointer without a pointee")?;
    if defs.get(&block).map(|s| s.operands.len()) != Some(1) { return Err("table block is not the one-member G1 block".into()); }
    let mut next = m.header.as_ref().ok_or("no SPIR-V header")?.bound;
    let mut fresh = || { let n = next; next += 1; n };
    let uint = defs.iter().find_map(|(&n, i)| (i.class.opcode == Op::TypeInt && i.operands == vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)]).then_some(n));
    let uint = match uint { Some(u) => u, None => { let n = fresh(); m.types_global_values.push(Instruction::new(Op::TypeInt, None, Some(n), vec![Operand::LiteralBit32(32), Operand::LiteralBit32(0)])); n } };
    let words = SLOTS * SLOT_BYTES / 4;
    let (cwords, arr, puint) = (fresh(), fresh(), fresh());
    let mut cidx: HashMap<u32, u32> = HashMap::new();
    let mut new_consts = vec![(cwords, words)];
    let mut want_const = |v: u32, new: &mut Vec<(u32, u32)>, fresh: &mut dyn FnMut() -> u32| -> u32 {
        *cidx.entry(v).or_insert_with(|| { let n = fresh(); new.push((n, v)); n })
    };
    let one = want_const(1, &mut new_consts, &mut fresh);
    let mut plan: Vec<(Site, Vec<u32>)> = Vec::new();
    for s in sites.iter().filter(|s| want & (1 << s.binding) != 0) {
        let base = (s.binding * SLOT_BYTES + s.offset) / 4;
        let ks: Vec<u32> = (0..s.lanes).map(|k| want_const(base + k, &mut new_consts, &mut fresh)).collect();
        plan.push((s.clone(), ks));
    }
    let take = |n: u32, m: &mut rspirv::dr::Module| -> Result<Instruction, String> {
        let p = m.types_global_values.iter().position(|i| i.result_id == Some(n)).ok_or("table type missing")?;
        Ok(m.types_global_values.remove(p))
    };
    let mut block_i = take(block, &mut m)?;
    let ptr_i = take(ptr_ty, &mut m)?;
    let var_i = take(table, &mut m)?;
    for &(n, v) in &new_consts { m.types_global_values.push(Instruction::new(Op::Constant, Some(uint), Some(n), vec![Operand::LiteralBit32(v)])); }
    m.types_global_values.push(Instruction::new(Op::TypeArray, None, Some(arr), vec![Operand::IdRef(uint), Operand::IdRef(cwords)]));
    block_i.operands.push(Operand::IdRef(arr));
    m.types_global_values.extend([block_i, ptr_i, var_i,
        Instruction::new(Op::TypePointer, None, Some(puint), vec![Operand::StorageClass(StorageClass::Uniform), Operand::IdRef(uint)])]);
    m.annotations.extend([
        Instruction::new(Op::Decorate, None, None, vec![Operand::IdRef(arr), Operand::Decoration(Decoration::ArrayStride), Operand::LiteralBit32(4)]),
        Instruction::new(Op::MemberDecorate, None, None, vec![Operand::IdRef(block), Operand::LiteralBit32(1), Operand::Decoration(Decoration::Offset), Operand::LiteralBit32(INLINE_OFFSET)]),
    ]);
    plan.sort_by(|a, b| (b.0.f, b.0.b, b.0.i).cmp(&(a.0.f, a.0.b, a.0.i)));
    for (s, ks) in plan {
        let result = m.functions[s.f].blocks[s.b].instructions[s.i].result_id.ok_or("load without a result")?;
        let mut seq = Vec::new();
        let mut lanes = Vec::new();
        for k in &ks {
            let (ac, w) = (fresh(), fresh());
            seq.push(Instruction::new(Op::AccessChain, Some(puint), Some(ac), vec![Operand::IdRef(table), Operand::IdRef(one), Operand::IdRef(*k)]));
            seq.push(Instruction::new(Op::Load, Some(uint), Some(w), vec![Operand::IdRef(ac)]));
            if s.lane_ty == uint { lanes.push(w); } else {
                let c = fresh();
                seq.push(Instruction::new(Op::Bitcast, Some(s.lane_ty), Some(c), vec![Operand::IdRef(w)]));
                lanes.push(c);
            }
        }
        let last = if s.lanes == 1 {
            Instruction::new(Op::CopyObject, Some(s.result_ty), Some(result), vec![Operand::IdRef(lanes[0])])
        } else {
            Instruction::new(Op::CompositeConstruct, Some(s.result_ty), Some(result), lanes.iter().map(|&l| Operand::IdRef(l)).collect())
        };
        seq.push(last);
        let insts = &mut m.functions[s.f].blocks[s.b].instructions;
        insts.splice(s.i..=s.i, seq);
    }
    m.header.as_mut().unwrap().bound = next;
    Ok((m.assemble().iter().flat_map(|w| w.to_le_bytes()).collect(), eligible))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer_addresses::lower;
    use spirv_tools::{assembler::Assembler, val::Validator};
    const K: &str = r#"
OpCapability Shader
OpMemoryModel Logical GLSL450
OpEntryPoint GLCompute %main "main" %args %data %out %gid
OpExecutionMode %main LocalSize 1 1 1
OpDecorate %gid BuiltIn GlobalInvocationId
OpDecorate %ablock Block
OpMemberDecorate %ablock 0 Offset 0
OpMemberDecorate %ablock 1 Offset 4
OpDecorate %arr ArrayStride 4
OpDecorate %dblock Block
OpMemberDecorate %dblock 0 Offset 0
OpDecorate %args DescriptorSet 0
OpDecorate %args Binding 0
OpDecorate %data DescriptorSet 0
OpDecorate %data Binding 1
OpDecorate %out DescriptorSet 0
OpDecorate %out Binding 2
%void = OpTypeVoid
%uint = OpTypeInt 32 0
%float = OpTypeFloat 32
%v3 = OpTypeVector %uint 3
%pin = OpTypePointer Input %v3
%gid = OpVariable %pin Input
%zero = OpConstant %uint 0
%one = OpConstant %uint 1
%ablock = OpTypeStruct %uint %float
%arr = OpTypeRuntimeArray %float
%dblock = OpTypeStruct %arr
%pa = OpTypePointer StorageBuffer %ablock
%pd = OpTypePointer StorageBuffer %dblock
%pu = OpTypePointer StorageBuffer %uint
%pf = OpTypePointer StorageBuffer %float
%args = OpVariable %pa StorageBuffer
%data = OpVariable %pd StorageBuffer
%out = OpVariable %pd StorageBuffer
%fn = OpTypeFunction %void
%main = OpFunction %void None %fn
%label = OpLabel
%g3 = OpLoad %v3 %gid
%g = OpCompositeExtract %uint %g3 0
%pn = OpAccessChain %pu %args %zero
%n = OpLoad %uint %pn
%ps = OpAccessChain %pf %args %one
%s = OpLoad %float %ps
%pdx = OpAccessChain %pf %data %zero %g
%x = OpLoad %float %pdx
%y = OpFMul %float %x %s
%i = OpIAdd %uint %g %n
%po = OpAccessChain %pf %out %zero %i
OpStore %po %y
OpReturn
OpFunctionEnd
"#;
    fn lowered_g1() -> Vec<u8> {
        let b = spirv_tools::assembler::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).assemble(K, Default::default()).unwrap().as_bytes().to_vec();
        crate::table_ubo::convert(&lower(&b).unwrap()).unwrap()
    }
    fn validate_scalar(b: &[u8]) {
        let w: Vec<u32> = b.chunks_exact(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect();
        let mut o = spirv_tools::val::ValidatorOptions::default(); o.scalar_block_layout = true;
        spirv_tools::val::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).validate(&w, Some(o)).unwrap();
    }
    #[test] fn only_the_constant_offset_argument_binding_is_eligible() {
        let (out, elig) = apply(&lowered_g1(), 0).unwrap();
        assert_eq!(elig, 0b1, "binding 0 (two constant-offset 32-bit loads) only; 1 is indexed, 2 is written: {elig:#b}");
        assert_eq!(out, lowered_g1(), "mask 0 must return the input byte for byte");
    }
    #[test] fn an_inlined_binding_reads_the_table_block_and_validates() {
        let (out, elig) = apply(&lowered_g1(), 0b1).unwrap();
        assert_eq!(elig, 0b1);
        validate_scalar(&out);
        let m = rspirv::dr::load_bytes(&out).unwrap();
        let block_members = m.types_global_values.iter().filter(|i| i.class.opcode == Op::TypeStruct)
            .map(|i| i.operands.len()).max().unwrap();
        assert!(block_members >= 2, "the table block gained the inline member");
        let tys: std::collections::HashMap<u32, u32> = m.all_inst_iter().filter_map(|i| Some((i.result_id?, i.result_type?))).collect();
        let uniform_uint_ptr: std::collections::HashSet<u32> = m.types_global_values.iter().filter(|i| i.class.opcode == Op::TypePointer
            && i.operands.first() == Some(&Operand::StorageClass(StorageClass::Uniform))
            && m.types_global_values.iter().any(|t| Some(t.result_id.unwrap_or(0)) == i.operands.get(1).and_then(id) && t.class.opcode == Op::TypeInt
                && t.operands.first() == Some(&Operand::LiteralBit32(32)))).filter_map(|i| i.result_id).collect();
        let inline_loads = m.all_inst_iter().filter(|i| i.class.opcode == Op::Load
            && i.operands.first().and_then(id).and_then(|p| tys.get(&p)).is_some_and(|t| uniform_uint_ptr.contains(t))).count();
        assert_eq!(inline_loads, 2, "the uint at 0 and the float at 4 each became one Uniform word load");
    }
    #[test] fn an_ineligible_binding_in_the_mask_is_ignored() {
        let (out, _) = apply(&lowered_g1(), 0b110).unwrap();
        assert_eq!(out, lowered_g1(), "bindings 1 and 2 are not eligible: nothing changes");
    }
    #[test] fn a_module_without_the_uniform_table_reports_nothing() {
        let b = spirv_tools::assembler::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).assemble(K, Default::default()).unwrap().as_bytes().to_vec();
        let low = lower(&b).unwrap();
        assert_eq!(apply(&low, u32::MAX).unwrap(), (low.clone(), 0));
    }
}
