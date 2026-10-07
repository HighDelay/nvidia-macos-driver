use crate::{meta, tools};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Clone, Debug)]
struct PassTy {
    spirv: String,
    base: &'static str,
    lanes: u32,
}

fn passthrough_type(air: &str) -> Result<PassTy, String> {
    let air = air.trim();
    let (base, rest) = if let Some(rest) = air.strip_prefix("float") {
        ("float", rest)
    } else if let Some(rest) = air.strip_prefix("half") {
        ("half", rest)
    } else if let Some(rest) = air.strip_prefix("uint") {
        ("uint", rest)
    } else if let Some(rest) = air.strip_prefix("int") {
        ("int", rest)
    } else if let Some(rest) = air.strip_prefix("ushort") {
        ("ushort", rest)
    } else if let Some(rest) = air.strip_prefix("short") {
        ("short", rest)
    } else if air == "bool" {
        ("bool", "")
    } else {
        return Err(format!("passthrough: unsupported varying type {air}"));
    };
    let lanes = if rest.is_empty() {
        1
    } else {
        rest.parse::<u32>()
            .map_err(|_| format!("passthrough: unsupported varying type {air}"))?
    };
    if !(1..=4).contains(&lanes) {
        return Err(format!("passthrough: unsupported varying type {air}"));
    }
    let spirv = if base == "bool" {
        "uint".to_string()
    } else if lanes == 1 {
        base.to_string()
    } else {
        format!("v{lanes}{base}")
    };
    Ok(PassTy { spirv, base, lanes })
}

impl PassTy {
    fn needs_flat(&self) -> bool {
        matches!(self.base, "bool" | "int" | "uint" | "short" | "ushort")
    }

    fn is_int16(&self) -> bool {
        matches!(self.base, "short" | "ushort")
    }
}

fn emit_float_value(
    p: &mut Vec<String>,
    prefix: &str,
    idx: usize,
    lanes: u32,
    distinct_float3: bool,
) -> Result<String, String> {
    match lanes {
        1 => Ok("%float_0_25".to_string()),
        2 => {
            let id = format!("%{prefix}{idx}");
            p.push(format!("{id} = OpCompositeConstruct %v2float %ux %uy"));
            Ok(id)
        }
        3 => {
            let id = format!("%{prefix}{idx}");
            let z = if distinct_float3 && idx > 0 {
                "%float_1"
            } else {
                "%float_0_5"
            };
            p.push(format!("{id} = OpCompositeConstruct %v3float %ux %uy {z}"));
            Ok(id)
        }
        4 => {
            let id = format!("%{prefix}{idx}");
            p.push(format!(
                "{id} = OpCompositeConstruct %v4float %ux %uy %float_0_5 %float_1"
            ));
            Ok(id)
        }
        _ => Err(format!("passthrough: unsupported lane count {lanes}")),
    }
}

fn emit_integer_value(p: &mut Vec<String>, prefix: &str, idx: usize, ty: &PassTy) -> String {
    let scalar_ids = match ty.base {
        "int" => ["%int_1", "%int_2", "%int_3", "%int_4"],
        "uint" => ["%uint_1", "%uint_2", "%uint_3", "%uint_4"],
        "short" => ["%short_1", "%short_2", "%short_3", "%short_4"],
        "ushort" => ["%ushort_1", "%ushort_2", "%ushort_3", "%ushort_4"],
        _ => unreachable!("integer emitter called for {}", ty.base),
    };
    if ty.lanes == 1 {
        return scalar_ids[0].to_string();
    }
    let id = format!("%{prefix}{idx}");
    let operands = scalar_ids[..ty.lanes as usize].join(" ");
    p.push(format!(
        "{id} = OpCompositeConstruct %{} {operands}",
        ty.spirv
    ));
    id
}

fn passthrough_vertex_spvasm(
    meta: &meta::FragMeta,
    distinct_float3_inputs: bool,
) -> Result<String, String> {
    let has_viewport_index = meta
        .roles
        .iter()
        .any(|(_, role)| matches!(role, meta::FragRole::ViewportArrayIndex));
    let mut locs: Vec<u32> = meta
        .roles
        .iter()
        .filter_map(|(_, role)| match role {
            meta::FragRole::Varying(loc) => Some(*loc),
            _ => None,
        })
        .collect();
    locs.sort_unstable();
    locs.dedup();

    let mut varyings = Vec::with_capacity(locs.len());
    for loc in locs {
        let ty_name = meta
            .varying_type(loc)
            .ok_or_else(|| format!("passthrough: missing type for varying location {loc}"))?;
        varyings.push((loc, passthrough_type(ty_name)?));
    }

    let mut out_types: BTreeSet<String> = BTreeSet::new();
    out_types.insert("v4float".to_string());
    if has_viewport_index {
        out_types.insert("uint".to_string());
    }
    for (_, ty) in &varyings {
        out_types.insert(ty.spirv.clone());
    }
    let has_half = varyings.iter().any(|(_, ty)| ty.base == "half");
    let has_int16 = varyings.iter().any(|(_, ty)| ty.is_int16());

    let mut p: Vec<String> = vec![];
    if has_half {
        p.push("OpCapability Float16".into());
    }
    if has_int16 {
        p.push("OpCapability Int16".into());
    }
    if has_viewport_index {
        p.push("OpCapability ShaderViewportIndex".into());
    }
    p.push("OpCapability Shader".into());
    p.push("OpMemoryModel Logical GLSL450".into());

    let mut iface = vec!["%glpos".to_string(), "%vidx".to_string()];
    if has_viewport_index {
        iface.push("%viewport".to_string());
    }
    for i in 0..varyings.len() {
        iface.push(format!("%vout{i}"));
    }
    p.push(format!(
        "OpEntryPoint Vertex %main \"main\" {}",
        iface.join(" ")
    ));
    p.push("OpDecorate %glpos BuiltIn Position".into());
    p.push("OpDecorate %vidx BuiltIn VertexIndex".into());
    if has_viewport_index {
        p.push("OpDecorate %viewport BuiltIn ViewportIndex".into());
    }
    for (i, (loc, ty)) in varyings.iter().enumerate() {
        p.push(format!("OpDecorate %vout{i} Location {loc}"));
        if ty.needs_flat() || meta.varying_is_flat(*loc) {
            p.push(format!("OpDecorate %vout{i} Flat"));
        }
    }

    p.push("%void = OpTypeVoid".into());
    p.push("%fnty = OpTypeFunction %void".into());
    p.push("%float = OpTypeFloat 32".into());
    if has_half {
        p.push("%half = OpTypeFloat 16".into());
    }
    p.push("%int = OpTypeInt 32 1".into());
    p.push("%uint = OpTypeInt 32 0".into());
    if has_int16 {
        p.push("%short = OpTypeInt 16 1".into());
        p.push("%ushort = OpTypeInt 16 0".into());
    }
    p.push("%v2float = OpTypeVector %float 2".into());
    p.push("%v3float = OpTypeVector %float 3".into());
    p.push("%v4float = OpTypeVector %float 4".into());
    if has_half {
        p.push("%v2half = OpTypeVector %half 2".into());
        p.push("%v3half = OpTypeVector %half 3".into());
        p.push("%v4half = OpTypeVector %half 4".into());
    }
    p.push("%v2int = OpTypeVector %int 2".into());
    p.push("%v3int = OpTypeVector %int 3".into());
    p.push("%v4int = OpTypeVector %int 4".into());
    p.push("%v2uint = OpTypeVector %uint 2".into());
    p.push("%v3uint = OpTypeVector %uint 3".into());
    p.push("%v4uint = OpTypeVector %uint 4".into());
    if has_int16 {
        p.push("%v2short = OpTypeVector %short 2".into());
        p.push("%v3short = OpTypeVector %short 3".into());
        p.push("%v4short = OpTypeVector %short 4".into());
        p.push("%v2ushort = OpTypeVector %ushort 2".into());
        p.push("%v3ushort = OpTypeVector %ushort 3".into());
        p.push("%v4ushort = OpTypeVector %ushort 4".into());
    }
    p.push("%_ptr_Input_int = OpTypePointer Input %int".into());
    for ty in &out_types {
        p.push(format!("%_ptr_Output_{ty} = OpTypePointer Output %{ty}"));
    }
    p.push("%float_0 = OpConstant %float 0".into());
    p.push("%float_0_25 = OpConstant %float 0.25".into());
    p.push("%float_0_5 = OpConstant %float 0.5".into());
    p.push("%float_1 = OpConstant %float 1".into());
    p.push("%float_2 = OpConstant %float 2".into());
    p.push("%float_4 = OpConstant %float 4".into());
    p.push("%int_1 = OpConstant %int 1".into());
    p.push("%int_2 = OpConstant %int 2".into());
    p.push("%int_3 = OpConstant %int 3".into());
    p.push("%int_4 = OpConstant %int 4".into());
    p.push("%uint_0 = OpConstant %uint 0".into());
    p.push("%uint_1 = OpConstant %uint 1".into());
    p.push("%uint_2 = OpConstant %uint 2".into());
    p.push("%uint_3 = OpConstant %uint 3".into());
    p.push("%uint_4 = OpConstant %uint 4".into());
    if has_int16 {
        p.push("%short_1 = OpConstant %short 1".into());
        p.push("%short_2 = OpConstant %short 2".into());
        p.push("%short_3 = OpConstant %short 3".into());
        p.push("%short_4 = OpConstant %short 4".into());
        p.push("%ushort_1 = OpConstant %ushort 1".into());
        p.push("%ushort_2 = OpConstant %ushort 2".into());
        p.push("%ushort_3 = OpConstant %ushort 3".into());
        p.push("%ushort_4 = OpConstant %ushort 4".into());
    }
    p.push("%glpos = OpVariable %_ptr_Output_v4float Output".into());
    p.push("%vidx = OpVariable %_ptr_Input_int Input".into());
    if has_viewport_index {
        p.push("%viewport = OpVariable %_ptr_Output_uint Output".into());
    }
    for (i, (_, ty)) in varyings.iter().enumerate() {
        p.push(format!(
            "%vout{i} = OpVariable %_ptr_Output_{} Output",
            ty.spirv
        ));
    }

    p.push("%main = OpFunction %void None %fnty".into());
    p.push("%entry = OpLabel".into());
    p.push("%idx = OpLoad %int %vidx".into());
    p.push("%ax_i = OpBitwiseAnd %int %idx %int_1".into());
    p.push("%ay_i = OpShiftRightArithmetic %int %idx %int_1".into());
    p.push("%axf = OpConvertSToF %float %ax_i".into());
    p.push("%ayf = OpConvertSToF %float %ay_i".into());
    p.push("%px0 = OpFMul %float %axf %float_4".into());
    p.push("%px = OpFSub %float %px0 %float_1".into());
    p.push("%py0 = OpFMul %float %ayf %float_4".into());
    p.push("%py = OpFSub %float %float_1 %py0".into());
    p.push("%pos = OpCompositeConstruct %v4float %px %py %float_0 %float_1".into());
    p.push("OpStore %glpos %pos".into());
    if has_viewport_index {
        p.push("OpStore %viewport %uint_0".into());
    }
    p.push("%ux = OpFMul %float %axf %float_2".into());
    p.push("%uy = OpFMul %float %ayf %float_2".into());

    for (i, (_, ty)) in varyings.iter().enumerate() {
        let val = match ty.base {
            "float" => emit_float_value(&mut p, "uvf", i, ty.lanes, distinct_float3_inputs)?,
            "half" => {
                let fval = emit_float_value(&mut p, "uvf", i, ty.lanes, distinct_float3_inputs)?;
                let hval = format!("%uvh{i}");
                p.push(format!("{hval} = OpFConvert %{} {fval}", ty.spirv));
                hval
            }
            "bool" => "%uint_1".to_string(),
            "int" | "uint" | "short" | "ushort" => emit_integer_value(&mut p, "uvi", i, ty),
            _ => unreachable!("unsupported passthrough base {}", ty.base),
        };
        p.push(format!("OpStore %vout{i} {val}"));
    }
    p.push("OpReturn".into());
    p.push("OpFunctionEnd".into());
    Ok(p.join("\n") + "\n")
}

fn assemble_spvasm(asm: &str, tmp: &Path, stem: &str) -> Result<Vec<u8>, String> {
    let asmf = tmp.join(format!("{stem}.spvasm"));
    let spvf = tmp.join(format!("{stem}.spv"));
    std::fs::write(&asmf, asm).map_err(|e| format!("write {}: {e}", asmf.display()))?;
    let asmf_s = asmf.to_str().ok_or("passthrough: bad asm path")?;
    let spvf_s = spvf.to_str().ok_or("passthrough: bad spv path")?;
    let assembled = tools::run(
        "spirv-as",
        &[
            "--target-env",
            tools::VULKAN_TARGET_ENV,
            asmf_s,
            "-o",
            spvf_s,
        ],
    );
    let bytes = match assembled {
        Ok(_) => std::fs::read(&spvf).map_err(|e| format!("read {}: {e}", spvf.display())),
        Err(e) => Err(e),
    };
    let _ = std::fs::remove_file(&asmf);
    let _ = std::fs::remove_file(&spvf);
    bytes
}

fn passthrough_sanitized_ll(src: &str, tmp: &Path) -> Result<String, String> {
    tools::air_to_sanitized_ll(src, tmp)
}

pub fn translate_passthrough(src: &str, tmp: &Path) -> Result<Vec<u8>, String> {
    let san_ll = passthrough_sanitized_ll(src, tmp)?;
    translate_passthrough_sanitized(&san_ll, tmp)
}

pub fn translate_passthrough_specialized(
    src: &str,
    tmp: &Path,
    function_constants: &[(u32, Vec<u8>)],
) -> Result<Vec<u8>, String> {
    let san_ll = passthrough_sanitized_ll(src, tmp)?;
    let specialized =
        crate::fc_air_specialize::specialize_air_function_constants(&san_ll, function_constants)?;
    translate_passthrough_sanitized(specialized.as_ref(), tmp)
}

fn translate_passthrough_sanitized(san_ll: &str, tmp: &Path) -> Result<Vec<u8>, String> {
    let frag = meta::parse_air_fragment_meta(san_ll)
        .ok_or_else(|| "passthrough: source has no !air.fragment metadata".to_string())?;
    let asm = passthrough_vertex_spvasm(&frag, fragment_requires_distinct_float3_inputs(san_ll))?;
    let spv = assemble_spvasm(&asm, tmp, "passthrough")?;
    tools::spirv_val_bytes(&spv, tmp)?;
    Ok(spv)
}

fn vertex_observer_fragment_spvasm(
    meta: &meta::VertMeta,
    varying_location: Option<u32>,
) -> Result<String, String> {
    let varying = varying_location
        .map(|location| -> Result<(u32, PassTy), String> {
            let name = meta.output_varying_type(location).ok_or_else(|| {
                format!("vertex observer: missing type for varying location {location}")
            })?;
            Ok((location, passthrough_type(name)?))
        })
        .transpose()?;
    let ty = varying
        .as_ref()
        .map(|(_, ty)| ty.clone())
        .unwrap_or(PassTy {
            spirv: "v4float".into(),
            base: "float",
            lanes: 4,
        });
    let output_base = match ty.base {
        "float" | "half" => "float",
        "int" | "short" => "int",
        "uint" | "ushort" | "bool" => "uint",
        _ => {
            return Err(format!(
                "vertex observer: unsupported type base {}",
                ty.base
            ))
        }
    };
    let output_ty = format!("v4{output_base}");
    let mut p = Vec::new();
    if ty.base == "half" {
        p.push("OpCapability Float16".into());
    }
    if ty.is_int16() {
        p.push("OpCapability Int16".into());
    }
    p.push("OpCapability Shader".into());
    p.push("OpMemoryModel Logical GLSL450".into());
    p.push("OpEntryPoint Fragment %main \"main\" %vin %color".into());
    p.push("OpExecutionMode %main OriginUpperLeft".into());
    match varying.as_ref() {
        Some((location, ty)) => {
            p.push(format!("OpDecorate %vin Location {location}"));
            if ty.needs_flat() {
                p.push("OpDecorate %vin Flat".into());
            }
        }
        None => p.push("OpDecorate %vin BuiltIn FragCoord".into()),
    }
    p.push("OpDecorate %color Location 0".into());
    p.push("%void = OpTypeVoid".into());
    p.push("%fnty = OpTypeFunction %void".into());
    p.push("%float = OpTypeFloat 32".into());
    p.push("%int = OpTypeInt 32 1".into());
    p.push("%uint = OpTypeInt 32 0".into());
    if ty.base == "half" {
        p.push("%half = OpTypeFloat 16".into());
    }
    if ty.is_int16() {
        p.push("%short = OpTypeInt 16 1".into());
        p.push("%ushort = OpTypeInt 16 0".into());
    }
    for lanes in 2..=4 {
        p.push(format!("%v{lanes}float = OpTypeVector %float {lanes}"));
        p.push(format!("%v{lanes}int = OpTypeVector %int {lanes}"));
        p.push(format!("%v{lanes}uint = OpTypeVector %uint {lanes}"));
        if ty.base == "half" {
            p.push(format!("%v{lanes}half = OpTypeVector %half {lanes}"));
        }
        if ty.is_int16() {
            p.push(format!("%v{lanes}short = OpTypeVector %short {lanes}"));
            p.push(format!("%v{lanes}ushort = OpTypeVector %ushort {lanes}"));
        }
    }
    p.push(format!(
        "%_ptr_Input_value = OpTypePointer Input %{}",
        ty.spirv
    ));
    p.push(format!(
        "%_ptr_Output_color = OpTypePointer Output %{output_ty}"
    ));
    p.push("%float_0 = OpConstant %float 0".into());
    p.push("%float_1 = OpConstant %float 1".into());
    p.push("%int_0 = OpConstant %int 0".into());
    p.push("%int_1 = OpConstant %int 1".into());
    p.push("%uint_0 = OpConstant %uint 0".into());
    p.push("%uint_1 = OpConstant %uint 1".into());
    p.push("%vin = OpVariable %_ptr_Input_value Input".into());
    p.push("%color = OpVariable %_ptr_Output_color Output".into());
    p.push("%main = OpFunction %void None %fnty".into());
    p.push("%entry = OpLabel".into());
    p.push(format!("%loaded = OpLoad %{} %vin", ty.spirv));
    let converted = match ty.base {
        "half" => {
            let destination = if ty.lanes == 1 {
                "%float".to_string()
            } else {
                format!("%v{}float", ty.lanes)
            };
            p.push(format!("%converted = OpFConvert {destination} %loaded"));
            "%converted"
        }
        "short" | "ushort" => {
            let destination = if ty.lanes == 1 {
                format!("%{output_base}")
            } else {
                format!("%v{}{output_base}", ty.lanes)
            };
            let opcode = if ty.base == "short" {
                "OpSConvert"
            } else {
                "OpUConvert"
            };
            p.push(format!("%converted = {opcode} {destination} %loaded"));
            "%converted"
        }
        _ => "%loaded",
    };
    if ty.lanes == 4 && matches!(ty.base, "float" | "int" | "uint" | "bool") {
        p.push(format!("OpStore %color {converted}"));
    } else {
        let scalar_ty = format!("%{output_base}");
        let mut components = Vec::new();
        for lane in 0..ty.lanes {
            if ty.lanes == 1 {
                components.push(converted.to_string());
            } else {
                let id = format!("%component{lane}");
                p.push(format!(
                    "{id} = OpCompositeExtract {scalar_ty} {converted} {lane}"
                ));
                components.push(id);
            }
        }
        let zero = format!("%{output_base}_0");
        let one = format!("%{output_base}_1");
        while components.len() < 3 {
            components.push(zero.clone());
        }
        while components.len() < 4 {
            components.push(one.clone());
        }
        p.push(format!(
            "%observed = OpCompositeConstruct %{output_ty} {}",
            components.join(" ")
        ));
        p.push("OpStore %color %observed".into());
    }
    p.push("OpReturn".into());
    p.push("OpFunctionEnd".into());
    Ok(p.join("\n") + "\n")
}

pub fn translate_vertex_observer(
    src: &str,
    varying_location: Option<u32>,
    tmp: &Path,
) -> Result<Vec<u8>, String> {
    let san_ll = passthrough_sanitized_ll(src, tmp)?;
    let vert = meta::parse_air_vertex_meta(&san_ll)
        .ok_or_else(|| "vertex observer: source has no !air.vertex metadata".to_string())?;
    let asm = vertex_observer_fragment_spvasm(&vert, varying_location)?;
    assemble_spvasm(&asm, tmp, "vertex-observer")
}

fn fragment_requires_distinct_float3_inputs(ll: &str) -> bool {
    ll.lines()
        .filter(|line| {
            line.contains(r#""air.fragment_input""#)
                && line.contains("!\"air.arg_type_name\", !\"float3\"")
        })
        .count()
        >= 2
        && ll.contains("@air.fast_rsqrt.f32")
        && ll
            .lines()
            .any(|line| line.trim_start().contains(" = fsub ") && line.contains("<3 x float>"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_vertex_flips_clip_y_for_vulkan_viewport() {
        let mut frag = meta::FragMeta::default();
        frag.roles.push((1, meta::FragRole::Varying(0)));
        frag.varying_types.insert(0, "float2".to_string());

        let asm = passthrough_vertex_spvasm(&frag, false).unwrap();
        assert!(asm.contains("%px = OpFSub %float %px0 %float_1"));
        assert!(asm.contains("%py = OpFSub %float %float_1 %py0"));
    }

    #[test]
    fn passthrough_vertex_matches_metal_generated_float4_value() {
        let mut frag = meta::FragMeta::default();
        frag.roles.push((1, meta::FragRole::Varying(0)));
        frag.varying_types.insert(0, "float4".to_string());

        let asm = passthrough_vertex_spvasm(&frag, false).unwrap();
        assert!(asm.contains("%float_0_5 = OpConstant %float 0.5"));
        assert!(asm.contains("%uvf0 = OpCompositeConstruct %v4float %ux %uy %float_0_5 %float_1"));
    }

    #[test]
    fn specialized_passthrough_omits_disabled_fragment_inputs() {
        let ll = r#"@state.MTL_FC_INIT_0_b = internal addrspace(2) externally_initialized constant i8 undef, section "air.fc_initializer", align 1
@enabled = internal addrspace(2) global i8 0, align 1
define internal void @_GLOBAL__sub_I_metadata() section "air.static_init" {
  %state = load i8, ptr addrspace(2) @state.MTL_FC_INIT_0_b
  store i8 %state, ptr addrspace(2) @enabled
  ret void
}
define <4 x half> @frag(float %conditional, float %always) { ret <4 x half> zeroinitializer }
!air.fragment = !{!0}
!0 = !{ptr @frag, !1, !2}
!1 = !{!3}
!2 = !{!4, !5}
!3 = !{i32 0, !"air.render_target", i32 0, !"air.arg_type_name", !"half4"}
!4 = !{i32 0, !"air.function_constant", !6, !"air.fragment_input", !"air.arg_type_name", !"float"}
!5 = !{i32 1, !"air.fragment_input", !"air.arg_type_name", !"float"}
!6 = !{ptr addrspace(2) @enabled, !"bool", !"enabled"}
"#;
        let specialized =
            crate::fc_air_specialize::specialize_air_function_constants(ll, &[(0, vec![0])])
                .unwrap();
        let frag = meta::parse_air_fragment_meta(specialized.as_ref()).unwrap();
        let asm = passthrough_vertex_spvasm(&frag, false).unwrap();

        assert_eq!(
            frag.roles
                .iter()
                .filter(|(_, role)| matches!(role, meta::FragRole::Varying(_)))
                .count(),
            1
        );
        assert!(asm.contains("OpDecorate %vout0 Location 0"), "{asm}");
        assert!(!asm.contains("%vout1"), "{asm}");
    }

    #[test]
    fn passthrough_vertex_separates_duplicate_float3_rsqrt_inputs() {
        let ll = r#"
define <4 x half> @frag(<3 x float> %a, <3 x float> %b) {
  %delta = fsub fast <3 x float> %a, %b
  %len2 = tail call fast float @air.dot.v3f32(<3 x float> %delta, <3 x float> %delta)
  %inv = tail call fast float @air.fast_rsqrt.f32(float %len2)
  ret <4 x half> zeroinitializer
}
declare float @air.fast_rsqrt.f32(float)
!1 = !{i32 0, !"air.fragment_input", !"generated(a)", !"air.arg_type_name", !"float3", !"air.arg_name", !"a"}
!2 = !{i32 1, !"air.fragment_input", !"generated(b)", !"air.arg_type_name", !"float3", !"air.arg_name", !"b"}
"#;
        assert!(fragment_requires_distinct_float3_inputs(ll));

        let mut frag = meta::FragMeta::default();
        frag.roles.push((0, meta::FragRole::Varying(0)));
        frag.roles.push((1, meta::FragRole::Varying(1)));
        frag.varying_types.insert(0, "float3".to_string());
        frag.varying_types.insert(1, "float3".to_string());
        let asm = passthrough_vertex_spvasm(&frag, true).unwrap();
        assert!(asm.contains("%uvf0 = OpCompositeConstruct %v3float %ux %uy %float_0_5"));
        assert!(asm.contains("%uvf1 = OpCompositeConstruct %v3float %ux %uy %float_1"));
    }

    #[test]
    fn passthrough_vertex_preserves_air_flat_float_varying() {
        let mut frag = meta::FragMeta::default();
        frag.roles.push((1, meta::FragRole::Varying(0)));
        frag.varying_types.insert(0, "float4".to_string());
        frag.varying_interpolation.insert(
            0,
            crate::meta::VaryingInterpolation {
                flat: true,
                ..Default::default()
            },
        );

        let asm = passthrough_vertex_spvasm(&frag, false).unwrap();
        assert!(asm.contains("OpDecorate %vout0 Flat"), "{asm}");
    }

    #[test]
    fn passthrough_vertex_supports_flat_integer_varyings() {
        let mut frag = meta::FragMeta::default();
        frag.roles.push((1, meta::FragRole::Varying(0)));
        frag.roles.push((2, meta::FragRole::Varying(1)));
        frag.roles.push((3, meta::FragRole::Varying(2)));
        frag.roles.push((4, meta::FragRole::Varying(3)));
        frag.varying_types.insert(0, "uint2".to_string());
        frag.varying_types.insert(1, "int".to_string());
        frag.varying_types.insert(2, "short3".to_string());
        frag.varying_types.insert(3, "ushort".to_string());

        let asm = passthrough_vertex_spvasm(&frag, false).unwrap();
        assert!(asm.contains("OpCapability Int16"), "{asm}");
        assert!(asm.contains("%int = OpTypeInt 32 1"));
        assert!(asm.contains("%uint = OpTypeInt 32 0"));
        assert!(asm.contains("%short = OpTypeInt 16 1"));
        assert!(asm.contains("%ushort = OpTypeInt 16 0"));
        assert!(asm.contains("%v2uint = OpTypeVector %uint 2"));
        assert!(asm.contains("%v3short = OpTypeVector %short 3"));
        assert!(asm.contains("OpDecorate %vout0 Flat"));
        assert!(asm.contains("OpDecorate %vout1 Flat"));
        assert!(asm.contains("OpDecorate %vout2 Flat"));
        assert!(asm.contains("OpDecorate %vout3 Flat"));
        assert!(asm.contains("OpStore %vout1 %int_1"));
        assert!(asm.contains("%uvi2 = OpCompositeConstruct %v3short %short_1 %short_2 %short_3"));
        assert!(asm.contains("OpStore %vout3 %ushort_1"));
    }

    #[test]
    fn passthrough_vertex_maps_scalar_bool_varying_to_flat_uint() {
        let mut frag = meta::FragMeta::default();
        frag.roles.push((1, meta::FragRole::Varying(0)));
        frag.varying_types.insert(0, "bool".to_string());

        let asm = passthrough_vertex_spvasm(&frag, false).unwrap();
        assert!(asm.contains("OpDecorate %vout0 Flat"), "{asm}");
        assert!(asm.contains("%vout0 = OpVariable %_ptr_Output_uint Output"));
        assert!(asm.contains("OpStore %vout0 %uint_1"));
        assert!(!asm.contains("OpTypeBool"), "{asm}");
    }

    #[test]
    fn passthrough_vertex_defines_requested_viewport_index() {
        let mut frag = meta::FragMeta::default();
        frag.roles.push((1, meta::FragRole::ViewportArrayIndex));

        let asm = passthrough_vertex_spvasm(&frag, false).unwrap();
        assert!(asm.contains("OpCapability ShaderViewportIndex"), "{asm}");
        assert!(
            asm.contains("OpDecorate %viewport BuiltIn ViewportIndex"),
            "{asm}"
        );
        assert!(
            asm.contains("%viewport = OpVariable %_ptr_Output_uint Output"),
            "{asm}"
        );
        assert!(asm.contains("OpStore %viewport %uint_0"), "{asm}");
    }

    #[test]
    fn passthrough_vertex_uses_feature_free_implicit_layer_zero() {
        let mut frag = meta::FragMeta::default();
        frag.roles.push((1, meta::FragRole::RenderTargetArrayIndex));

        let asm = passthrough_vertex_spvasm(&frag, false).unwrap();
        assert!(!asm.contains("ShaderLayer"), "{asm}");
        assert!(!asm.contains("BuiltIn Layer"), "{asm}");
    }

    #[test]
    fn vertex_observer_pads_float2_into_rgba32_float() {
        let mut vert = meta::VertMeta::default();
        vert.output_roles.push(meta::VertOutRole::Varying(3));
        vert.output_varying_types.insert(3, "float2".into());
        let asm = vertex_observer_fragment_spvasm(&vert, Some(3)).unwrap();
        assert!(asm.contains("OpDecorate %vin Location 3"), "{asm}");
        assert!(asm.contains(
            "%observed = OpCompositeConstruct %v4float %component0 %component1 %float_0 %float_1"
        ));
    }

    #[test]
    fn vertex_observer_uses_flat_integer_input_and_uint_attachment() {
        let mut vert = meta::VertMeta::default();
        vert.output_roles.push(meta::VertOutRole::Varying(1));
        vert.output_varying_types.insert(1, "ushort".into());
        let asm = vertex_observer_fragment_spvasm(&vert, Some(1)).unwrap();
        assert!(asm.contains("OpCapability Int16"), "{asm}");
        assert!(asm.contains("OpDecorate %vin Flat"), "{asm}");
        assert!(
            asm.contains("%converted = OpUConvert %uint %loaded"),
            "{asm}"
        );
        assert!(asm.contains("%_ptr_Output_color = OpTypePointer Output %v4uint"));
    }
}
