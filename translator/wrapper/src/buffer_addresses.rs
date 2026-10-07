use rspirv::{binary::Assemble, dr::{Instruction, Operand}, spirv::*};
use std::collections::{HashMap, HashSet};

pub const TABLE_BINDING: u32 = metal2vulkan::reflect::SYNTHETIC_BINDING_BASE;

fn inst(op: Op, ty: Option<u32>, id: Option<u32>, args: Vec<Operand>) -> Instruction {
    Instruction::new(op, ty, id, args)
}
fn id(o: &Operand) -> Option<u32> { if let Operand::IdRef(v) = o { Some(*v) } else { None } }
fn lit(o: &Operand) -> Option<u32> { if let Operand::LiteralBit32(v) = o { Some(*v) } else { None } }
fn scalar_align(ty: u32, defs: &HashMap<u32, Instruction>, depth: usize) -> Result<u32, String> {
    if depth > 32 { return Err("recursive buffer type".into()); }
    let t = defs.get(&ty).ok_or("missing buffer type")?;
    match t.class.opcode {
        Op::TypeInt | Op::TypeFloat => Ok(lit(&t.operands[0]).ok_or("scalar width")? / 8),
        Op::TypePointer => Ok(8),
        Op::TypeVector | Op::TypeMatrix | Op::TypeArray | Op::TypeRuntimeArray => scalar_align(id(&t.operands[0]).ok_or("element type")?, defs, depth+1),
        Op::TypeStruct => t.operands.iter().map(|o| scalar_align(id(o).ok_or("member type")?, defs, depth+1)).try_fold(1, |a,b| b.map(|b| a.max(b))),
        _ => Err(format!("unsupported buffer pointee {:?}", t.class.opcode)),
    }
}

fn keep_stated_align(scalar: u32, stated: Option<&Operand>) -> u32 {
    if std::env::var_os("NVMTL_NO_AIR_ALIGN").is_some() { return scalar; }
    match stated { Some(Operand::LiteralBit32(a)) if a.is_power_of_two() && *a <= 16 => scalar.max(*a), _ => scalar }
}

pub fn lower(bytes: &[u8]) -> Result<Vec<u8>, String> {
    match crate::linearize::apply(bytes) {
        Ok((lin, _why)) => lower_addresses(&lin),
        Err(_) => lower_addresses(bytes),
    }
}
fn lower_addresses(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut m = rspirv::dr::load_bytes(bytes).map_err(|e| e.to_string())?;
    let defs: HashMap<_,_> = m.types_global_values.iter().filter_map(|i| i.result_id.map(|n|(n,i.clone()))).collect();
    let sb_types: HashMap<u32,u32> = defs.iter().filter_map(|(&n,i)| {
        (i.class.opcode == Op::TypePointer && i.operands.first() == Some(&Operand::StorageClass(StorageClass::StorageBuffer)))
            .then(|| (n, id(&i.operands[1]).unwrap()))
    }).collect();
    let mut bindings = HashMap::new(); let mut sets = HashMap::new();
    for a in &m.annotations {
        if a.class.opcode == Op::Decorate && a.operands.len() >= 3 {
            if let (Some(n), Some(v)) = (id(&a.operands[0]), lit(&a.operands[2])) {
                match a.operands[1] { Operand::Decoration(Decoration::Binding) => { bindings.insert(n,v); }, Operand::Decoration(Decoration::DescriptorSet) => { sets.insert(n,v); }, _ => {} }
            }
        }
    }
    let roots: Vec<(u32,u32,u32)> = m.types_global_values.iter().filter_map(|i| {
        (i.class.opcode == Op::Variable && i.operands.first() == Some(&Operand::StorageClass(StorageClass::StorageBuffer)))
            .then(|| (i.result_id.unwrap(),i.result_type.unwrap(),*bindings.get(&i.result_id.unwrap()).unwrap_or(&u32::MAX)))
    }).collect();
    if roots.is_empty() { return Ok(bytes.to_vec()); }
    if bindings.values().any(|b| *b == TABLE_BINDING) { return Ok(bytes.to_vec()); }
    for &(n,_,b) in &roots {
        if b >= 32 || sets.get(&n) != Some(&0) { return Err(format!("unsupported source buffer descriptor {n}, binding {b}")); }
    }
    let root_ids: HashSet<_> = roots.iter().map(|r|r.0).collect();
    let mut value_types = HashMap::new();
    for i in m.all_inst_iter() { if let (Some(n),Some(t)) = (i.result_id,i.result_type) { value_types.insert(n,t); } }
    let wide_hints = crate::wide_loads::hints(&m, &defs, &sb_types, &value_types);
    for i in m.all_inst_iter() {
        if matches!(i.class.opcode, Op::ArrayLength | Op::PtrEqual | Op::PtrNotEqual | Op::PtrDiff | Op::CopyMemory | Op::CopyMemorySized) && i.operands.iter().filter_map(id).any(|n|value_types.get(&n).is_some_and(|t|sb_types.contains_key(t))) {
            return Err(format!("buffer address lowering does not yet support {:?}",i.class.opcode));
        }
        if i.class.opcode == Op::ConstantNull && i.result_type.is_some_and(|t|sb_types.contains_key(&t)) { return Err("logical null buffer pointer requires explicit lowering".into()); }
    }
    let mut next = m.header.as_ref().ok_or("no SPIR-V header")?.bound;
    let mut fresh = || { let n=next; next+=1; n };
    let uint = defs.iter().find_map(|(&n,i)| (i.class.opcode == Op::TypeInt && i.operands == vec![Operand::LiteralBit32(32),Operand::LiteralBit32(0)]).then_some(n)).unwrap_or_else(|| { let n=fresh(); m.types_global_values.push(inst(Op::TypeInt,None,Some(n),vec![Operand::LiteralBit32(32),Operand::LiteralBit32(0)])); n });
    let ulong = defs.iter().find_map(|(&n,i)| (i.class.opcode == Op::TypeInt && i.operands == vec![Operand::LiteralBit32(64),Operand::LiteralBit32(0)]).then_some(n)).unwrap_or_else(|| { let n=fresh(); m.types_global_values.push(inst(Op::TypeInt,None,Some(n),vec![Operand::LiteralBit32(64),Operand::LiteralBit32(0)])); n });
    for t in &mut m.types_global_values {
        if t.result_id.is_some_and(|n|sb_types.contains_key(&n)) { t.operands[0]=Operand::StorageClass(StorageClass::PhysicalStorageBuffer); }
    }
    m.types_global_values.retain(|i|!i.result_id.is_some_and(|n|root_ids.contains(&n)));
    m.annotations.retain(|i| !i.operands.first().and_then(id).is_some_and(|n|root_ids.contains(&n)));
    m.debug_names.retain(|i| !i.operands.first().and_then(id).is_some_and(|n|root_ids.contains(&n)));
    let arr=fresh(); let block=fresh(); let ptr_block=fresh(); let ptr_ulong=fresh(); let table=fresh();
    m.types_global_values.extend([
        inst(Op::TypeRuntimeArray,None,Some(arr),vec![Operand::IdRef(ulong)]),
        inst(Op::TypeStruct,None,Some(block),vec![Operand::IdRef(arr)]),
        inst(Op::TypePointer,None,Some(ptr_block),vec![Operand::StorageClass(StorageClass::StorageBuffer),Operand::IdRef(block)]),
        inst(Op::TypePointer,None,Some(ptr_ulong),vec![Operand::StorageClass(StorageClass::StorageBuffer),Operand::IdRef(ulong)]),
        inst(Op::Variable,Some(ptr_block),Some(table),vec![Operand::StorageClass(StorageClass::StorageBuffer)]),
    ]);
    m.annotations.extend([
        inst(Op::Decorate,None,None,vec![Operand::IdRef(arr),Operand::Decoration(Decoration::ArrayStride),Operand::LiteralBit32(8)]),
        inst(Op::Decorate,None,None,vec![Operand::IdRef(block),Operand::Decoration(Decoration::Block)]),
        inst(Op::MemberDecorate,None,None,vec![Operand::IdRef(block),Operand::LiteralBit32(0),Operand::Decoration(Decoration::Offset),Operand::LiteralBit32(0)]),
        inst(Op::Decorate,None,None,vec![Operand::IdRef(table),Operand::Decoration(Decoration::DescriptorSet),Operand::LiteralBit32(0)]),
        inst(Op::Decorate,None,None,vec![Operand::IdRef(table),Operand::Decoration(Decoration::Binding),Operand::LiteralBit32(TABLE_BINDING)]),
        inst(Op::Decorate,None,None,vec![Operand::IdRef(table),Operand::Decoration(Decoration::NonWritable)]),
    ]);
    let mut constants=HashMap::new();
    for b in std::iter::once(0).chain(roots.iter().map(|r|r.2)) {
        constants.entry(b).or_insert_with(|| { let n=fresh(); m.types_global_values.push(inst(Op::Constant,Some(uint),Some(n),vec![Operand::LiteralBit32(b)])); n });
    }
    for e in &mut m.entry_points {
        e.operands.retain(|o|!id(o).is_some_and(|n|root_ids.contains(&n)));
        if m.header.as_ref().unwrap().version >= 0x00010400 { e.operands.push(Operand::IdRef(table)); }
    }
    for cap in [Capability::Int64,Capability::PhysicalStorageBufferAddresses] {
        if !m.capabilities.iter().any(|i|i.operands==vec![Operand::Capability(cap)]) { m.capabilities.push(inst(Op::Capability,None,None,vec![Operand::Capability(cap)])); }
    }
    let ext=Operand::LiteralString("SPV_KHR_physical_storage_buffer".into());
    if !m.extensions.iter().any(|i|i.operands==vec![ext.clone()]) { m.extensions.push(inst(Op::Extension,None,None,vec![ext])); }
    m.memory_model.as_mut().ok_or("missing memory model")?.operands[0]=Operand::AddressingModel(AddressingModel::PhysicalStorageBuffer64);
    for f in &mut m.functions {
        if f.blocks.is_empty() { continue; }
        let mut replace=HashMap::new(); let mut prelude=Vec::new();
        for &(root,ty,b) in &roots {
            let p=fresh(); let addr=fresh(); let physical=fresh();
            prelude.extend([
                inst(Op::AccessChain,Some(ptr_ulong),Some(p),vec![Operand::IdRef(table),Operand::IdRef(constants[&0]),Operand::IdRef(constants[&b])]),
                inst(Op::Load,Some(ulong),Some(addr),vec![Operand::IdRef(p)]),
                inst(Op::ConvertUToPtr,Some(ty),Some(physical),vec![Operand::IdRef(addr)]),
            ]); replace.insert(root,physical);
        }
        for param in &f.parameters {
            if param.result_type.is_some_and(|t|sb_types.contains_key(&t)) {
                let n=param.result_id.unwrap();
                m.annotations.retain(|i| !(i.class.opcode==Op::Decorate && i.operands.first()==Some(&Operand::IdRef(n)) && matches!(i.operands.get(1),Some(Operand::Decoration(Decoration::Restrict|Decoration::Aliased)))));
                m.annotations.push(inst(Op::Decorate,None,None,vec![Operand::IdRef(n),Operand::Decoration(Decoration::Aliased)]));
            }
        }
        for b in &mut f.blocks { for i in &mut b.instructions {
            let mem = match i.class.opcode { Op::Load => Some(1),Op::Store => Some(2), _=>None };
            if let Some(at)=mem {
                if let Some(ty)=i.operands.first().and_then(id).and_then(|n|value_types.get(&n)).and_then(|t|sb_types.get(t)) {
                    let align=scalar_align(*ty,&defs,0)?;
                    if let Some(Operand::MemoryAccess(flags))=i.operands.get(at).cloned() {
                        if flags.contains(MemoryAccess::ALIGNED) { i.operands[at+1]=Operand::LiteralBit32(keep_stated_align(align,i.operands.get(at+1))); }
                        else { i.operands[at]=Operand::MemoryAccess(flags|MemoryAccess::ALIGNED); i.operands.insert(at+1,Operand::LiteralBit32(align)); }
                    } else { i.operands.extend([Operand::MemoryAccess(MemoryAccess::ALIGNED),Operand::LiteralBit32(align)]); }
                }
            }
            for o in &mut i.operands { if let Operand::IdRef(n)=o { if let Some(r)=replace.get(n) { *n=*r; } } }
        }}
        let at=f.blocks[0].instructions.iter().position(|i|i.class.opcode!=Op::Variable).unwrap_or(f.blocks[0].instructions.len());
        f.blocks[0].instructions.splice(at..at,prelude);
    }
    if std::env::var_os("NVMTL_NO_WIDE_WORD_LOAD").is_none() { crate::wide_loads::fuse(&mut m, &mut next, uint, ulong, &wide_hints); }
    m.header.as_mut().unwrap().bound=next;
    let result:Vec<u8>=m.assemble().iter().flat_map(|w|w.to_le_bytes()).collect();
    metal2vulkan::canonicalize_spirv_bytes(&result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use spirv_tools::{assembler::Assembler, val::Validator};
    const FIXTURE: &str = r#"
OpCapability Shader
OpMemoryModel Logical GLSL450
OpEntryPoint GLCompute %main "main" %buf
OpExecutionMode %main LocalSize 1 1 1
OpDecorate %arr ArrayStride 4
OpDecorate %block Block
OpMemberDecorate %block 0 Offset 0
OpDecorate %buf DescriptorSet 0
OpDecorate %buf Binding 0
%void = OpTypeVoid
%uint = OpTypeInt 32 0
%zero = OpConstant %uint 0
%one = OpConstant %uint 1
%arr = OpTypeRuntimeArray %uint
%block = OpTypeStruct %arr
%pb = OpTypePointer StorageBuffer %block
%pu = OpTypePointer StorageBuffer %uint
%buf = OpVariable %pb StorageBuffer
%fn = OpTypeFunction %void
%main = OpFunction %void None %fn
%label = OpLabel
%p = OpAccessChain %pu %buf %zero %one
%x = OpLoad %uint %p
%y = OpIAdd %uint %x %one
OpStore %p %y
OpReturn
OpFunctionEnd
"#;
    fn source(text: &str) -> Vec<u8> {
        spirv_tools::assembler::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).assemble(text,Default::default()).unwrap().as_bytes().to_vec()
    }
    fn validate(b: &[u8]) {
        let w:Vec<u32>=b.chunks_exact(4).map(|w|u32::from_le_bytes(w.try_into().unwrap())).collect();
        spirv_tools::val::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).validate(&w,None).unwrap();
    }
    #[test] fn loads_and_stores_use_exact_address_table() {
        let input=source(FIXTURE); validate(&input); let output=lower(&input).unwrap(); validate(&output);
        let m=rspirv::dr::load_bytes(&output).unwrap();
        assert!(m.all_inst_iter().any(|i|i.class.opcode==Op::ConvertUToPtr));
        assert!(m.annotations.iter().any(|i|i.operands.ends_with(&[Operand::Decoration(Decoration::Binding),Operand::LiteralBit32(640)])));
    }
    #[test] fn keeps_memory_access_flags() {
        let input=source(&FIXTURE.replace("%x = OpLoad %uint %p","%x = OpLoad %uint %p Volatile|Aligned 16").replace("OpStore %p %y","OpStore %p %y Volatile"));
        let output=lower(&input).unwrap(); validate(&output);
        let m=rspirv::dr::load_bytes(&output).unwrap();
        assert!(m.all_inst_iter().any(|i|i.operands.contains(&Operand::MemoryAccess(MemoryAccess::VOLATILE|MemoryAccess::ALIGNED))));
    }
    #[test] fn a_module_that_already_has_the_translators_table_is_left_byte_identical() {
        let input=source(&FIXTURE.replace("Binding 0","Binding 640")); assert_eq!(lower(&input).unwrap(), input);
    }
    #[test] #[ignore] fn corpus_lowered_shaders_still_validate() {
        use metal2vulkan::passes::Stage; use std::collections::BTreeMap;
        let check=|b:&[u8]|->Result<(),String>{ let w:Vec<u32>=b.chunks_exact(4).map(|w|u32::from_le_bytes(w.try_into().unwrap())).collect();
            spirv_tools::val::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).validate(&w,None).map_err(|e|e.to_string()) };
        let mut files:Vec<_>=std::fs::read_dir("/Library/GPUBundles/nvmtl/aircache").unwrap().filter_map(|e|e.ok()).map(|e|e.path()).collect(); files.sort();
        let t0=std::time::Instant::now();
        let (mut units,mut lowered,mut refused,mut passthrough,mut ok_ok,mut bad_bad,mut fixed,mut broke)=(0,0,0,0,0,0,0,0);
        let mut broke_why:BTreeMap<String,(u32,String)>=BTreeMap::new();
        for f in files.iter().step_by(4) {
            if t0.elapsed().as_secs()>480 { println!("STOPPED EARLY at the 480 s cap"); break; }
            let ll=match std::fs::read_to_string(f){Ok(s)=>s,Err(_)=>continue};
            for (tag,st) in [("!air.vertex =",Stage::Vertex),("!air.fragment =",Stage::Fragment),("!air.kernel =",Stage::Kernel)] {
                if !ll.contains(tag) { continue; }
                let Ok(spv)=metal2vulkan::translate_native_no_retry(&ll,st) else { continue };
                units+=1;
                let out=match lower(&spv){Ok(o)=>o,Err(_)=>{refused+=1;continue}};
                if out==spv { passthrough+=1; continue; }
                lowered+=1;
                match (check(&spv).is_ok(),check(&out)) {
                    (true,Ok(()))=>ok_ok+=1, (false,Err(_))=>bad_bad+=1, (false,Ok(()))=>fixed+=1,
                    (true,Err(e))=>{ broke+=1; let k:String=e.chars().filter(|c|!c.is_ascii_digit()).take(140).collect();
                        let v=broke_why.entry(k).or_insert((0,f.file_name().unwrap().to_string_lossy().into())); v.0+=1; }
                }
            }
        }
        println!("CORPUS stride 4: units {units}  lowered {lowered}  refused {refused}  passthrough {passthrough}  in {} s",t0.elapsed().as_secs());
        println!("  valid->valid {ok_ok}   invalid->invalid {bad_bad} (translator baseline)   invalid->valid {fixed}   valid->INVALID {broke}");
        for (k,(n,eg)) in &broke_why { println!("    {n:5}  {k}   e.g. {eg}"); }
        assert!(lowered>100,"the corpus arm lowered only {lowered} shaders — it measured nothing");
        assert_eq!(broke,0,"{broke} shaders were valid before lowering and INVALID after");
    }
    #[test] fn rejects_unknown_descriptor_abi() {
        assert!(lower(&source(&FIXTURE.replace("Binding 0","Binding 32"))).unwrap_err().contains("unsupported source buffer"));
    }
}
