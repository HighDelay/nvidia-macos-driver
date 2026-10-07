use rspirv::binary::Assemble;
use rspirv::dr::{Instruction, Module, Operand};
use rspirv::spirv::{BuiltIn, Decoration, ExecutionModel, Op, StorageClass};
use std::collections::HashMap;

pub const LINEARIZE_SPEC_ID: u32 = 3;

fn idr(o: &Operand) -> Option<u32> { if let Operand::IdRef(v) = o { Some(*v) } else { None } }

pub fn apply(bytes: &[u8]) -> Result<(Vec<u8>, Option<&'static str>), String> {
    if std::env::var_os("NVMTL_NO_TG_LINEARIZE").is_some() { return Ok((bytes.to_vec(), Some("NVMTL_NO_TG_LINEARIZE"))); }
    let mut m = rspirv::dr::load_bytes(bytes).map_err(|e| e.to_string())?;
    match rewrite(&mut m) {
        Ok(()) => Ok((m.assemble().iter().flat_map(|w| w.to_le_bytes()).collect(), None)),
        Err(why) => Ok((bytes.to_vec(), Some(why))),
    }
}

fn rewrite(m: &mut Module) -> Result<(), &'static str> {
    if !m.entry_points.iter().any(|e| e.operands.first() == Some(&Operand::ExecutionModel(ExecutionModel::GLCompute))) {
        return Err("not a compute module");
    }
    let mut spec_of: HashMap<u32, u32> = HashMap::new();
    let mut builtin_var: HashMap<BuiltIn, u32> = HashMap::new();
    let mut wg_dec: Option<usize> = None;
    for (k, a) in m.annotations.iter().enumerate() {
        if a.class.opcode != Op::Decorate { continue; }
        match (a.operands.first().and_then(idr), a.operands.get(1), a.operands.get(2)) {
            (Some(t), Some(Operand::Decoration(Decoration::SpecId)), Some(Operand::LiteralBit32(v))) => { spec_of.insert(t, *v); }
            (Some(t), Some(Operand::Decoration(Decoration::BuiltIn)), Some(Operand::BuiltIn(b))) => {
                if *b == BuiltIn::WorkgroupSize { wg_dec = Some(k); } else { builtin_var.insert(*b, t); }
            }
            _ => {}
        }
    }
    if spec_of.values().any(|v| *v == LINEARIZE_SPEC_ID) { return Err("SpecId 3 already taken"); }
    let wg_dec = wg_dec.ok_or("no WorkgroupSize builtin")?;
    let w = m.annotations[wg_dec].operands.first().and_then(idr).ok_or("bad WorkgroupSize decoration")?;
    let defs: HashMap<u32, Instruction> =
        m.types_global_values.iter().filter_map(|i| i.result_id.map(|n| (n, i.clone()))).collect();
    let wi = defs.get(&w).ok_or("WorkgroupSize target not global")?;
    if wi.class.opcode != Op::SpecConstantComposite { return Err("WorkgroupSize is not a spec-constant composite"); }
    let comps: Vec<u32> = wi.operands.iter().filter_map(idr).collect();
    if comps.len() != 3 { return Err("WorkgroupSize is not 3 components"); }
    for (d, c) in comps.iter().enumerate() {
        if spec_of.get(c) != Some(&(d as u32)) { return Err("WorkgroupSize components are not SpecIds 0/1/2"); }
    }
    let (x, y, z) = (comps[0], comps[1], comps[2]);
    let v3u = wi.result_type.ok_or("composite without type")?;
    let uint = match defs.get(&v3u) {
        Some(i) if i.class.opcode == Op::TypeVector => i.operands.first().and_then(idr).ok_or("vector without component")?,
        _ => return Err("WorkgroupSize type is not a vector"),
    };
    let lid = builtin_var.get(&BuiltIn::LocalInvocationId).copied();
    let gid = builtin_var.get(&BuiltIn::GlobalInvocationId).copied();

    let const_u32 = |id: u32| -> Option<u32> {
        let d = defs.get(&id)?;
        if d.class.opcode != Op::Constant { return None; }
        if let Some(Operand::LiteralBit32(v)) = d.operands.first() { Some(*v) } else { None }
    };
    let mut chain: HashMap<u32, (u32, u32)> = HashMap::new();
    for f in &m.functions { for b in &f.blocks { for i in &b.instructions {
        if i.class.opcode == Op::AccessChain || i.class.opcode == Op::InBoundsAccessChain {
            let base = i.operands.first().and_then(idr);
            if base.is_some() && (base == lid || base == gid) {
                if i.operands.len() != 2 { return Err("multi-index chain into an id builtin"); }
                let c = i.operands.get(1).and_then(idr).and_then(|c| const_u32(c)).ok_or("non-constant component index")?;
                if c > 2 { return Err("component index out of range"); }
                chain.insert(i.result_id.unwrap(), (base.unwrap(), c));
            }
        }
    }}}
    for f in &m.functions { for b in &f.blocks { for i in &b.instructions {
        let is_load = i.class.opcode == Op::Load;
        for (k, o) in i.operands.iter().enumerate() {
            let Some(v) = idr(o) else { continue };
            let ours = Some(v) == lid || Some(v) == gid || chain.contains_key(&v);
            if !ours { continue; }
            let ok = (is_load && k == 0)
                || ((i.class.opcode == Op::AccessChain || i.class.opcode == Op::InBoundsAccessChain) && k == 0 && (Some(v) == lid || Some(v) == gid));
            if !ok { return Err("an id builtin is used other than by load / constant access chain"); }
        }
    }}}

    let mut next = m.header.as_ref().ok_or("no header")?.bound;
    let mut fresh = || { let n = next; next += 1; n };
    let mut globals: Vec<Instruction> = Vec::new();
    let mut annotations: Vec<Instruction> = Vec::new();
    let find = |op: Op, pred: &dyn Fn(&Instruction) -> bool| -> Option<u32> {
        m.types_global_values.iter().find(|i| i.class.opcode == op && pred(i)).and_then(|i| i.result_id)
    };
    let tbool = match find(Op::TypeBool, &|_| true) { Some(t) => t, None => { let t = fresh(); globals.push(Instruction::new(Op::TypeBool, None, Some(t), vec![])); t } };
    let tv3b = match find(Op::TypeVector, &|i| i.operands == vec![Operand::IdRef(tbool), Operand::LiteralBit32(3)]) {
        Some(t) => t,
        None => { let t = fresh(); globals.push(Instruction::new(Op::TypeVector, None, Some(t), vec![Operand::IdRef(tbool), Operand::LiteralBit32(3)])); t }
    };
    let one = match find(Op::Constant, &|i| i.result_type == Some(uint) && i.operands == vec![Operand::LiteralBit32(1)]) {
        Some(c) => c,
        None => { let c = fresh(); globals.push(Instruction::new(Op::Constant, Some(uint), Some(c), vec![Operand::LiteralBit32(1)])); c }
    };
    let flag = fresh();
    globals.push(Instruction::new(Op::SpecConstantFalse, Some(tbool), Some(flag), vec![]));
    annotations.push(Instruction::new(Op::Decorate, None, None,
        vec![Operand::IdRef(flag), Operand::Decoration(Decoration::SpecId), Operand::LiteralBit32(LINEARIZE_SPEC_ID)]));
    let sco = |g: &mut Vec<Instruction>, id: u32, op: Op, args: &[u32]| {
        let mut ops = vec![Operand::LiteralSpecConstantOpInteger(op)];
        ops.extend(args.iter().map(|a| Operand::IdRef(*a)));
        g.push(Instruction::new(Op::SpecConstantOp, Some(uint), Some(id), ops));
    };
    let (xy, xyz, nx, ny, nz, w2, flag3) = (fresh(), fresh(), fresh(), fresh(), fresh(), fresh(), fresh());
    sco(&mut globals, xy, Op::IMul, &[x, y]);
    sco(&mut globals, xyz, Op::IMul, &[xy, z]);
    sco(&mut globals, nx, Op::Select, &[flag, xyz, x]);
    sco(&mut globals, ny, Op::Select, &[flag, one, y]);
    sco(&mut globals, nz, Op::Select, &[flag, one, z]);
    globals.push(Instruction::new(Op::SpecConstantComposite, Some(v3u), Some(w2), vec![Operand::IdRef(nx), Operand::IdRef(ny), Operand::IdRef(nz)]));
    globals.push(Instruction::new(Op::SpecConstantComposite, Some(tv3b), Some(flag3), vec![Operand::IdRef(flag), Operand::IdRef(flag), Operand::IdRef(flag)]));
    m.annotations[wg_dec].operands[0] = Operand::IdRef(w2);

    let mut new_iface: Vec<u32> = Vec::new();
    let input_var = |g: &mut Vec<Instruction>, a: &mut Vec<Instruction>, fresh: &mut dyn FnMut() -> u32, ty: u32, b: BuiltIn,
                         existing: Option<u32>, iface: &mut Vec<u32>| -> u32 {
        if let Some(v) = existing { return v; }
        let ptr = m.types_global_values.iter().chain(g.iter()).find(|i| i.class.opcode == Op::TypePointer
            && i.operands == vec![Operand::StorageClass(StorageClass::Input), Operand::IdRef(ty)]).and_then(|i| i.result_id);
        let ptr = match ptr { Some(p) => p, None => { let p = fresh(); g.push(Instruction::new(Op::TypePointer, None, Some(p),
            vec![Operand::StorageClass(StorageClass::Input), Operand::IdRef(ty)])); p } };
        let v = fresh();
        g.push(Instruction::new(Op::Variable, Some(ptr), Some(v), vec![Operand::StorageClass(StorageClass::Input)]));
        a.push(Instruction::new(Op::Decorate, None, None, vec![Operand::IdRef(v), Operand::Decoration(Decoration::BuiltIn), Operand::BuiltIn(b)]));
        iface.push(v);
        v
    };
    let need = lid.is_some() || gid.is_some();
    let (lidx, wid) = if need {
        let lidx = input_var(&mut globals, &mut annotations, &mut fresh, uint, BuiltIn::LocalInvocationIndex,
                             builtin_var.get(&BuiltIn::LocalInvocationIndex).copied(), &mut new_iface);
        let wid = if gid.is_some() { Some(input_var(&mut globals, &mut annotations, &mut fresh, v3u, BuiltIn::WorkgroupId,
                             builtin_var.get(&BuiltIn::WorkgroupId).copied(), &mut new_iface)) } else { None };
        (Some(lidx), wid)
    } else { (None, None) };

    let comps_of = |f: &mut dyn FnMut() -> u32, out: &mut Vec<Instruction>, lidx: u32| -> [u32; 3] {
        let (i, lx, t, ly, lz) = (f(), f(), f(), f(), f());
        out.push(Instruction::new(Op::Load, Some(uint), Some(i), vec![Operand::IdRef(lidx)]));
        out.push(Instruction::new(Op::UMod, Some(uint), Some(lx), vec![Operand::IdRef(i), Operand::IdRef(x)]));
        out.push(Instruction::new(Op::UDiv, Some(uint), Some(t), vec![Operand::IdRef(i), Operand::IdRef(x)]));
        out.push(Instruction::new(Op::UMod, Some(uint), Some(ly), vec![Operand::IdRef(t), Operand::IdRef(y)]));
        out.push(Instruction::new(Op::UDiv, Some(uint), Some(lz), vec![Operand::IdRef(t), Operand::IdRef(y)]));
        [lx, ly, lz]
    };
    let global_of = |f: &mut dyn FnMut() -> u32, out: &mut Vec<Instruction>, l: [u32; 3], wid: u32| -> [u32; 3] {
        let wv = f();
        out.push(Instruction::new(Op::Load, Some(v3u), Some(wv), vec![Operand::IdRef(wid)]));
        let size = [x, y, z];
        let mut g = [0u32; 3];
        for d in 0..3 {
            let (e, p, s) = (f(), f(), f());
            out.push(Instruction::new(Op::CompositeExtract, Some(uint), Some(e), vec![Operand::IdRef(wv), Operand::LiteralBit32(d as u32)]));
            out.push(Instruction::new(Op::IMul, Some(uint), Some(p), vec![Operand::IdRef(e), Operand::IdRef(size[d])]));
            out.push(Instruction::new(Op::IAdd, Some(uint), Some(s), vec![Operand::IdRef(p), Operand::IdRef(l[d])]));
            g[d] = s;
        }
        g
    };
    let mut rewrote = 0u32;
    for f in m.functions.iter_mut() {
        for b in f.blocks.iter_mut() {
            let old = std::mem::take(&mut b.instructions);
            for i in old {
                let ptr = if i.class.opcode == Op::Load { i.operands.first().and_then(idr) } else { None };
                let target = ptr.and_then(|p| if Some(p) == lid || Some(p) == gid { Some((p, None)) } else { chain.get(&p).map(|(v, c)| (*v, Some(*c))) });
                let Some((var, comp)) = target else { b.instructions.push(i); continue };
                let (Some(lidx), Some(res), Some(rty)) = (lidx, i.result_id, i.result_type) else { b.instructions.push(i); continue };
                let mut out = Vec::new();
                let l = comps_of(&mut fresh, &mut out, lidx);
                let v = if Some(var) == gid { global_of(&mut fresh, &mut out, l, wid.unwrap()) } else { l };
                let orig = fresh();
                let mut load = i.clone(); load.result_id = Some(orig);
                out.push(load);
                match comp {
                    None => {
                        let lin = fresh();
                        out.push(Instruction::new(Op::CompositeConstruct, Some(v3u), Some(lin), v.iter().map(|c| Operand::IdRef(*c)).collect()));
                        out.push(Instruction::new(Op::Select, Some(rty), Some(res), vec![Operand::IdRef(flag3), Operand::IdRef(lin), Operand::IdRef(orig)]));
                    }
                    Some(c) => {
                        out.push(Instruction::new(Op::Select, Some(rty), Some(res), vec![Operand::IdRef(flag), Operand::IdRef(v[c as usize]), Operand::IdRef(orig)]));
                    }
                }
                b.instructions.extend(out);
                rewrote += 1;
            }
        }
    }
    let _ = rewrote;
    m.types_global_values.extend(globals);
    m.annotations.extend(annotations);
    for e in m.entry_points.iter_mut() {
        if e.operands.first() == Some(&Operand::ExecutionModel(ExecutionModel::GLCompute)) {
            e.operands.extend(new_iface.iter().map(|v| Operand::IdRef(*v)));
        }
    }
    m.header.as_mut().unwrap().bound = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spirv_tools::{assembler::Assembler, val::Validator};
    const FIXTURE: &str = r#"
OpCapability Shader
OpMemoryModel Logical GLSL450
OpEntryPoint GLCompute %main "main" %lid %gid
OpExecutionMode %main LocalSize 1 1 1
OpDecorate %sx SpecId 0
OpDecorate %sy SpecId 1
OpDecorate %sz SpecId 2
OpDecorate %wg BuiltIn WorkgroupSize
OpDecorate %lid BuiltIn LocalInvocationId
OpDecorate %gid BuiltIn GlobalInvocationId
%void = OpTypeVoid
%fn = OpTypeFunction %void
%uint = OpTypeInt 32 0
%v3 = OpTypeVector %uint 3
%pv3 = OpTypePointer Input %v3
%pu = OpTypePointer Input %uint
%c2 = OpConstant %uint 2
%sx = OpSpecConstant %uint 64
%sy = OpSpecConstant %uint 1
%sz = OpSpecConstant %uint 1
%wg = OpSpecConstantComposite %v3 %sx %sy %sz
%lid = OpVariable %pv3 Input
%gid = OpVariable %pv3 Input
%main = OpFunction %void None %fn
%l = OpLabel
%a = OpLoad %v3 %lid
%p = OpAccessChain %pu %gid %c2
%b = OpLoad %uint %p
OpReturn
OpFunctionEnd
"#;
    fn bytes(src: &str) -> Vec<u8> {
        spirv_tools::assembler::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).assemble(src, Default::default()).unwrap().as_bytes().to_vec()
    }
    fn validate(b: &[u8]) {
        let w: Vec<u32> = b.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        spirv_tools::val::create(Some(spirv_tools::TargetEnv::Vulkan_1_2)).validate(&w, None).expect("module must validate");
    }
    #[test]
    fn a_dynamic_kernel_gains_spec_id_3_and_stays_valid() {
        let (out, why) = apply(&bytes(FIXTURE)).unwrap();
        assert_eq!(why, None, "the translator's dynamic shape must be rewritten");
        let m = rspirv::dr::load_bytes(&out).unwrap();
        let spec3 = m.annotations.iter().filter(|a| a.operands.get(1) == Some(&Operand::Decoration(Decoration::SpecId))
            && a.operands.get(2) == Some(&Operand::LiteralBit32(LINEARIZE_SPEC_ID))).count();
        assert_eq!(spec3, 1, "exactly one SpecId 3");
        let selects = m.functions.iter().flat_map(|f| f.blocks.iter()).flat_map(|b| b.instructions.iter())
            .filter(|i| i.class.opcode == Op::Select).count();
        assert_eq!(selects, 2, "both id reads (vector and chained component) go through the flag");
        validate(&bytes(FIXTURE));
        validate(&out);
    }
    #[test]
    fn a_fixed_size_kernel_is_left_byte_identical() {
        let src = FIXTURE.replace("%wg = OpSpecConstantComposite %v3 %sx %sy %sz", "%wg = OpConstantComposite %v3 %c2 %c2 %c2");
        let inp = bytes(&src);
        let (out, why) = apply(&inp).unwrap();
        assert_eq!(out, inp, "not ours -> not touched");
        assert!(why.is_some(), "and the reason rides back");
    }
    #[test]
    fn a_pointer_passed_on_refuses_the_whole_module() {
        let src = FIXTURE.replace("%b = OpLoad %uint %p\n", "%b = OpLoad %uint %p\n%q = OpCopyObject %pv3 %lid\n");
        let inp = bytes(&src);
        let (out, why) = apply(&inp).unwrap();
        assert_eq!(out, inp, "a use the pass cannot see through leaves the module alone");
        assert!(why.is_some());
    }
}
