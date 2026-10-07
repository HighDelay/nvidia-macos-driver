use metal2vulkan::passes::{Stage, TransformOptions};
use metal2vulkan::{disassemble, translate_sanitized_native, translate_sanitized_native_reflected};
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "m2v_selected_struct_gep_off_optional_resource_{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn source(field: u32) -> String {
    format!(
        r#"target triple = "air64_v28-apple-macosx26.5.0"

%struct.GammaLUTs = type {{ [4 x half], [4 x half] }}

define void @k(ptr addrspace(1) %luts, ptr addrspace(1) %out, i32 %gid) {{
entry:
  %has_luts = icmp eq i32 %gid, 0
  %p = select i1 %has_luts, ptr addrspace(1) null, ptr addrspace(1) %luts
  %idx = zext i32 %gid to i64
  %elem = getelementptr inbounds %struct.GammaLUTs, ptr addrspace(1) %p, i64 0, i32 {field}, i64 %idx
  br i1 %has_luts, label %miss, label %hit
hit:
  %v = load half, ptr addrspace(1) %elem, align 2
  %vf = fpext half %v to float
  br label %join
miss:
  br label %join
join:
  %result = phi float [ %vf, %hit ], [ 0.0, %miss ]
  %outp = getelementptr inbounds float, ptr addrspace(1) %out, i64 %idx
  store float %result, ptr addrspace(1) %outp, align 4
  ret void
}}

!air.kernel = !{{!0}}
!0 = !{{ptr @k, !1, !2}}
!1 = !{{}}
!2 = !{{!3, !4, !5}}
!3 = !{{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"struct GammaLUTs", !"air.arg_name", !"luts"}}
!4 = !{{i32 1, !"air.buffer", !"air.location_index", i32 1, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"float", !"air.arg_name", !"out"}}
!5 = !{{i32 2, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"gid"}}
"#
    )
}

fn assert_guarded_half_address(asm: &str, byte_offset: u32) {
    let def = |id: &str| {
        asm.lines()
            .find_map(|line| {
                let (result, rest) = line.trim().split_once(" = ")?;
                (result == id).then_some(rest)
            })
            .expect("SSA definition")
    };
    let constant = |id: &str, value: u32| {
        let parts: Vec<_> = def(id).split_whitespace().collect();
        parts.first() == Some(&"OpConstant") && parts.last() == Some(&value.to_string().as_str())
    };
    let resource = asm
        .lines()
        .find_map(|line| {
            let p: Vec<_> = line.split_whitespace().collect();
            (p.len() == 4 && p[0] == "OpDecorate" && p[2] == "Binding" && p[3] == "0").then(|| p[1])
        })
        .expect("input binding zero");
    let chain = asm
        .lines()
        .find_map(|line| {
            let (id, rest) = line.trim().split_once(" = ")?;
            let p: Vec<_> = rest.split_whitespace().collect();
            (p.first() == Some(&"OpInBoundsAccessChain") && p.get(2) == Some(&resource))
                .then_some((id, p))
        })
        .expect("input access chain");
    let divide: Vec<_> = def(chain.1[4]).split_whitespace().collect();
    assert_eq!(divide[0], "OpUDiv", "{asm}");
    assert!(constant(divide[3], 4), "raw word stride: {asm}");
    let add: Vec<_> = def(divide[2]).split_whitespace().collect();
    assert_eq!(add[0], "OpIAdd", "{asm}");
    assert!(constant(add[2], byte_offset), "field byte offset: {asm}");
    let multiply: Vec<_> = def(add[3]).split_whitespace().collect();
    assert_eq!(multiply[0], "OpIMul", "{asm}");
    assert!(constant(multiply[3], 2), "half byte stride: {asm}");
    let chain_pos = asm.find(&format!("{} = ", chain.0)).unwrap();
    let prefix = &asm[..chain_pos];
    assert!(
        prefix.contains("OpBranchConditional"),
        "guard before resource access: {asm}"
    );
    assert!(
        prefix.rfind("OpLabel").unwrap() > prefix.rfind("OpBranchConditional").unwrap(),
        "resource is in guarded successor: {asm}"
    );
    assert!(
        asm.contains("OpPhi") && asm.contains("OpVectorExtractDynamic"),
        "guarded half extraction: {asm}"
    );
}

#[test]
fn fixed_frag_lph_cph_shape_null_vs_real_struct_select_then_array_gep_field1() {
    let ll = source(1);
    let spv = translate_sanitized_native(&ll, Stage::Kernel, &tmp())
        .expect("a null-vs-real select into a struct buffer must translate, not hit the access-chain contract");
    let asm = disassemble(&spv).expect("disassemble");
    assert_guarded_half_address(&asm, 8);
}

#[test]
fn fixed_frag_lph_cpf_shape_null_vs_real_struct_select_then_array_gep_field0() {
    let ll = source(0);
    let spv = translate_sanitized_native(&ll, Stage::Kernel, &tmp())
        .expect("a null-vs-real select into a struct buffer must translate, not hit the access-chain contract");
    let asm = disassemble(&spv).expect("disassemble");
    assert_guarded_half_address(&asm, 0);
}

#[test]
fn the_selected_resource_is_still_reflected_as_a_bound_buffer() {
    let ll = source(1);
    let (_, reflection) = translate_sanitized_native_reflected(
        &ll,
        Stage::Kernel,
        &tmp(),
        TransformOptions::default(),
    )
    .expect("translate+reflect");
    assert!(
        reflection.bindings.iter().any(|b| b.metal_index == 0
            && b.param_index == Some(0)
            && b.descriptor.as_ref().is_some_and(|d| d.binding == 0)),
        "expected at least one bound resource in the reflection, got none: {:?}",
        reflection.bindings
    );
}

#[test]
fn swapped_nullable_arms_preserve_padded_member_offsets() {
    for field in 0..2 {
        let ll = source(field + 1)
            .replace(
                "[4 x half], [4 x half]",
                "[3 x i32], [4 x half], [4 x half]",
            )
            .replace(
                "ptr addrspace(1) null, ptr addrspace(1) %luts",
                "ptr addrspace(1) %luts, ptr addrspace(1) null",
            )
            .replace(
                "br i1 %has_luts, label %miss, label %hit",
                "br i1 %has_luts, label %hit, label %miss",
            );
        let spv =
            translate_sanitized_native(&ll, Stage::Kernel, &tmp()).expect("padded optional arrays");
        assert_guarded_half_address(&disassemble(&spv).expect("disassemble"), 12 + 8 * field);
    }
}

#[test]
fn driver_construction_api_preserves_optional_half_addresses() {
    for field in 0..2 {
        let spv = metal2vulkan::translate_native_no_retry_constructed_with_options(
            &source(field),
            Stage::Kernel,
            TransformOptions::default(),
        )
        .expect("driver raw-layout construction");
        assert_guarded_half_address(&disassemble(&spv).expect("disassemble"), field * 8);
    }
}

#[test]
fn selected_constant_config_first_byte_keeps_descriptor_element_pointer() {
    let source =
        include_str!("../validation/fixtures/public/kernel_selected_constant_config_byte.ll");
    for ll in [
        source.to_string(),
        source.replace(
            "ptr addrspace(2) @fallback, ptr addrspace(2) %config",
            "ptr addrspace(2) %config, ptr addrspace(2) @fallback",
        ),
    ] {
        let spv = metal2vulkan::translate_native_no_retry_constructed_with_options(
            &ll,
            Stage::Kernel,
            TransformOptions::default(),
        )
        .expect("constant/data aggregate first-byte construction");
        let path = tmp().join("selected-config-first-byte.spv");
        std::fs::write(&path, spv).expect("write authored output");
        metal2vulkan::tools::spirv_val(path.to_str().expect("UTF-8 scratch path"))
            .expect("validate descriptor element pointer");
        std::fs::remove_file(path).expect("remove authored output");
    }
}
