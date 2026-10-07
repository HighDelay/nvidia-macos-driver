use std::collections::BTreeMap;

pub const MESH_OUT_BUFFER: u32 = 30;
pub const VERTEX_ENTRY: &str = "m2v_mesh_vs";
pub const PAYLOAD_BUFFER: u32 = 29;
pub const HEADER_BYTES: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Link {
    Direct,
    Object,
    Indirect,
}

pub fn record_stride(payload_len: u32) -> u32 {
    16 + ((payload_len + 15) & !15)
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshLayout {
    pub nv: u32,
    pub np: u32,
    pub k: u32,
    pub vs: u32,
    pub ps: u32,
    pub idx_off: u32,
    pub block: u32,
}

#[derive(Debug, Clone)]
pub struct LoweredMesh {
    pub kernel_ll: String,
    pub vertex_ll: String,
    pub layout: MeshLayout,
    pub tr: u32,
}

#[derive(Debug, Clone)]
struct Attr {
    generated: String,
    type_name: String,
    arg_name: String,
    slot: u32,
}

fn node_lines(ll: &str) -> BTreeMap<u32, String> {
    let mut m = BTreeMap::new();
    for l in ll.lines() {
        let t = l.trim_start();
        if let Some(rest) = t.strip_prefix('!') {
            if let Some((n, body)) = rest.split_once(" = ") {
                if let Ok(n) = n.trim().parse::<u32>() {
                    m.insert(n, body.trim().to_string());
                }
            }
        }
    }
    m
}

fn refs(body: &str) -> Vec<u32> {
    let mut v = Vec::new();
    let b = body.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'!' && i + 1 < b.len() && b[i + 1].is_ascii_digit() {
            let s = i + 1;
            let mut e = s;
            while e < b.len() && b[e].is_ascii_digit() {
                e += 1;
            }
            v.push(body[s..e].parse().unwrap());
            i = e;
        } else {
            i += 1;
        }
    }
    v
}

fn entry_arg_list(tail: &str) -> Option<u32> {
    refs(tail).get(1).copied()
}

fn entry_arg_list_span(line: &str, entry: &str, arg_list: u32) -> Option<(usize, usize)> {
    let from = line.find(&format!("@{entry},"))?;
    let b = line.as_bytes();
    let (mut i, mut k) = (from, 0);
    while i < b.len() {
        if b[i] == b'!' && i + 1 < b.len() && b[i + 1].is_ascii_digit() {
            let mut e = i + 1;
            while e < b.len() && b[e].is_ascii_digit() {
                e += 1;
            }
            if k == 1 {
                return (line[i + 1..e].parse::<u32>().ok()? == arg_list).then_some((i, e));
            }
            k += 1;
            i = e;
        } else {
            i += 1;
        }
    }
    None
}

fn quoted_after<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let at = body.find(&format!("!\"{key}\", !\""))? + key.len() + 7;
    let rest = &body[at..];
    Some(&rest[..rest.find('"')?])
}

fn first_int(body: &str) -> Option<u32> {
    body.split("i32 ")
        .nth(1)?
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

fn llvm_type(name: &str) -> Result<(String, u32), String> {
    let (base, n) = match name.find(|c: char| c.is_ascii_digit()) {
        Some(i) => (
            &name[..i],
            name[i..]
                .parse::<u32>()
                .map_err(|_| format!("mesh attribute type {name}"))?,
        ),
        None => (name, 1),
    };
    let (ty, sz) = match base {
        "float" => ("float", 4),
        "half" => ("half", 2),
        "int" | "uint" => ("i32", 4),
        "short" | "ushort" => ("i16", 2),
        "char" | "uchar" => ("i8", 1),
        _ => return Err(format!("mesh attribute type {name} is not lowered")),
    };
    if !(1..=4).contains(&n) || n * sz > 16 {
        return Err(format!("mesh attribute type {name} is not lowered"));
    }
    Ok((
        if n == 1 {
            ty.to_string()
        } else {
            format!("<{n} x {ty}>")
        },
        n * sz,
    ))
}

fn split_args(s: &str) -> Vec<String> {
    let (mut v, mut cur, mut d) = (Vec::new(), String::new(), 0i32);
    for c in s.chars() {
        match c {
            '(' | '[' | '<' | '{' => d += 1,
            ')' | ']' | '>' | '}' => d -= 1,
            _ => {}
        }
        if c == ',' && d == 0 {
            v.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() {
        v.push(cur.trim().to_string());
    }
    v
}

fn ty_val(arg: &str) -> (String, String) {
    let arg = arg.trim();
    let splat = arg.find(" splat (").filter(|_| arg.ends_with(')'));
    let val = if let Some(sp) = splat {
        arg[sp + 1..].to_string()
    } else if arg.ends_with('>') {
        let mut d = 0i32;
        let mut start = 0;
        for (i, ch) in arg.char_indices().rev() {
            match ch {
                '>' => d += 1,
                '<' => {
                    d -= 1;
                    if d == 0 {
                        start = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        arg[start..].to_string()
    } else {
        arg.rsplit(' ').next().unwrap_or("").to_string()
    };
    let mut ty = arg[..arg.len() - val.len()].trim().to_string();
    for a in ["noundef", "nocapture", "readonly", "writeonly", "nonnull"] {
        while ty.ends_with(a) {
            ty = ty[..ty.len() - a.len()].trim().to_string();
        }
    }
    if splat.is_some() {
        let lanes = ty
            .strip_prefix('<')
            .and_then(|t| t.split(" x ").next())
            .and_then(|n| n.trim().parse::<usize>().ok());
        if let Some(n) = lanes {
            let inner = val["splat (".len()..val.len() - 1].trim().to_string();
            return (ty, format!("<{}>", vec![inner; n].join(", ")));
        }
    }
    (ty, val)
}

fn bare_ptr_ty(ty: &str) -> String {
    match ty.find('*') {
        Some(i) => ty[..=i].to_string(),
        None => ty.split(' ').take(2).collect::<Vec<_>>().join(" "),
    }
}

fn paren_close(s: &str, open: usize) -> Option<usize> {
    let mut d = 0i32;
    for (i, c) in s[open..].char_indices() {
        match c {
            '(' => d += 1,
            ')' => {
                d -= 1;
                if d == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

fn emit_load(opaque: bool, name: &str, base: &str, off: &str, ty: &str) -> String {
    if opaque {
        format!("  {name}.a = getelementptr inbounds i8, ptr addrspace(1) {base}, i32 {off}\n  {name} = load {ty}, ptr addrspace(1) {name}.a, align 4\n")
    } else {
        format!(
            "  {name}.a = getelementptr inbounds i8, i8 addrspace(1)* {base}, i32 {off}\n  {name}.p = bitcast i8 addrspace(1)* {name}.a to {ty} addrspace(1)*\n  \
             {name} = load {ty}, {ty} addrspace(1)* {name}.p, align 4\n"
        )
    }
}

fn emit_atomic_max(opaque: bool, ind: &str, name: &str, base: &str, off: u32, val: &str) -> String {
    if opaque {
        format!(
            "{ind}{name}.a = getelementptr inbounds i8, ptr addrspace(1) {base}, i32 {off}\n\
             {ind}{name} = call i32 @air.atomic.global.max.u.i32(ptr addrspace(1) {name}.a, i32 {val}, i32 0, i32 2, i1 true)\n"
        )
    } else {
        format!(
            "{ind}{name}.a = getelementptr inbounds i8, i8 addrspace(1)* {base}, i32 {off}\n\
             {ind}{name}.p = bitcast i8 addrspace(1)* {name}.a to i32 addrspace(1)*\n\
             {ind}{name} = call i32 @air.atomic.global.max.u.i32(i32 addrspace(1)* {name}.p, i32 {val}, i32 0, i32 2, i1 true)\n"
        )
    }
}

fn atomic_max_decl(opaque: bool) -> &'static str {
    if opaque {
        "declare i32 @air.atomic.global.max.u.i32(ptr addrspace(1) nocapture, i32, i32, i32, i1)\n"
    } else {
        "declare i32 @air.atomic.global.max.u.i32(i32 addrspace(1)* nocapture, i32, i32, i32, i1)\n"
    }
}

fn respell_object_barrier(line: &str) -> Result<String, String> {
    for callee in ["@air.wg.barrier(i32 ", "@air.simdgroup.barrier(i32 "] {
        if let Some(at) = line.find(callee) {
            let s = at + callee.len();
            let e = s + line[s..]
                .find(',')
                .ok_or("mesh lowering: malformed barrier")?;
            let f: u32 = line[s..e].trim().parse().map_err(|_| {
                format!(
                    "mesh lowering: barrier flags are not a constant: {}",
                    line.trim()
                )
            })?;
            if f & 16 != 0 {
                return Ok(format!("{}{}{}", &line[..s], (f & !16) | 1, &line[e..]));
            }
        }
    }
    Ok(line.to_string())
}

fn implicit_entry_label(params: &[String]) -> usize {
    params
        .iter()
        .filter(|p| {
            let (_, v) = ty_val(p);
            v.len() > 1 && v.starts_with('%') && v[1..].bytes().all(|b| b.is_ascii_digit())
        })
        .count()
}

pub fn payload_len(ll: &str, entry: &str) -> Result<u32, String> {
    let nodes = node_lines(ll);
    let at = format!("@{entry},");
    let fnode = ll
        .lines()
        .filter(|l| l.starts_with("!air.mesh = ") || l.starts_with("!air.object = "))
        .flat_map(refs)
        .find_map(|n| nodes.get(&n).filter(|b| b.contains(&at)))
        .ok_or_else(|| {
            format!("mesh lowering: \"{entry}\" is in neither !air.mesh nor !air.object")
        })?;
    let arg_list = entry_arg_list(&fnode[fnode.find(&at).unwrap()..])
        .ok_or("mesh lowering: entry node has no argument list")?;
    for a in refs(&nodes[&arg_list]) {
        let b = &nodes[&a];
        if b.contains("!\"air.payload\"") {
            return b
                .split("!\"air.arg_type_size\", i32 ")
                .nth(1)
                .and_then(|r| r.split(|c: char| !c.is_ascii_digit()).next())
                .and_then(|n| n.parse().ok())
                .ok_or_else(|| format!("mesh lowering: the payload of @{entry} states no size"));
        }
    }
    Ok(0)
}

pub fn lower_mesh(ll: &str, entry: &str) -> Result<LoweredMesh, String> {
    lower_mesh_linked(ll, entry, Link::Direct, 0, 0)
}

pub fn lower_mesh_linked(
    ll: &str,
    entry: &str,
    link: Link,
    rs: u32,
    cap: u32,
) -> Result<LoweredMesh, String> {
    let linked = link != Link::Direct;
    if linked && (rs < 16 || !rs.is_multiple_of(16) || cap == 0) {
        return Err(format!(
            "mesh lowering: a linked mesh needs a record stride and a cap (rs {rs}, cap {cap})"
        ));
    }
    let opaque = ll.contains("ptr addrspace(7)");
    let mesh_ty = if opaque {
        "ptr addrspace(7)"
    } else {
        "%struct._mesh_t addrspace(7)*"
    };
    let out_ty = if opaque {
        "ptr addrspace(1)"
    } else {
        "i8 addrspace(1)*"
    };
    let nodes = node_lines(ll);

    let mesh_list = ll
        .lines()
        .find(|l| l.starts_with("!air.mesh = "))
        .ok_or_else(|| {
            format!("mesh lowering: no !air.mesh list (is \"{entry}\" a [[mesh]] function?)")
        })?;
    let at = format!("@{entry},");
    let fnode = refs(mesh_list)
        .into_iter()
        .find(|n| nodes.get(n).is_some_and(|b| b.contains(&at)))
        .ok_or_else(|| format!("mesh lowering: \"{entry}\" is not in !air.mesh"))?;
    let fbody = nodes[&fnode].clone();
    let arg_list = entry_arg_list(&fbody[fbody.find(&at).unwrap()..])
        .ok_or("mesh lowering: entry node has no argument list")?;
    let args: Vec<u32> = refs(&nodes[&arg_list]);
    let mut mesh_arg = None;
    let mut held: [Option<(u32, usize, String)>; 2] = [None, None];
    let mut payload: Option<(u32, usize)> = None;
    for a in &args {
        let b = &nodes[a];
        if b.contains("!\"air.payload\"") {
            match link {
                Link::Object => payload = Some((*a, first_int(b).ok_or("mesh lowering: payload argument has no index")? as usize)),
                Link::Direct => return Err("mesh lowering: an object-stage payload ([[payload]]) needs the object stage it belongs to (lower_mesh_linked with Link::Object)".into()),
                Link::Indirect => return Err("mesh lowering: a mesh function with a [[payload]] cannot be drawn without its object stage".into()),
            }
        }
        if linked
            && (b.contains("!\"air.thread_position_in_grid\"")
                || b.contains("!\"air.threads_per_grid\""))
        {
            return Err(format!("mesh lowering: [[thread_position_in_grid]] / [[threads_per_grid]] in a mesh stage behind an object stage or an indirect draw is not lowered ({entry})"));
        }
        if b.contains("!\"air.buffer\"") && b.contains("!\"air.location_index\"") {
            let loc = b
                .split("!\"air.location_index\", i32 ")
                .nth(1)
                .and_then(|r| r.split(',').next())
                .and_then(|n| n.trim().parse::<u32>().ok());
            if loc == Some(MESH_OUT_BUFFER) || (linked && loc == Some(PAYLOAD_BUFFER)) {
                return Err(format!(
                    "mesh lowering: the mesh function binds buffer {}, which the lowering owns",
                    loc.unwrap()
                ));
            }
        }
        for (h, key) in [
            "air.threadgroup_position_in_grid",
            "air.threadgroups_per_grid",
        ]
        .iter()
        .enumerate()
        {
            if b.contains(&format!("!\"{key}\"")) {
                let tn = quoted_after(b, "air.arg_type_name")
                    .ok_or("mesh lowering: grid builtin without a type")?
                    .to_string();
                if !["uint", "uint2", "uint3"].contains(&tn.as_str()) {
                    return Err(format!(
                        "mesh lowering: grid builtin {key} of type {tn} is not lowered"
                    ));
                }
                held[h] = Some((
                    *a,
                    first_int(b).ok_or("mesh lowering: grid builtin has no index")? as usize,
                    tn,
                ));
            }
        }
        if b.contains("!\"air.mesh\"") {
            mesh_arg = Some((
                *a,
                first_int(b).ok_or("mesh lowering: mesh argument has no index")?,
                refs(b),
            ));
        }
    }
    let (mesh_node, mesh_index, mesh_refs) =
        mesh_arg.ok_or("mesh lowering: the entry has no mesh<> argument")?;
    let tinfo = *mesh_refs
        .first()
        .ok_or("mesh lowering: mesh argument has no type info")?;
    let tbody = nodes[&tinfo].clone();
    if !tbody.contains("!\"air.mesh_type_info\"") {
        return Err(format!("mesh lowering: unexpected mesh type info {tbody}"));
    }
    let tr = refs(&tbody);
    let (vlist, plist) = (tr[0], tr[1]);
    let ints: Vec<u32> = tbody
        .split("i32 ")
        .skip(1)
        .filter_map(|r| r.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok())
        .collect();
    let (nv, np) = (
        *ints.first().ok_or("mesh lowering: no vertex count")?,
        *ints.get(1).ok_or("mesh lowering: no primitive count")?,
    );
    let k = if tbody.contains("!\"air.triangle\"") {
        3
    } else if tbody.contains("!\"air.line\"") {
        2
    } else if tbody.contains("!\"air.point\"") {
        1
    } else {
        return Err(format!("mesh lowering: unknown topology in {tbody}"));
    };
    let mut vattrs = Vec::new();
    let mut have_pos = false;
    for r in refs(&nodes[&vlist]) {
        let b = &nodes[&r];
        if b.starts_with("!{!\"air.position\"") {
            have_pos = true;
        } else if b.starts_with("!{!\"air.mesh_vertex_data\"") {
            vattrs.push(Attr {
                slot: first_int(b).ok_or("mesh lowering: vertex attribute without slot")?,
                generated: b
                    .split("!\"generated(")
                    .nth(1)
                    .map(|r| format!("generated({}", &r[..r.find('"').unwrap_or(0)]))
                    .ok_or("mesh lowering: vertex attribute without generated name")?,
                type_name: quoted_after(b, "air.arg_type_name")
                    .ok_or("mesh lowering: vertex attribute without type")?
                    .to_string(),
                arg_name: quoted_after(b, "air.arg_name")
                    .unwrap_or("attr")
                    .to_string(),
            });
        } else {
            return Err(format!("mesh lowering: vertex output {b} is not lowered (point size / clip distance / invariant)"));
        }
    }
    if !have_pos {
        return Err("mesh lowering: the mesh vertex has no [[position]]".into());
    }
    let mut pattrs = Vec::new();
    for r in refs(&nodes[&plist]) {
        let b = &nodes[&r];
        if b.starts_with("!{!\"air.mesh_primitive_data\"") {
            pattrs.push(Attr {
                slot: first_int(b).ok_or("mesh lowering: primitive attribute without slot")?,
                generated: b
                    .split("!\"generated(")
                    .nth(1)
                    .map(|r| format!("generated({}", &r[..r.find('"').unwrap_or(0)]))
                    .ok_or("mesh lowering: primitive attribute without generated name")?,
                type_name: quoted_after(b, "air.arg_type_name")
                    .ok_or("mesh lowering: primitive attribute without type")?
                    .to_string(),
                arg_name: quoted_after(b, "air.arg_name")
                    .unwrap_or("prim")
                    .to_string(),
            });
        } else {
            return Err(format!("mesh lowering: primitive output {b} is not lowered (primitive id / layer / viewport / culled)"));
        }
    }
    for a in vattrs.iter().chain(pattrs.iter()) {
        llvm_type(&a.type_name)?;
    }
    let vs = 16 * (1 + vattrs.len() as u32);
    let ps = 16 * pattrs.len() as u32;
    let idx_off = 16 + nv * vs + np * ps;
    let block = (idx_off + np * k * 4 + 15) & !15;
    let layout = MeshLayout {
        nv,
        np,
        k,
        vs,
        ps,
        idx_off,
        block,
    };
    let vpos = |slot: u32| {
        vattrs
            .iter()
            .position(|a| a.slot == slot)
            .map(|p| 16 * (1 + p as u32))
    };
    let ppos = |slot: u32| {
        pattrs
            .iter()
            .position(|a| a.slot == slot)
            .map(|p| 16 * p as u32)
    };

    let tr = if linked { HEADER_BYTES + block } else { 0 };
    let pk = np * k;

    let retyped = if link == Link::Object {
        ll.replace(mesh_ty, out_ty)
            .replace("addrspace(6)", "addrspace(1)")
    } else {
        ll.replace(mesh_ty, out_ty)
    };
    let mut out = String::with_capacity(ll.len() + 4096);
    let mut in_entry = false;
    let mut prologue: Option<String> = None;
    let mut mesh_value = String::new();
    let mut nparams = 0usize;
    let mut entry_num = 0usize;
    let mut c = 0usize;
    let store = |out: &mut String, ind: &str, c: usize, h: &str, off: &str, ty: &str, val: &str| {
        if opaque {
            out.push_str(&format!(
                "{ind}%m2v.ma{c} = getelementptr inbounds i8, ptr addrspace(1) {h}, i32 {off}\n"
            ));
            out.push_str(&format!(
                "{ind}store {ty} {val}, ptr addrspace(1) %m2v.ma{c}, align 4\n"
            ));
        } else {
            out.push_str(&format!(
                "{ind}%m2v.ma{c} = getelementptr inbounds i8, i8 addrspace(1)* {h}, i32 {off}\n"
            ));
            out.push_str(&format!(
                "{ind}%m2v.mp{c} = bitcast i8 addrspace(1)* %m2v.ma{c} to {ty} addrspace(1)*\n"
            ));
            out.push_str(&format!(
                "{ind}store {ty} {val}, {ty} addrspace(1)* %m2v.mp{c}, align 4\n"
            ));
        }
    };
    for line in retyped.lines() {
        let t = line.trim_start();
        let ind = &line[..line.len() - t.len()];
        if let Some(p) = prologue.take() {
            let label =
                !t.starts_with(';') && t.split(';').next().unwrap_or("").trim_end().ends_with(':');
            if label {
                out.push_str(line);
                out.push('\n');
                out.push_str(&p);
                continue;
            }
            out.push_str(&format!("{entry_num}:\n"));
            out.push_str(&p);
        }
        if t.starts_with("declare ")
            && t.contains("@air.set_")
            && (t.contains("_mesh(") || t.contains("_mesh."))
        {
            continue;
        }
        if t.starts_with("define ") && t.contains(&format!("@{entry}(")) {
            let open = t.find(&format!("@{entry}(")).unwrap() + entry.len() + 1;
            let close =
                paren_close(t, open).ok_or("mesh lowering: entry parameter list is not closed")?;
            let params = split_args(&t[open + 1..close]);
            nparams = params.len();
            entry_num = implicit_entry_label(&params);
            let (_, v) = ty_val(
                params
                    .get(mesh_index as usize)
                    .ok_or("mesh lowering: mesh argument index past the parameters")?,
            );
            mesh_value = v;
            let mut ps = params.clone();
            let mut narrow = String::new();
            let (utgp, untg) = match link {
                Link::Direct => ("%m2v.tgp", "%m2v.ntg"),
                Link::Object => ("%m2v.ltgp", "%m2v.g"),
                Link::Indirect => ("%m2v.tgp", "%m2v.g"),
            };
            for (h, (name, src)) in [("%m2v.tgp", utgp), ("%m2v.ntg", untg)].iter().enumerate() {
                match &held[h] {
                    Some((_, i, tn)) => {
                        let (_, orig) = ty_val(
                            ps.get(*i)
                                .ok_or("mesh lowering: grid builtin index past the parameters")?,
                        );
                        ps[*i] = format!("<3 x i32> noundef {name}");
                        narrow.push_str(&match tn.as_str() {
                            "uint" => format!("  {orig} = extractelement <3 x i32> {src}, i32 0\n"),
                            "uint2" => format!("  {orig} = shufflevector <3 x i32> {src}, <3 x i32> undef, <2 x i32> <i32 0, i32 1>\n"),
                            _ => format!("  {orig} = shufflevector <3 x i32> {src}, <3 x i32> undef, <3 x i32> <i32 0, i32 1, i32 2>\n"),
                        });
                    }
                    None => ps.push(format!("<3 x i32> noundef {name}")),
                }
            }
            let mut pdef = String::new();
            if let Some((_, pi)) = payload {
                let p = ps
                    .get(pi)
                    .ok_or("mesh lowering: payload index past the parameters")?
                    .clone();
                let (pty, orig) = ty_val(&p);
                let pty = bare_ptr_ty(&pty);
                ps[pi] = format!("{}%m2v.plraw", &p[..p.len() - orig.len()]);
                pdef = if opaque {
                    format!("  {orig} = getelementptr inbounds i8, ptr addrspace(1) %m2v.plraw, i32 %m2v.plo\n")
                } else {
                    format!(
                        "  %m2v.plb = bitcast {pty} %m2v.plraw to i8 addrspace(1)*\n  %m2v.pla = getelementptr inbounds i8, i8 addrspace(1)* %m2v.plb, i32 %m2v.plo\n  \
                         {orig} = bitcast i8 addrspace(1)* %m2v.pla to {pty}\n"
                    )
                };
            }
            let def = format!("{}{}{}", &t[..open + 1], ps.join(", "), &t[close..]);
            out.push_str(&format!("{ind}{def}\n"));
            let gep = if opaque {
                format!("  %m2v.mesh = getelementptr inbounds i8, ptr addrspace(1) {mesh_value}, i32 %m2v.boff\n")
            } else {
                format!("  %m2v.mesh = getelementptr inbounds i8, i8 addrspace(1)* {mesh_value}, i32 %m2v.boff\n")
            };
            let lin = "  %m2v.t0 = extractelement <3 x i32> %m2v.tgp, i32 0\n  %m2v.t1 = extractelement <3 x i32> %m2v.tgp, i32 1\n  \
                       %m2v.t2 = extractelement <3 x i32> %m2v.tgp, i32 2\n  %m2v.n0 = extractelement <3 x i32> GRID, i32 0\n  \
                       %m2v.n1 = extractelement <3 x i32> GRID, i32 1\n  %m2v.l0 = mul i32 %m2v.t2, %m2v.n1\n  \
                       %m2v.l1 = add i32 %m2v.l0, %m2v.t1\n  %m2v.l2 = mul i32 %m2v.l1, %m2v.n0\n  %m2v.lin = add i32 %m2v.l2, %m2v.t0\n";
            let mv = mesh_value.as_str();
            prologue = Some(match link {
                Link::Direct => format!("{narrow}{}  %m2v.boff = mul i32 %m2v.lin, {block}\n{gep}", lin.replace("GRID", "%m2v.ntg")),
                Link::Object => format!(
                    "  %m2v.j = extractelement <3 x i32> %m2v.tgp, i32 0\n  %m2v.o = extractelement <3 x i32> %m2v.tgp, i32 1\n\
                     {}{}  %m2v.big = icmp ugt i32 %m2v.mx0, {cap}\n  %m2v.mx = select i1 %m2v.big, i32 {cap}, i32 %m2v.mx0\n  \
                     %m2v.r0 = mul i32 %m2v.o, {rs}\n  %m2v.rec = add i32 %m2v.r0, {tr}\n  %m2v.plo = add i32 %m2v.rec, 16\n\
                     {}  %m2v.gx = extractelement <3 x i32> %m2v.g, i32 0\n  %m2v.gy = extractelement <3 x i32> %m2v.g, i32 1\n  \
                     %m2v.gz = extractelement <3 x i32> %m2v.g, i32 2\n  %m2v.pr0 = mul i32 %m2v.gx, %m2v.gy\n  %m2v.prod = mul i32 %m2v.pr0, %m2v.gz\n  \
                     %m2v.dd0 = icmp uge i32 %m2v.j, %m2v.prod\n  %m2v.dd1 = icmp uge i32 %m2v.j, %m2v.mx\n  %m2v.dead = or i1 %m2v.dd0, %m2v.dd1\n  \
                     %m2v.gx0 = icmp eq i32 %m2v.gx, 0\n  %m2v.gx1 = select i1 %m2v.gx0, i32 1, i32 %m2v.gx\n  \
                     %m2v.gy0 = icmp eq i32 %m2v.gy, 0\n  %m2v.gy1 = select i1 %m2v.gy0, i32 1, i32 %m2v.gy\n  \
                     %m2v.jj = select i1 %m2v.dead, i32 0, i32 %m2v.j\n  %m2v.lx = urem i32 %m2v.jj, %m2v.gx1\n  %m2v.q = udiv i32 %m2v.jj, %m2v.gx1\n  \
                     %m2v.ly = urem i32 %m2v.q, %m2v.gy1\n  %m2v.lz = udiv i32 %m2v.q, %m2v.gy1\n  \
                     %m2v.lt0 = insertelement <3 x i32> undef, i32 %m2v.lx, i32 0\n  %m2v.lt1 = insertelement <3 x i32> %m2v.lt0, i32 %m2v.ly, i32 1\n  \
                     %m2v.ltgp = insertelement <3 x i32> %m2v.lt1, i32 %m2v.lz, i32 2\n  \
                     %m2v.m0 = mul i32 %m2v.no, {rs}\n  %m2v.moff = add i32 %m2v.m0, {tr}\n  %m2v.tt0 = mul i32 %m2v.o, %m2v.mx\n  \
                     %m2v.tt = add i32 %m2v.tt0, %m2v.j\n  %m2v.bo0 = mul i32 %m2v.tt, {block}\n  %m2v.bo = add i32 %m2v.bo0, %m2v.moff\n  \
                     %m2v.boff = select i1 %m2v.dead, i32 {HEADER_BYTES}, i32 %m2v.bo\n{gep}{pdef}{narrow}",
                    emit_load(opaque, "%m2v.mx0", mv, "0", "i32"),
                    emit_load(opaque, "%m2v.no", mv, "4", "i32"),
                    emit_load(opaque, "%m2v.g", mv, "%m2v.rec", "<3 x i32>"),
                ),
                Link::Indirect => format!(
                    "{}{}  %m2v.dead = icmp uge i32 %m2v.lin, {cap}\n  %m2v.bo0 = mul i32 %m2v.lin, {block}\n  %m2v.bo = add i32 %m2v.bo0, {}\n  \
                     %m2v.boff = select i1 %m2v.dead, i32 {HEADER_BYTES}, i32 %m2v.bo\n  %m2v.lp0 = add i32 %m2v.lin, 1\n  \
                     %m2v.lp = select i1 %m2v.dead, i32 0, i32 %m2v.lp0\n  %m2v.vc = mul i32 %m2v.lp, {pk}\n{}{}{gep}{narrow}",
                    emit_load(opaque, "%m2v.g", mv, &tr.to_string(), "<3 x i32>"),
                    lin.replace("GRID", "%m2v.g"),
                    tr + rs,
                    emit_atomic_max(opaque, "  ", "%m2v.ax", mv, 0, "%m2v.lp"),
                    emit_atomic_max(opaque, "  ", "%m2v.av", mv, 16, "%m2v.vc"),
                ),
            });
            in_entry = true;
            continue;
        }
        let line_owned;
        let line = if linked
            && (t.contains("@air.wg.barrier(") || t.contains("@air.simdgroup.barrier("))
        {
            line_owned = respell_object_barrier(line)?;
            line_owned.as_str()
        } else {
            line
        };
        let t = line.trim_start();
        let mut l = line.to_string();
        if in_entry {
            if t == "}" {
                in_entry = false;
            } else {
                for form in [
                    format!("{out_ty} {mesh_value}"),
                    format!("{out_ty} nocapture {mesh_value}"),
                ] {
                    for end in [",", ")"] {
                        l = l.replace(&format!("{form}{end}"), &format!("{out_ty} %m2v.mesh{end}"));
                    }
                }
            }
        }
        let lt = l.trim_start();
        if let Some(open) = lt.find("@air.set_").filter(|_| {
            lt.contains("call ")
                && lt.contains("_mesh")
                && !lt.contains("@air.set_threadgroups_per_grid_mesh_properties(")
        }) {
            let name_end = lt[open..]
                .find('(')
                .ok_or("mesh lowering: malformed mesh call")?
                + open;
            let name = &lt[open + 1..name_end];
            let close =
                paren_close(lt, name_end).ok_or("mesh lowering: mesh call is not closed")?;
            let a: Vec<(String, String)> = split_args(&lt[name_end + 1..close])
                .iter()
                .map(|s| ty_val(s))
                .collect();
            let h = a[0].1.clone();
            let lit = |v: &str| {
                v.parse::<u32>()
                    .map_err(|_| format!("mesh lowering: {name} slot {v} is not a constant"))
            };
            c += 1;
            if name == "air.set_position_mesh" {
                out.push_str(&format!("{ind}%m2v.mo{c} = mul i32 {}, {vs}\n{ind}%m2v.mq{c} = add i32 %m2v.mo{c}, 16\n", a[1].1));
                store(
                    &mut out,
                    ind,
                    c,
                    &h,
                    &format!("%m2v.mq{c}"),
                    &a[2].0,
                    &a[2].1,
                );
            } else if name.starts_with("air.set_vertex_data_mesh") {
                let off = vpos(lit(&a[1].1)?).ok_or_else(|| {
                    format!(
                        "mesh lowering: vertex attribute slot {} is not declared",
                        a[1].1
                    )
                })?;
                out.push_str(&format!("{ind}%m2v.mo{c} = mul i32 {}, {vs}\n{ind}%m2v.mq{c} = add i32 %m2v.mo{c}, {}\n", a[2].1, 16 + off));
                store(
                    &mut out,
                    ind,
                    c,
                    &h,
                    &format!("%m2v.mq{c}"),
                    &a[3].0,
                    &a[3].1,
                );
            } else if name.starts_with("air.set_primitive_data_mesh") {
                let off = ppos(lit(&a[1].1)?).ok_or_else(|| {
                    format!(
                        "mesh lowering: primitive attribute slot {} is not declared",
                        a[1].1
                    )
                })?;
                out.push_str(&format!("{ind}%m2v.mo{c} = mul i32 {}, {ps}\n{ind}%m2v.mq{c} = add i32 %m2v.mo{c}, {}\n", a[2].1, 16 + nv * vs + off));
                store(
                    &mut out,
                    ind,
                    c,
                    &h,
                    &format!("%m2v.mq{c}"),
                    &a[3].0,
                    &a[3].1,
                );
            } else if name == "air.set_index_mesh" {
                let (ity, iv) = (&a[2].0, &a[2].1);
                let wide = if ity == "i32" {
                    iv.clone()
                } else {
                    out.push_str(&format!("{ind}%m2v.mx{c} = zext {ity} {iv} to i32\n"));
                    format!("%m2v.mx{c}")
                };
                out.push_str(&format!("{ind}%m2v.mo{c} = mul i32 {}, 4\n{ind}%m2v.mq{c} = add i32 %m2v.mo{c}, {idx_off}\n", a[1].1));
                store(&mut out, ind, c, &h, &format!("%m2v.mq{c}"), "i32", &wide);
            } else if name == "air.set_primitive_count_mesh" {
                store(&mut out, ind, c, &h, "0", "i32", &a[1].1);
            } else {
                return Err(format!(
                    "mesh lowering: mesh intrinsic @{name} is not lowered"
                ));
            }
            continue;
        }
        out.push_str(&l);
        out.push('\n');
    }
    if mesh_value.is_empty() {
        return Err(format!("mesh lowering: no definition of @{entry}"));
    }
    if let Some(bad) = out.lines().find(|l| {
        l.contains("addrspace(7)")
            && !l.trim_start().starts_with('%')
            && !l.trim_start().starts_with("!")
    }) {
        if !bad.contains("type opaque") {
            return Err(format!(
                "mesh lowering: a mesh handle survived the lowering: {}",
                bad.trim()
            ));
        }
    }
    if let Some(bad) = out
        .lines()
        .find(|l| l.contains("_mesh(") && l.contains("@air."))
    {
        return Err(format!(
            "mesh lowering: an unlowered mesh intrinsic remains: {}",
            bad.trim()
        ));
    }

    let mut next = nodes.keys().next_back().copied().unwrap_or(0) + 1;
    let (buf_n, tgp_n, ntg_n, list_n, pay_n) = (next, next + 1, next + 2, next + 3, next + 4);
    next += 5;
    let _ = next;
    let grid_nodes = [tgp_n, ntg_n];
    let mut added = 0usize;
    let mut grid_index = [0usize; 2];
    for h in 0..2 {
        grid_index[h] = match &held[h] {
            Some((_, i, _)) => *i,
            None => {
                added += 1;
                nparams + added - 1
            }
        };
    }
    let new_args: Vec<String> = args
        .iter()
        .map(|a| {
            if *a == mesh_node {
                format!("!{buf_n}")
            } else if payload.is_some_and(|p| p.0 == *a) {
                format!("!{pay_n}")
            } else if let Some(h) = (0..2).find(|h| held[*h].as_ref().is_some_and(|x| x.0 == *a)) {
                format!("!{}", grid_nodes[h])
            } else {
                format!("!{a}")
            }
        })
        .chain(
            (0..2)
                .filter(|h| held[*h].is_none())
                .map(|h| format!("!{}", grid_nodes[h])),
        )
        .collect();
    let mut kernel = String::with_capacity(out.len() + 1024);
    for line in out.lines() {
        if line.starts_with("!air.mesh = ") {
            continue;
        }
        if line.starts_with(&format!("!{fnode} = ")) {
            let mut l = line.to_string();
            if !opaque {
                let fp = l
                    .find(&format!(")* @{entry},"))
                    .ok_or("mesh lowering: entry node has no function type")?;
                let ft_open = l[..fp]
                    .rfind(" (")
                    .ok_or("mesh lowering: entry node function type has no parameter list")?
                    + 1;
                let mut tys = split_args(&l[ft_open + 1..fp]);
                for held_builtin in &held {
                    match held_builtin {
                        Some((_, i, _)) => {
                            *tys.get_mut(*i)
                                .ok_or("mesh lowering: grid builtin past the function type")? =
                                "<3 x i32>".into()
                        }
                        None => tys.push("<3 x i32>".into()),
                    }
                }
                l.replace_range(ft_open + 1..fp, &tys.join(", "));
            }
            let (a0, a1) = entry_arg_list_span(&l, entry, arg_list)
                .ok_or("mesh lowering: the entry node's reference [1] is not its argument list")?;
            l.replace_range(a0..a1, &format!("!{list_n}"));
            kernel.push_str(&l);
            kernel.push('\n');
            continue;
        }
        kernel.push_str(line);
        kernel.push('\n');
    }
    if kernel.contains("\n!air.kernel = !{") {
        let at = kernel.find("\n!air.kernel = !{").unwrap() + "\n!air.kernel = !{".len();
        kernel.insert_str(at, &format!("!{fnode}, "));
    } else {
        kernel.push_str(&format!("!air.kernel = !{{!{fnode}}}\n"));
    }
    kernel.push_str(&format!(
        "!{buf_n} = !{{i32 {mesh_index}, !\"air.buffer\", !\"air.location_index\", i32 {MESH_OUT_BUFFER}, i32 1, !\"air.read_write\", !\"air.address_space\", i32 1, !\"air.arg_type_size\", i32 1, !\"air.arg_type_align_size\", i32 1, !\"air.arg_type_name\", !\"uchar\", !\"air.arg_name\", !\"m2v_mesh_out\"}}\n\
         !{tgp_n} = !{{i32 {}, !\"air.threadgroup_position_in_grid\", !\"air.arg_type_name\", !\"uint3\", !\"air.arg_name\", !\"m2v_tgp\"}}\n\
         !{ntg_n} = !{{i32 {}, !\"air.threadgroups_per_grid\", !\"air.arg_type_name\", !\"uint3\", !\"air.arg_name\", !\"m2v_ntg\"}}\n\
         !{list_n} = !{{{}}}\n",
        grid_index[0],
        grid_index[1],
        new_args.join(", ")
    ));
    if let Some((_, pi)) = payload {
        kernel.push_str(&format!(
            "!{pay_n} = !{{i32 {pi}, !\"air.buffer\", !\"air.location_index\", i32 {PAYLOAD_BUFFER}, i32 1, !\"air.read\", !\"air.address_space\", i32 1, !\"air.arg_type_size\", i32 1, !\"air.arg_type_align_size\", i32 1, !\"air.arg_type_name\", !\"uchar\", !\"air.arg_name\", !\"m2v_payload\"}}\n"
        ));
    }
    if link == Link::Indirect && !kernel.contains("declare i32 @air.atomic.global.max.u.i32(") {
        let at = kernel.find("\n!").map(|i| i + 1).unwrap_or(kernel.len());
        kernel.insert_str(at, atomic_max_decl(opaque));
    }

    let header: String = ll
        .lines()
        .filter(|l| l.starts_with("target datalayout") || l.starts_with("target triple"))
        .map(|l| format!("{l}\n"))
        .collect();
    let copy_named = |name: &str| -> Option<String> {
        let l = ll.lines().find(|l| l.starts_with(&format!("!{name} = ")))?;
        let n = *refs(l).first()?;
        Some(nodes.get(&n)?.clone())
    };
    let mut ret_tys = vec!["<4 x float>".to_string()];
    for a in vattrs.iter().chain(pattrs.iter()) {
        ret_tys.push(llvm_type(&a.type_name)?.0);
    }
    let ret = format!("<{{ {} }}>", ret_tys.join(", "));
    let hb = if linked { "%2" } else { "%1" };
    let mut v = String::new();
    v.push_str(&header);
    if linked {
        v.push_str(&format!("\ndefine {ret} @{VERTEX_ENTRY}(i32 noundef %0, i32 noundef %1, i8 addrspace(1)* noundef readonly %2) {{\n"));
    } else {
        v.push_str(&format!("\ndefine {ret} @{VERTEX_ENTRY}(i32 noundef %0, i8 addrspace(1)* noundef readonly %1) {{\n"));
    }
    let load = |v: &mut String, name: &str, off: &str, ty: &str| {
        v.push_str(&format!(
            "  %{name}.a = getelementptr inbounds i8, i8 addrspace(1)* {hb}, i32 {off}\n"
        ));
        v.push_str(&format!(
            "  %{name}.p = bitcast i8 addrspace(1)* %{name}.a to {ty} addrspace(1)*\n"
        ));
        v.push_str(&format!(
            "  %{name} = load {ty}, {ty} addrspace(1)* %{name}.p, align 4\n"
        ));
    };
    if linked {
        load(&mut v, "m.mx0", "0", "i32");
        load(&mut v, "m.no", "4", "i32");
        v.push_str(&format!(
            "  %m.big = icmp ugt i32 %m.mx0, {cap}\n  %m.mx = select i1 %m.big, i32 {cap}, i32 %m.mx0\n  \
             %m.j = udiv i32 %0, {pk}\n  %m.r = urem i32 %0, {pk}\n  %m.p = udiv i32 %m.r, {k}\n  %m.jok = icmp ult i32 %m.j, %m.mx\n  \
             %m.t0 = mul i32 %1, %m.mx\n  %m.t = add i32 %m.t0, %m.j\n  %m.mo0 = mul i32 %m.no, {rs}\n  %m.mo = add i32 %m.mo0, {tr}\n  \
             %m.bk0 = mul i32 %m.t, {block}\n  %m.bk1 = add i32 %m.bk0, %m.mo\n  %m.blk = select i1 %m.jok, i32 %m.bk1, i32 {HEADER_BYTES}\n"
        ));
        load(&mut v, "m.cnt", "%m.blk", "i32");
        v.push_str("  %m.live0 = icmp ult i32 %m.p, %m.cnt\n  %m.live = and i1 %m.live0, %m.jok\n");
    } else {
        v.push_str(&format!("  %m.t = udiv i32 %0, {pk}\n  %m.r = urem i32 %0, {pk}\n  %m.p = udiv i32 %m.r, {k}\n  %m.blk = mul i32 %m.t, {block}\n"));
        load(&mut v, "m.cnt", "%m.blk", "i32");
        v.push_str("  %m.live = icmp ult i32 %m.p, %m.cnt\n");
    }
    v.push_str(&format!("  %m.io = mul i32 %m.r, 4\n  %m.io2 = add i32 %m.io, {idx_off}\n  %m.io3 = add i32 %m.io2, %m.blk\n"));
    load(&mut v, "m.idx", "%m.io3", "i32");
    v.push_str(&format!(
        "  %m.iok = icmp ult i32 %m.idx, {nv}\n  %m.ok = and i1 %m.live, %m.iok\n  %m.lv = select i1 %m.ok, i32 %m.idx, i32 0\n  \
         %m.vo = mul i32 %m.lv, {vs}\n  %m.vb = add i32 %m.vo, 16\n  %m.vb2 = add i32 %m.vb, %m.blk\n  \
         %m.po = mul i32 %m.p, {ps}\n  %m.pb = add i32 %m.po, {}\n  %m.pb2 = add i32 %m.pb, %m.blk\n",
        16 + nv * vs
    ));
    load(&mut v, "m.pos0", "%m.vb2", "<4 x float>");
    v.push_str("  %m.pos = select i1 %m.ok, <4 x float> %m.pos0, <4 x float> <float 2.000000e+00, float 2.000000e+00, float 2.000000e+00, float 1.000000e+00>\n");
    let mut vals = vec![("<4 x float>".to_string(), "%m.pos".to_string())];
    for (j, a) in vattrs.iter().enumerate() {
        let ty = llvm_type(&a.type_name)?.0;
        v.push_str(&format!("  %m.va{j}o = add i32 %m.vb2, {}\n", 16 * (1 + j)));
        load(&mut v, &format!("m.va{j}"), &format!("%m.va{j}o"), &ty);
        vals.push((ty, format!("%m.va{j}")));
    }
    for (j, a) in pattrs.iter().enumerate() {
        let ty = llvm_type(&a.type_name)?.0;
        v.push_str(&format!("  %m.pa{j}o = add i32 %m.pb2, {}\n", 16 * j));
        load(&mut v, &format!("m.pa{j}"), &format!("%m.pa{j}o"), &ty);
        vals.push((ty, format!("%m.pa{j}")));
    }
    let mut prev = "undef".to_string();
    for (j, (ty, val)) in vals.iter().enumerate() {
        v.push_str(&format!(
            "  %m.s{j} = insertvalue {ret} {prev}, {ty} {val}, {j}\n"
        ));
        prev = format!("%m.s{j}");
    }
    v.push_str(&format!("  ret {ret} {prev}\n}}\n\n"));
    v.push_str("!air.vertex = !{!0}\n");
    let mut md = Vec::new();
    let mut outs = vec![
        "!{!\"air.position\", !\"air.arg_type_name\", !\"float4\", !\"air.arg_name\", !\"pos\"}"
            .to_string(),
    ];
    for a in vattrs.iter().chain(pattrs.iter()) {
        outs.push(format!("!{{!\"air.vertex_output\", !\"{}\", !\"air.arg_type_name\", !\"{}\", !\"air.arg_name\", !\"{}\"}}", a.generated, a.type_name, a.arg_name));
    }
    let first_out = 6u32;
    let bi = if linked { 2 } else { 1 };
    if linked {
        md.push(format!(
            "!0 = !{{{ret} (i32, i32, i8 addrspace(1)*)* @{VERTEX_ENTRY}, !1, !2}}"
        ));
        md.push("!2 = !{!3, !5, !4}".into());
        md.push("!5 = !{i32 1, !\"air.instance_id\", !\"air.arg_type_name\", !\"uint\", !\"air.arg_name\", !\"iid\"}".into());
    } else {
        md.push(format!(
            "!0 = !{{{ret} (i32, i8 addrspace(1)*)* @{VERTEX_ENTRY}, !1, !2}}"
        ));
        md.push("!2 = !{!3, !4}".into());
        md.push("!5 = !{}".into());
    }
    md.push(format!(
        "!1 = !{{{}}}",
        (0..outs.len())
            .map(|i| format!("!{}", first_out + i as u32))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    md.push("!3 = !{i32 0, !\"air.vertex_id\", !\"air.arg_type_name\", !\"uint\", !\"air.arg_name\", !\"vid\"}".into());
    md.push(format!("!4 = !{{i32 {bi}, !\"air.buffer\", !\"air.location_index\", i32 {MESH_OUT_BUFFER}, i32 1, !\"air.read\", !\"air.address_space\", i32 1, !\"air.arg_type_size\", i32 1, !\"air.arg_type_align_size\", i32 1, !\"air.arg_type_name\", !\"uchar\", !\"air.arg_name\", !\"m2v_mesh_in\"}}"));
    for (i, o) in outs.iter().enumerate() {
        md.push(format!("!{} = {o}", first_out + i as u32));
    }
    let mut n = first_out + outs.len() as u32;
    for name in ["air.version", "air.language_version"] {
        if let Some(body) = copy_named(name) {
            v.push_str(&format!("!{name} = !{{!{n}}}\n"));
            md.push(format!("!{n} = {body}"));
            n += 1;
        }
    }
    v.push('\n');
    for m in md {
        v.push_str(&m);
        v.push('\n');
    }
    Ok(LoweredMesh {
        kernel_ll: kernel,
        vertex_ll: v,
        layout,
        tr,
    })
}

pub fn lower_object(
    ll: &str,
    entry: &str,
    mesh: &MeshLayout,
    tr: u32,
    rs: u32,
) -> Result<String, String> {
    let opaque = !ll.contains("%struct._mesh_grid_properties_t addrspace(3)*");
    let mgp_ty = if opaque {
        "ptr addrspace(3)"
    } else {
        "%struct._mesh_grid_properties_t addrspace(3)*"
    };
    let out_ty = if opaque {
        "ptr addrspace(1)"
    } else {
        "i8 addrspace(1)*"
    };
    let pk = mesh.np * mesh.k;
    let nodes = node_lines(ll);
    let list = ll
        .lines()
        .find(|l| l.starts_with("!air.object = "))
        .ok_or_else(|| {
            format!("object lowering: no !air.object list (is \"{entry}\" an [[object]] function?)")
        })?;
    let at = format!("@{entry},");
    let fnode = refs(list)
        .into_iter()
        .find(|n| nodes.get(n).is_some_and(|b| b.contains(&at)))
        .ok_or_else(|| format!("object lowering: \"{entry}\" is not in !air.object"))?;
    let fbody = nodes[&fnode].clone();
    let arg_list = entry_arg_list(&fbody[fbody.find(&at).unwrap()..])
        .ok_or("object lowering: entry node has no argument list")?;
    let args: Vec<u32> = refs(&nodes[&arg_list]);
    let (mut payload, mut mgp) = (None, None);
    let mut held: [Option<(u32, usize, String)>; 2] = [None, None];
    for a in &args {
        let b = &nodes[a];
        let idx = || {
            first_int(b)
                .map(|i| i as usize)
                .ok_or("object lowering: argument has no index")
        };
        if b.contains("!\"air.payload\"") {
            payload = Some((*a, idx()?));
        } else if b.contains("!\"air.mesh_grid_properties\"") {
            mgp = Some((*a, idx()?));
        } else if b.contains("!\"air.buffer\"") && b.contains("!\"air.location_index\"") {
            let loc = b
                .split("!\"air.location_index\", i32 ")
                .nth(1)
                .and_then(|r| r.split(',').next())
                .and_then(|n| n.trim().parse::<u32>().ok());
            if loc == Some(MESH_OUT_BUFFER) || loc == Some(PAYLOAD_BUFFER) {
                return Err(format!(
                    "object lowering: the object function binds buffer {}, which the lowering owns",
                    loc.unwrap()
                ));
            }
        }
        for (h, key) in [
            "air.threadgroup_position_in_grid",
            "air.threadgroups_per_grid",
        ]
        .iter()
        .enumerate()
        {
            if b.contains(&format!("!\"{key}\"")) {
                let tn = quoted_after(b, "air.arg_type_name")
                    .ok_or("object lowering: grid builtin without a type")?
                    .to_string();
                if !["uint", "uint2", "uint3"].contains(&tn.as_str()) {
                    return Err(format!(
                        "object lowering: grid builtin {key} of type {tn} is not lowered"
                    ));
                }
                held[h] = Some((*a, idx()?, tn));
            }
        }
    }
    let (mgp_node, mgp_index) = mgp.ok_or_else(|| {
        format!(
            "object lowering: \"{entry}\" takes no mesh_grid_properties - it can launch no mesh"
        )
    })?;

    let retyped = if opaque {
        ll.replace("addrspace(6)", "addrspace(1)")
    } else {
        ll.replace(mgp_ty, out_ty)
            .replace("addrspace(6)", "addrspace(1)")
    };
    let mut out = String::with_capacity(ll.len() + 4096);
    let (mut in_entry, mut prologue, mut mgp_value, mut nparams, mut c) =
        (false, None::<String>, String::new(), 0usize, 0usize);
    let mut entry_num = 0usize;
    for line in retyped.lines() {
        let t = line.trim_start();
        let ind = &line[..line.len() - t.len()];
        if let Some(p) = prologue.take() {
            let label =
                !t.starts_with(';') && t.split(';').next().unwrap_or("").trim_end().ends_with(':');
            if label {
                out.push_str(line);
                out.push('\n');
                out.push_str(&p);
                continue;
            }
            out.push_str(&format!("{entry_num}:\n"));
            out.push_str(&p);
        }
        if t.starts_with("declare ")
            && t.contains("@air.set_threadgroups_per_grid_mesh_properties(")
        {
            continue;
        }
        if t.starts_with("define ") && t.contains(&format!("@{entry}(")) {
            let open = t.find(&format!("@{entry}(")).unwrap() + entry.len() + 1;
            let close = paren_close(t, open)
                .ok_or("object lowering: entry parameter list is not closed")?;
            let params = split_args(&t[open + 1..close]);
            nparams = params.len();
            entry_num = implicit_entry_label(&params);
            let mut ps = params.clone();
            let mp = ps
                .get(mgp_index)
                .ok_or("object lowering: mesh_grid_properties index past the parameters")?
                .clone();
            mgp_value = ty_val(&mp).1;
            if opaque {
                ps[mgp_index] = format!("{out_ty} {mgp_value}");
            }
            let mut narrow = String::new();
            for (h, name) in ["%m2v.tgp", "%m2v.ntg"].iter().enumerate() {
                match &held[h] {
                    Some((_, i, tn)) => {
                        let (_, orig) = ty_val(
                            ps.get(*i)
                                .ok_or("object lowering: grid builtin index past the parameters")?,
                        );
                        ps[*i] = format!("<3 x i32> noundef {name}");
                        narrow.push_str(&match tn.as_str() {
                            "uint" => format!("  {orig} = extractelement <3 x i32> {name}, i32 0\n"),
                            "uint2" => format!("  {orig} = shufflevector <3 x i32> {name}, <3 x i32> undef, <2 x i32> <i32 0, i32 1>\n"),
                            _ => format!("  {orig} = shufflevector <3 x i32> {name}, <3 x i32> undef, <3 x i32> <i32 0, i32 1, i32 2>\n"),
                        });
                    }
                    None => ps.push(format!("<3 x i32> noundef {name}")),
                }
            }
            let mut pdef = String::new();
            if let Some((_, pi)) = payload {
                let p = ps
                    .get(pi)
                    .ok_or("object lowering: payload index past the parameters")?
                    .clone();
                let (pty, orig) = ty_val(&p);
                let pty = bare_ptr_ty(&pty);
                ps[pi] = format!("{}%m2v.plraw", &p[..p.len() - orig.len()]);
                pdef = if opaque {
                    format!("  {orig} = getelementptr inbounds i8, ptr addrspace(1) %m2v.plraw, i32 %m2v.plo\n")
                } else {
                    format!(
                        "  %m2v.plb = bitcast {pty} %m2v.plraw to i8 addrspace(1)*\n  %m2v.pla = getelementptr inbounds i8, i8 addrspace(1)* %m2v.plb, i32 %m2v.plo\n  \
                         {orig} = bitcast i8 addrspace(1)* %m2v.pla to {pty}\n"
                    )
                };
            }
            out.push_str(&format!(
                "{ind}{}{}{}\n",
                &t[..open + 1],
                ps.join(", "),
                &t[close..]
            ));
            prologue = Some(format!(
                "{narrow}  %m2v.t0 = extractelement <3 x i32> %m2v.tgp, i32 0\n  %m2v.t1 = extractelement <3 x i32> %m2v.tgp, i32 1\n  \
                 %m2v.t2 = extractelement <3 x i32> %m2v.tgp, i32 2\n  %m2v.n0 = extractelement <3 x i32> %m2v.ntg, i32 0\n  \
                 %m2v.n1 = extractelement <3 x i32> %m2v.ntg, i32 1\n  %m2v.l0 = mul i32 %m2v.t2, %m2v.n1\n  \
                 %m2v.l1 = add i32 %m2v.l0, %m2v.t1\n  %m2v.l2 = mul i32 %m2v.l1, %m2v.n0\n  %m2v.lin = add i32 %m2v.l2, %m2v.t0\n  \
                 %m2v.r0 = mul i32 %m2v.lin, {rs}\n  %m2v.rec = add i32 %m2v.r0, {tr}\n  %m2v.plo = add i32 %m2v.rec, 16\n{pdef}"
            ));
            in_entry = true;
            continue;
        }
        if in_entry && t == "}" {
            in_entry = false;
        }
        let l = if t.contains("@air.wg.barrier(") || t.contains("@air.simdgroup.barrier(") {
            respell_object_barrier(line)?
        } else {
            line.to_string()
        };
        let lt = l.trim_start();
        if let Some(open) = lt
            .find("@air.set_threadgroups_per_grid_mesh_properties(")
            .filter(|_| lt.contains("call "))
        {
            if !in_entry {
                return Err("object lowering: set_threadgroups_per_grid outside the object entry (in a helper) is not lowered".into());
            }
            let name_end = open + "@air.set_threadgroups_per_grid_mesh_properties".len();
            let close = paren_close(lt, name_end)
                .ok_or("object lowering: set_threadgroups_per_grid is not closed")?;
            let a: Vec<(String, String)> = split_args(&lt[name_end + 1..close])
                .iter()
                .map(|s| ty_val(s))
                .collect();
            if a.len() != 2 || a[1].0 != "<3 x i32>" {
                return Err(format!(
                    "object lowering: unexpected set_threadgroups_per_grid shape: {}",
                    lt
                ));
            }
            let (h, g) = (&a[0].1, &a[1].1);
            if *h != mgp_value {
                return Err("object lowering: set_threadgroups_per_grid on a value that is not the entry's mesh_grid_properties".into());
            }
            c += 1;
            if opaque {
                out.push_str(&format!("{ind}%m2v.ga{c} = getelementptr inbounds i8, ptr addrspace(1) {h}, i32 %m2v.rec\n{ind}store <3 x i32> {g}, ptr addrspace(1) %m2v.ga{c}, align 4\n"));
            } else {
                out.push_str(&format!(
                    "{ind}%m2v.ga{c} = getelementptr inbounds i8, i8 addrspace(1)* {h}, i32 %m2v.rec\n\
                     {ind}%m2v.gp{c} = bitcast i8 addrspace(1)* %m2v.ga{c} to <3 x i32> addrspace(1)*\n\
                     {ind}store <3 x i32> {g}, <3 x i32> addrspace(1)* %m2v.gp{c}, align 4\n"
                ));
            }
            out.push_str(&format!(
                "{ind}%m2v.gx{c} = extractelement <3 x i32> {g}, i32 0\n{ind}%m2v.gy{c} = extractelement <3 x i32> {g}, i32 1\n\
                 {ind}%m2v.gz{c} = extractelement <3 x i32> {g}, i32 2\n{ind}%m2v.gq{c} = mul i32 %m2v.gx{c}, %m2v.gy{c}\n\
                 {ind}%m2v.gr{c} = mul i32 %m2v.gq{c}, %m2v.gz{c}\n{ind}%m2v.gv{c} = mul i32 %m2v.gr{c}, {pk}\n"
            ));
            out.push_str(&emit_atomic_max(
                opaque,
                ind,
                &format!("%m2v.gm{c}"),
                h,
                0,
                &format!("%m2v.gr{c}"),
            ));
            out.push_str(&emit_atomic_max(
                opaque,
                ind,
                &format!("%m2v.gn{c}"),
                h,
                16,
                &format!("%m2v.gv{c}"),
            ));
            continue;
        }
        out.push_str(&l);
        out.push('\n');
    }
    if mgp_value.is_empty() {
        return Err(format!("object lowering: no definition of @{entry}"));
    }
    if c == 0 {
        return Err(format!("object lowering: \"{entry}\" never calls set_threadgroups_per_grid - it can launch no mesh"));
    }
    if let Some(bad) = out
        .lines()
        .find(|l| l.contains("@air.set_threadgroups_per_grid_mesh_properties("))
    {
        return Err(format!(
            "object lowering: an unlowered set_threadgroups_per_grid remains: {}",
            bad.trim()
        ));
    }

    let next = nodes.keys().next_back().copied().unwrap_or(0) + 1;
    let (buf_n, pay_n, tgp_n, ntg_n, list_n) = (next, next + 1, next + 2, next + 3, next + 4);
    let grid_nodes = [tgp_n, ntg_n];
    let mut added = 0usize;
    let mut grid_index = [0usize; 2];
    for h in 0..2 {
        grid_index[h] = match &held[h] {
            Some((_, i, _)) => *i,
            None => {
                added += 1;
                nparams + added - 1
            }
        };
    }
    let new_args: Vec<String> = args
        .iter()
        .map(|a| {
            if *a == mgp_node {
                format!("!{buf_n}")
            } else if payload.is_some_and(|p| p.0 == *a) {
                format!("!{pay_n}")
            } else if let Some(h) = (0..2).find(|h| held[*h].as_ref().is_some_and(|x| x.0 == *a)) {
                format!("!{}", grid_nodes[h])
            } else {
                format!("!{a}")
            }
        })
        .chain(
            (0..2)
                .filter(|h| held[*h].is_none())
                .map(|h| format!("!{}", grid_nodes[h])),
        )
        .collect();
    let mut kernel = String::with_capacity(out.len() + 1024);
    for line in out.lines() {
        if line.starts_with("!air.object = ") {
            continue;
        }
        if line.starts_with(&format!("!{fnode} = ")) {
            let mut l = line.to_string();
            if !opaque {
                let fp = l
                    .find(&format!(")* @{entry},"))
                    .ok_or("object lowering: entry node has no function type")?;
                let ft_open = l[..fp]
                    .rfind(" (")
                    .ok_or("object lowering: entry node function type has no parameter list")?
                    + 1;
                let mut tys = split_args(&l[ft_open + 1..fp]);
                for held_builtin in &held {
                    match held_builtin {
                        Some((_, i, _)) => {
                            *tys.get_mut(*i)
                                .ok_or("object lowering: grid builtin past the function type")? =
                                "<3 x i32>".into()
                        }
                        None => tys.push("<3 x i32>".into()),
                    }
                }
                l.replace_range(ft_open + 1..fp, &tys.join(", "));
            }
            let (a0, a1) = entry_arg_list_span(&l, entry, arg_list).ok_or(
                "object lowering: the entry node's reference [1] is not its argument list",
            )?;
            l.replace_range(a0..a1, &format!("!{list_n}"));
            kernel.push_str(&l);
            kernel.push('\n');
            continue;
        }
        kernel.push_str(line);
        kernel.push('\n');
    }
    if kernel.contains("\n!air.kernel = !{") {
        let at = kernel.find("\n!air.kernel = !{").unwrap() + "\n!air.kernel = !{".len();
        kernel.insert_str(at, &format!("!{fnode}, "));
    } else {
        kernel.push_str(&format!("!air.kernel = !{{!{fnode}}}\n"));
    }
    kernel.push_str(&format!(
        "!{buf_n} = !{{i32 {mgp_index}, !\"air.buffer\", !\"air.location_index\", i32 {MESH_OUT_BUFFER}, i32 1, !\"air.read_write\", !\"air.address_space\", i32 1, !\"air.arg_type_size\", i32 1, !\"air.arg_type_align_size\", i32 1, !\"air.arg_type_name\", !\"uchar\", !\"air.arg_name\", !\"m2v_mesh_grid\"}}\n\
         !{tgp_n} = !{{i32 {}, !\"air.threadgroup_position_in_grid\", !\"air.arg_type_name\", !\"uint3\", !\"air.arg_name\", !\"m2v_tgp\"}}\n\
         !{ntg_n} = !{{i32 {}, !\"air.threadgroups_per_grid\", !\"air.arg_type_name\", !\"uint3\", !\"air.arg_name\", !\"m2v_ntg\"}}\n\
         !{list_n} = !{{{}}}\n",
        grid_index[0],
        grid_index[1],
        new_args.join(", ")
    ));
    if let Some((_, pi)) = payload {
        kernel.push_str(&format!(
            "!{pay_n} = !{{i32 {pi}, !\"air.buffer\", !\"air.location_index\", i32 {PAYLOAD_BUFFER}, i32 1, !\"air.read_write\", !\"air.address_space\", i32 1, !\"air.arg_type_size\", i32 1, !\"air.arg_type_align_size\", i32 1, !\"air.arg_type_name\", !\"uchar\", !\"air.arg_name\", !\"m2v_payload\"}}\n"
        ));
    }
    if !kernel.contains("declare i32 @air.atomic.global.max.u.i32(") {
        let at = kernel.find("\n!").map(|i| i + 1).unwrap_or(kernel.len());
        kernel.insert_str(at, atomic_max_decl(opaque));
    }
    Ok(kernel)
}

#[cfg(test)]
mod mesh_lower_tests {
    use super::*;
    const M42: &str = include_str!("testdata/m42_mesh.ll");
    const OBJ: &str = include_str!("testdata/mesh_object.ll");

    #[test]
    fn a_mesh_entry_becomes_a_kernel_with_every_mesh_intrinsic_a_store() {
        let m = lower_mesh(M42, "m_cols").expect("lowers");
        assert!(m.kernel_ll.contains("!air.kernel = !{"), "no kernel list");
        assert!(
            !m.kernel_ll.contains("!air.mesh = "),
            "the mesh list must be gone"
        );
        assert!(!m.kernel_ll.contains("@air.set_"), "an intrinsic survived");
        assert!(
            !m.kernel_ll
                .lines()
                .any(|l| l.contains("addrspace(7)*") && !l.contains("type opaque")),
            "a mesh handle survived"
        );
        assert!(m
            .kernel_ll
            .contains("%m2v.mesh = getelementptr inbounds i8, i8 addrspace(1)* %0, i32 %m2v.boff"));
        assert!(
            m.kernel_ll.contains("air.threadgroups_per_grid")
                && m.kernel_ll.contains("i32 30, i32 1, !\"air.read_write\"")
        );
        assert!(
            m.kernel_ll.contains("(i8 addrspace(1)* %m2v.mesh,"),
            "{}",
            m.kernel_ll
        );
    }

    fn with_attr(ll: &str, entry: &str, n: u32) -> String {
        let at = format!("@{entry},");
        let mut out = String::new();
        for l in ll.lines() {
            if l.starts_with('!') && l.contains(&at) && l.ends_with('}') {
                out.push_str(&format!("{}, !{n}}}\n", &l[..l.len() - 1]));
            } else {
                out.push_str(l);
                out.push('\n');
            }
        }
        out.push_str(&format!(
            "!{n} = !{{!\"air.max_work_group_size\", i32 32}}\n"
        ));
        out
    }

    fn body(ll: &str, name: &str) -> String {
        let pat = format!("@{name}(");
        let mut on = false;
        let mut out = String::new();
        for l in ll.lines() {
            if l.starts_with("define ") && l.contains(&pat) {
                on = true;
            }
            if on {
                out.push_str(l);
                out.push('\n');
                if l == "}" {
                    break;
                }
            }
        }
        out
    }

    #[test]
    fn a_mesh_entry_with_an_attribute_after_its_argument_list_lowers_like_its_plain_twin() {
        let plain = lower_mesh(M42, "m_cols").expect("plain lowers");
        let ll = with_attr(M42, "m_cols", 9999);
        assert!(
            ll.contains(", !9999}"),
            "the fixture rewrite must append the attribute"
        );
        let m = lower_mesh(&ll, "m_cols").expect("the attributed twin lowers");
        assert_eq!(m.layout.block, plain.layout.block);
        assert_eq!(
            body(&m.kernel_ll, "m_cols"),
            body(&plain.kernel_ll, "m_cols"),
            "the kernel body is the plain twin's"
        );
        assert!(
            !body(&m.kernel_ll, "m_cols").is_empty(),
            "an empty body would make the comparison vacuous"
        );
        let e = m
            .kernel_ll
            .lines()
            .find(|l| l.starts_with('!') && l.contains("@m_cols,"))
            .unwrap();
        assert!(
            e.ends_with(", !9999}"),
            "the attribute stays after the rewritten argument list: {e}"
        );
    }

    #[test]
    fn an_object_and_its_mesh_with_attributes_after_their_argument_lists_lower_like_the_plain_pair()
    {
        let (pm, po) = obj_pair();
        let ll = with_attr(&with_attr(OBJ, "o_quads", 9998), "m_quad", 9999);
        assert_eq!(
            payload_len(&ll, "o_quads").unwrap(),
            16,
            "the payload is found past the attribute"
        );
        let rs = record_stride(16);
        let m = lower_mesh_linked(&ll, "m_quad", Link::Object, rs, 1024)
            .expect("the attributed mesh lowers linked");
        let o = lower_object(&ll, "o_quads", &m.layout, m.tr, rs)
            .expect("the attributed object lowers");
        assert_eq!(m.tr, pm.tr);
        assert_eq!(
            body(&o, "o_quads"),
            body(&po, "o_quads"),
            "the object kernel body is the plain pair's"
        );
        assert_eq!(
            body(&m.kernel_ll, "m_quad"),
            body(&pm.kernel_ll, "m_quad"),
            "the mesh kernel body is the plain pair's"
        );
        assert!(
            !body(&o, "o_quads").is_empty() && !body(&m.kernel_ll, "m_quad").is_empty(),
            "empty bodies would be vacuous"
        );
    }

    #[test]
    fn an_object_grid_spelled_as_a_splat_constant_lowers_like_the_bracketed_vector() {
        let (m, _) = obj_pair();
        let rs = record_stride(payload_len(OBJ, "o_quads").unwrap());
        let sp = OBJ.replace(
            "<3 x i32> <i32 2, i32 1, i32 1>)",
            "<3 x i32> splat (i32 2))",
        );
        assert_ne!(sp, OBJ, "the fixture rewrite must produce a splat grid");
        let o = lower_object(&sp, "o_quads", &m.layout, m.tr, rs).expect("a splat grid lowers");
        assert!(
            o.contains(
                "store <3 x i32> <i32 2, i32 2, i32 2>, <3 x i32> addrspace(1)* %m2v.gp1, align 4"
            ),
            "{o}"
        );
        assert!(
            !o.contains("splat ("),
            "the splat is expanded, never passed on half-parsed"
        );
    }

    #[test]
    fn ty_val_reads_a_splat_as_one_value_and_leaves_every_other_operand_alone() {
        assert_eq!(
            ty_val("<3 x i32> splat (i32 2)"),
            ("<3 x i32>".to_string(), "<i32 2, i32 2, i32 2>".to_string())
        );
        assert_eq!(
            ty_val("<3 x i32> noundef splat (i32 7)"),
            ("<3 x i32>".to_string(), "<i32 7, i32 7, i32 7>".to_string())
        );
        assert_eq!(
            ty_val("<3 x i32> <i32 2, i32 1, i32 1>"),
            ("<3 x i32>".to_string(), "<i32 2, i32 1, i32 1>".to_string())
        );
        assert_eq!(ty_val("i32 %x"), ("i32".to_string(), "%x".to_string()));
    }

    #[test]
    fn the_argument_list_is_reference_one_after_the_needle_and_a_short_node_says_so() {
        assert_eq!(entry_arg_list("@f, !10, !11, !12}"), Some(11));
        assert_eq!(
            entry_arg_list("@f, !10, !11, !\"early_fragment_tests\"}"),
            Some(11)
        );
        assert_eq!(
            entry_arg_list("@f, !10}"),
            None,
            "a node with no argument list names nothing"
        );
        let l = "!9 = !{ptr @f, !10, !11, !12}";
        assert_eq!(
            entry_arg_list_span(l, "f", 11).map(|(a, b)| &l[a..b]),
            Some("!11")
        );
        assert_eq!(
            entry_arg_list_span(l, "f", 12),
            None,
            "an attribute is never taken for the argument list"
        );
    }

    #[test]
    fn the_layout_matches_the_declared_mesh_type() {
        let m = lower_mesh(M42, "m_cols").unwrap();
        assert_eq!(
            m.layout,
            MeshLayout {
                nv: 8,
                np: 4,
                k: 3,
                vs: 32,
                ps: 16,
                idx_off: 336,
                block: 384
            }
        );
    }

    #[test]
    fn the_generated_vertex_carries_the_fragments_varyings_by_name_and_culls_dead_primitives() {
        let m = lower_mesh(M42, "m_cols").unwrap();
        assert!(
            m.vertex_ll
                .contains("!\"air.vertex_output\", !\"generated(3colDv4_f)\""),
            "{}",
            m.vertex_ll
        );
        assert!(m
            .vertex_ll
            .contains("!\"air.vertex_output\", !\"generated(4tintDv4_f)\""));
        assert!(m.vertex_ll.contains("%m.live = icmp ult i32 %m.p, %m.cnt"));
        assert!(
            m.vertex_ll.contains("udiv i32 %0, 12"),
            "PK = 4 primitives x 3 corners"
        );
    }

    #[test]
    fn a_grid_builtin_the_user_holds_is_retyped_in_place_never_duplicated() {
        let m = lower_mesh(M42, "m_cols").unwrap();
        let k = &m.kernel_ll;
        assert_eq!(
            k.matches("!\"air.threadgroup_position_in_grid\"").count(),
            3,
            "m_cols's and m_2d's original nodes stay in the module, plus ONE uint3 for the entry"
        );
        let list = k.lines().find(|l| l.starts_with("!9 = ")).unwrap();
        let entry_args = list
            .rsplit(", ")
            .next()
            .unwrap()
            .trim_end_matches('}')
            .to_string();
        let node = k
            .lines()
            .find(|l| l.starts_with(&format!("{entry_args} = ")))
            .unwrap();
        assert!(
            !node.contains("!22"),
            "the entry must no longer list the uint gid node: {node}"
        );
        assert!(
            k.contains("i32 noundef %3, <3 x i32> noundef %m2v.tgp, <3 x i32> noundef %m2v.ntg)"),
            "{}",
            k.lines().find(|l| l.contains("@m_cols(")).unwrap()
        );
        assert!(k.contains("  %4 = extractelement <3 x i32> %m2v.tgp, i32 0\n"));
        assert!(
            k.contains("i32, <3 x i32>, <3 x i32>)* @m_cols,"),
            "function type"
        );
        assert!(k.contains(
            "!{i32 4, !\"air.threadgroup_position_in_grid\", !\"air.arg_type_name\", !\"uint3\""
        ));
        assert!(k.contains("!{i32 5, !\"air.threadgroups_per_grid\""));
    }

    #[test]
    fn a_two_d_grid_entry_lowers_and_keeps_its_own_threadgroup_argument() {
        let m = lower_mesh(M42, "m_2d").expect("lowers");
        assert!(
            m.kernel_ll.contains(
                "i32 noundef %2, <3 x i32> noundef %m2v.tgp, <3 x i32> noundef %m2v.ntg)"
            ),
            "{}",
            m.kernel_ll
                .lines()
                .find(|l| l.contains("@m_2d("))
                .unwrap_or("")
        );
        assert!(m.kernel_ll.contains(
            "  %3 = shufflevector <3 x i32> %m2v.tgp, <3 x i32> undef, <2 x i32> <i32 0, i32 1>\n"
        ));
    }

    #[test]
    fn a_payload_mesh_drawn_without_its_object_stage_is_refused() {
        let e = lower_mesh(OBJ, "m_quad").expect_err("must refuse");
        assert!(e.contains("object stage"), "{e}");
        let e = lower_mesh_linked(OBJ, "m_quad", Link::Indirect, 32, 64).expect_err("must refuse");
        assert!(e.contains("without its object stage"), "{e}");
    }

    fn obj_pair() -> (LoweredMesh, String) {
        let rs = record_stride(payload_len(OBJ, "o_quads").unwrap());
        let m =
            lower_mesh_linked(OBJ, "m_quad", Link::Object, rs, 1024).expect("mesh lowers linked");
        let o = lower_object(OBJ, "o_quads", &m.layout, m.tr, rs).expect("object lowers");
        (m, o)
    }

    #[test]
    fn the_payload_size_is_found_in_opaque_pointer_air_as_the_driver_hands_it() {
        let mut opaque = String::new();
        for l in OBJ.lines() {
            let l = match (l.find("= !{void ("), l.find(")* @")) {
                (Some(a), Some(b)) if a < b => format!("{}= !{{ptr {}", &l[..a], &l[b + 3..]),
                _ => l.to_string(),
            };
            opaque.push_str(&l);
            opaque.push('\n');
        }
        assert!(
            opaque.contains("!{ptr @o_quads,"),
            "the fixture rewrite must produce an opaque entry node"
        );
        assert_eq!(
            payload_len(&opaque, "o_quads").unwrap(),
            payload_len(OBJ, "o_quads").unwrap()
        );
        assert!(
            payload_len(OBJ, "o_quads").unwrap() > 0,
            "o_quads declares a payload"
        );
        let e = payload_len(&opaque, "not_an_entry").unwrap_err();
        assert!(e.contains("neither !air.mesh nor !air.object"), "{e}");
    }

    #[test]
    fn the_payload_size_is_read_from_the_object_and_rounds_into_the_record_stride() {
        assert_eq!(payload_len(OBJ, "o_quads").unwrap(), 16);
        assert_eq!(record_stride(16), 32);
        assert_eq!(record_stride(17), 48);
        assert_eq!(
            payload_len(OBJ, "m_only").unwrap(),
            0,
            "a mesh without a payload states none"
        );
    }

    #[test]
    fn an_object_entry_becomes_a_kernel_that_stores_its_grid_and_raises_the_header_atomically() {
        let (m, o) = obj_pair();
        assert_eq!(m.tr, HEADER_BYTES + m.layout.block);
        assert!(
            !o.contains("!air.object = "),
            "the object list must be gone"
        );
        assert!(o.contains("!air.kernel = !{"));
        assert!(
            !o.contains("@air.set_threadgroups_per_grid_mesh_properties("),
            "the grid call survived"
        );
        assert!(
            !o.contains("addrspace(6)") && !o.contains("_mesh_grid_properties_t addrspace(3)*"),
            "object_data / grid handle survived"
        );
        assert!(o.contains(&format!("%m2v.rec = add i32 %m2v.r0, {}", m.tr)));
        assert!(
            o.contains(
                "store <3 x i32> <i32 2, i32 1, i32 1>, <3 x i32> addrspace(1)* %m2v.gp1, align 4"
            ),
            "{o}"
        );
        assert!(o.contains("@air.atomic.global.max.u.i32(i32 addrspace(1)* %m2v.gm1.p, i32 %m2v.gr1, i32 0, i32 2, i1 true)"));
        assert!(
            o.contains("%m2v.gv1 = mul i32 %m2v.gr1, 12"),
            "PK = mesh<.., 8, 4, triangle>: 4 primitives x 3 corners"
        );
        assert!(
            o.contains("%m2v.gn1.a = getelementptr inbounds i8, i8 addrspace(1)* %1, i32 16\n"),
            "draw.vertexCount lives at 16"
        );
        assert!(o.contains("declare i32 @air.atomic.global.max.u.i32("));
        assert!(
            o.contains("i32 30, i32 1, !\"air.read_write\"")
                && o.contains("i32 29, i32 1, !\"air.read_write\"")
        );
        assert!(o.contains("%struct.Payload addrspace(1)* nocapture noundef writeonly align 4 dereferenceable(16) %m2v.plraw"), "{}", o.lines().find(|l| l.contains("@o_quads(")).unwrap());
        assert!(o.contains(
            "  %0 = bitcast i8 addrspace(1)* %m2v.pla to %struct.Payload addrspace(1)*\n"
        ));
    }

    #[test]
    fn the_mesh_behind_an_object_reads_its_grid_and_payload_from_the_objects_record() {
        let (m, _) = obj_pair();
        let k = &m.kernel_ll;
        assert!(
            k.contains("%m2v.o = extractelement <3 x i32> %m2v.tgp, i32 1"),
            "(j, o) come from the indirect dispatch"
        );
        assert!(k.contains("%m2v.r0 = mul i32 %m2v.o, 32"), "record stride");
        assert!(
            k.contains("%m2v.boff = select i1 %m2v.dead, i32 64, i32 %m2v.bo"),
            "a dead threadgroup writes the trash block"
        );
        assert!(
            k.contains(
                "  %1 = bitcast i8 addrspace(1)* %m2v.pla to %struct.Payload addrspace(1)*\n"
            ),
            "payload = record + 16: {}",
            k.lines()
                .filter(|l| l.contains("m2v.pl") || l.contains("@m_quad("))
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert!(k.contains("i32 29, i32 1, !\"air.read\""));
        assert!(
            k.contains("  %4 = extractelement <3 x i32> %m2v.ltgp, i32 0\n"),
            "{}",
            &k[..3000.min(k.len())]
        );
        assert!(!k.contains("addrspace(6)"));
    }

    #[test]
    fn the_linked_vertex_takes_the_object_from_the_instance_and_culls_past_the_grid() {
        let (m, _) = obj_pair();
        let v = &m.vertex_ll;
        assert!(
            v.contains("(i32 noundef %0, i32 noundef %1, i8 addrspace(1)* noundef readonly %2)")
        );
        assert!(v.contains("!\"air.instance_id\""));
        assert!(v.contains("%m.t0 = mul i32 %1, %m.mx"));
        assert!(v.contains("%m.live = and i1 %m.live0, %m.jok"));
        assert!(v.contains("!4 = !{i32 2, !\"air.buffer\", !\"air.location_index\", i32 30"));
    }

    #[test]
    fn an_indirect_mesh_reads_its_grid_from_record_zero_and_raises_the_draw_itself() {
        let m = lower_mesh_linked(M42, "m_cols", Link::Indirect, 16, 4096).expect("lowers");
        let k = &m.kernel_ll;
        assert!(k.contains(&format!(
            "%m2v.g.a = getelementptr inbounds i8, i8 addrspace(1)* %0, i32 {}",
            m.tr
        )));
        assert!(
            k.contains("%m2v.n0 = extractelement <3 x i32> %m2v.g, i32 0"),
            "the grid is the app's, not the push constant's"
        );
        assert!(k.contains("%m2v.dead = icmp uge i32 %m2v.lin, 4096"));
        assert!(
            k.contains("i32 %m2v.lp, i32 0, i32 2, i1 true)")
                && k.contains("i32 %m2v.vc, i32 0, i32 2, i1 true)")
        );
        assert!(k.contains("declare i32 @air.atomic.global.max.u.i32("));
        assert!(k.contains("extractelement <3 x i32> %m2v.g, i32 0\n"));
    }

    #[test]
    fn an_object_barrier_on_payload_memory_is_respelled_as_device_memory() {
        assert_eq!(
            respell_object_barrier("  tail call void @air.wg.barrier(i32 16, i32 1) #6").unwrap(),
            "  tail call void @air.wg.barrier(i32 1, i32 1) #6"
        );
        assert_eq!(
            respell_object_barrier("  call void @air.wg.barrier(i32 18, i32 1)").unwrap(),
            "  call void @air.wg.barrier(i32 3, i32 1)"
        );
        assert_eq!(
            respell_object_barrier("  call void @air.wg.barrier(i32 2, i32 1)").unwrap(),
            "  call void @air.wg.barrier(i32 2, i32 1)",
            "a threadgroup barrier is left alone"
        );
        assert!(
            respell_object_barrier("  call void @air.wg.barrier(i32 %7, i32 1)").is_err(),
            "non-constant flags must fail loud"
        );
    }

    #[test]
    fn an_object_that_never_sets_a_grid_is_refused_by_name() {
        let bad = OBJ.replacen("  tail call void @air.set_threadgroups_per_grid_mesh_properties(%struct._mesh_grid_properties_t addrspace(3)* nocapture %1, <3 x i32> <i32 2, i32 1, i32 1>) #5\n", "", 1);
        assert_ne!(bad, OBJ, "the anchor must exist");
        let (m, _) = obj_pair();
        let e = lower_object(&bad, "o_quads", &m.layout, m.tr, 32).expect_err("must refuse");
        assert!(e.contains("never calls set_threadgroups_per_grid"), "{e}");
    }

    #[test]
    fn a_linked_mesh_that_asks_for_its_grid_thread_position_is_refused() {
        let bad = OBJ.replace(
            "!\"air.thread_index_in_threadgroup\"",
            "!\"air.thread_position_in_grid\"",
        );
        let e = lower_mesh_linked(&bad, "m_quad", Link::Object, 32, 1024).expect_err("must refuse");
        assert!(e.contains("thread_position_in_grid"), "{e}");
    }

    #[test]
    fn a_function_that_is_not_a_mesh_is_refused_by_name() {
        let e = lower_mesh(M42, "f_mesh").expect_err("must refuse");
        assert!(e.contains("f_mesh"), "{e}");
    }

    #[test]
    fn an_unknown_mesh_intrinsic_is_refused_by_name() {
        let bad = M42.replacen(
            "@air.set_primitive_count_mesh(",
            "@air.set_primitive_culled_mesh(",
            1,
        );
        let e = lower_mesh(&bad, "m_cols").expect_err("must refuse");
        assert!(e.contains("set_primitive_culled_mesh"), "{e}");
    }

    #[test]
    fn an_unlabelled_entry_keeps_its_implicit_number_so_a_phi_can_still_name_it() {
        let m = lower_mesh(M42, "m_cols").unwrap();
        assert!(
            m.kernel_ll.contains(" {\n5:\n"),
            "mesh kernel entry label not pinned"
        );
        let (_, o) = obj_pair();
        assert!(
            o.contains(" {\n4:\n"),
            "object kernel entry label not pinned"
        );
        assert_eq!(
            implicit_entry_label(&[
                "i32 %0".into(),
                "i32 noundef %1".into(),
                "<3 x i32> noundef %m2v.tgp".into()
            ]),
            2
        );
    }

    #[test]
    fn the_prologue_lands_after_a_labelled_entry_block_never_above_it() {
        let def = M42
            .lines()
            .find(|l| l.starts_with("define void @m_cols("))
            .unwrap();
        let labelled = M42.replacen(&format!("{def}\n"), &format!("{def}\nentry:\n"), 1);
        let m = lower_mesh(&labelled, "m_cols").unwrap();
        assert!(
            m.kernel_ll
                .contains("%m2v.ntg) local_unnamed_addr #0 {\nentry:\n  %4 = extractelement"),
            "{}",
            &m.kernel_ll[..1200.min(m.kernel_ll.len())]
        );
        let plain = lower_mesh(M42, "m_cols").unwrap();
        assert!(
            plain
                .kernel_ll
                .contains("%m2v.ntg) local_unnamed_addr #0 {\n5:\n  %4 = extractelement"),
            "an unlabelled entry keeps the prologue first, under its pinned number"
        );
    }

    #[test]
    fn an_opaque_pointer_module_lowers_to_opaque_stores() {
        let op = M42.replace("%struct._mesh_t addrspace(7)*", "ptr addrspace(7)");
        let m = lower_mesh(&op, "m_cols").expect("lowers");
        assert!(
            m.kernel_ll
                .contains("store <4 x float> %18, ptr addrspace(1) %m2v.ma"),
            "{}",
            m.kernel_ll
        );
        assert!(!m.kernel_ll.contains("ptr addrspace(7)"));
    }
}
