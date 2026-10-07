use metal2vulkan::passes::{Stage, TransformOptions};
use metal2vulkan::reflect::KernelDispatch;
use metal2vulkan::{
    disassemble, reflect_sanitized, translate_sanitized_native,
    translate_sanitized_native_with_options,
};
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("m2v_stage_root_attributes_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

const FRAGMENT: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define <4 x float> @frag(<4 x float> %pos, ptr addrspace(1) %out) {
entry:
  store float 1.000000e+00, ptr addrspace(1) %out, align 4
  ret <4 x float> %pos
}

!air.fragment = !{!0}
!0 = !{ptr @frag, !1, !3ATTR}
!1 = !{!2}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!4, !5}
!4 = !{i32 0, !"air.position", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
!5 = !{i32 1, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_name", !"float*", !"air.arg_name", !"out"}
"#;

const FRAGMENT_WITH_DEPTH: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define <{ <4 x float>, float }> @frag(<4 x float> %pos) {
entry:
  %d = extractelement <4 x float> %pos, i32 2
  %r0 = insertvalue <{ <4 x float>, float }> undef, <4 x float> %pos, 0
  %r1 = insertvalue <{ <4 x float>, float }> %r0, float %d, 1
  ret <{ <4 x float>, float }> %r1
}

!air.fragment = !{!0}
!0 = !{ptr @frag, !1, !4ATTR}
!1 = !{!2, !3}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!"air.depth", !"air.any", !"air.arg_type_name", !"float"}
!4 = !{!5}
!5 = !{i32 0, !"air.position", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
"#;

const FRAGMENT_WITH_STENCIL: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define <{ <4 x float>, i32 }> @frag(<4 x float> %pos) {
entry:
  %d = extractelement <4 x float> %pos, i32 2
  %s = bitcast float %d to i32
  %r0 = insertvalue <{ <4 x float>, i32 }> undef, <4 x float> %pos, 0
  %r1 = insertvalue <{ <4 x float>, i32 }> %r0, i32 %s, 1
  ret <{ <4 x float>, i32 }> %r1
}

!air.fragment = !{!0}
!0 = !{ptr @frag, !1, !4ATTR}
!1 = !{!2, !3}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!"air.stencil", !"air.arg_type_name", !"uint"}
!4 = !{!5}
!5 = !{i32 0, !"air.position", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
"#;

const KERNEL: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define void @k(ptr addrspace(1) %out) {
entry:
  store float 1.000000e+00, ptr addrspace(1) %out, align 4
  ret void
}

!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2ATTR}
!1 = !{}
!2 = !{!3}
!3 = !{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_name", !"float*", !"air.arg_name", !"out"}
"#;

const VERTEX: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define <4 x float> @vert(i32 %vid) {
entry:
  %f = sitofp i32 %vid to float
  %v0 = insertelement <4 x float> undef, float %f, i32 0
  %v1 = insertelement <4 x float> %v0, float 0.000000e+00, i32 1
  %v2 = insertelement <4 x float> %v1, float 0.000000e+00, i32 2
  %v3 = insertelement <4 x float> %v2, float 1.000000e+00, i32 3
  ret <4 x float> %v3
}

!air.vertex = !{!0}
!0 = !{ptr @vert, !1, !3ATTR}
!1 = !{!2}
!2 = !{!"air.position", !"air.arg_type_name", !"float4"}
!3 = !{!4}
!4 = !{i32 0, !"air.vertex_id", !"air.arg_type_name", !"uint", !"air.arg_name", !"vid"}
"#;

fn with_attribute(template: &str, attr: &str) -> String {
    let spliced = template.replace("ATTR", attr);
    assert_ne!(spliced, template, "template has no ATTR splice point");
    spliced
}

fn translate(template: &str, attr: &str, stage: Stage) -> Result<String, String> {
    let ll = with_attribute(template, attr);
    let spv = translate_sanitized_native(&ll, stage, &tmp()).map_err(|error| error.to_string())?;
    Ok(disassemble(&spv).expect("disassemble"))
}

fn dispatch(template: &str, attr: &str, local_size: [u32; 3]) -> Result<String, String> {
    let ll = with_attribute(template, attr);
    let options = TransformOptions {
        kernel_local_size: local_size,
        kernel_dispatch: Some(KernelDispatch::Workgroups),
        ..TransformOptions::default()
    };
    let spv = translate_sanitized_native_with_options(&ll, Stage::Kernel, &tmp(), options)?;
    Ok(disassemble(&spv).expect("disassemble"))
}

fn execution_modes(asm: &str) -> Vec<String> {
    asm.lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("OpExecutionMode "))
        .filter_map(|rest| rest.split_once(' ').map(|(_, mode)| mode.to_string()))
        .collect()
}

#[test]
fn early_fragment_tests_reaches_the_execution_mode() {
    let declared = translate(FRAGMENT, ", !\"early_fragment_tests\"", Stage::Fragment)
        .expect("fragment declaring early_fragment_tests translates");
    assert!(
        execution_modes(&declared)
            .iter()
            .any(|mode| mode == "EarlyFragmentTests"),
        "a fragment declaring early_fragment_tests must say so in SPIR-V:\n{declared}"
    );
}

#[test]
fn a_fragment_that_did_not_ask_for_early_tests_does_not_get_them() {
    let plain = translate(FRAGMENT, "", Stage::Fragment).expect("plain fragment translates");
    assert!(
        !execution_modes(&plain)
            .iter()
            .any(|mode| mode == "EarlyFragmentTests"),
        "nothing declared early_fragment_tests:\n{plain}"
    );
    assert!(
        execution_modes(&plain)
            .iter()
            .any(|mode| mode == "OriginUpperLeft"),
        "the fragment's ordinary modes must survive:\n{plain}"
    );
}

#[test]
fn early_fragment_tests_and_a_written_test_value_are_refused() {
    for (template, role) in [
        (FRAGMENT_WITH_DEPTH, "air.depth"),
        (FRAGMENT_WITH_STENCIL, "air.stencil"),
    ] {
        let error = translate(template, ", !\"early_fragment_tests\"", Stage::Fragment)
            .expect_err("a fragment cannot write a test value and demand the test run first");
        assert!(
            error.contains("early_fragment_tests") && error.contains(role),
            "the refusal must name both halves of the contradiction: {error}"
        );

        translate(template, "", Stage::Fragment)
            .unwrap_or_else(|error| panic!("writing {role} alone must translate: {error}"));
    }
}

#[test]
fn a_dispatch_past_the_declared_threadgroup_ceiling_is_refused() {
    let ceiling =
        |threads: u32| format!(", !6}}\n!6 = !{{!\"air.max_work_group_size\", i32 {threads}");

    let error =
        dispatch(KERNEL, &ceiling(32), [64, 1, 1]).expect_err("64 threads is past a ceiling of 32");
    assert!(
        error.contains("air.max_work_group_size") && error.contains("32"),
        "the refusal must name the ceiling it read: {error}"
    );

    for (attr, local_size) in [
        (ceiling(32), [8u32, 2, 2]),
        (ceiling(64), [64, 1, 1]),
        (ceiling(729), [9, 9, 9]),
    ] {
        let [x, y, z] = local_size;
        let asm = dispatch(KERNEL, &attr, local_size)
            .unwrap_or_else(|error| panic!("{x}x{y}x{z} must fit `{attr}`: {error}"));
        assert!(
            execution_modes(&asm)
                .iter()
                .any(|mode| mode == &format!("LocalSize {x} {y} {z}")),
            "the requested local size must still be emitted:\n{asm}"
        );
    }
}

#[test]
fn the_reflected_threadgroup_ceiling_is_the_one_translation_enforces() {
    let declared = with_attribute(KERNEL, ", !6}\n!6 = !{!\"air.max_work_group_size\", i32 64");
    let reflection = reflect_sanitized(&declared, Stage::Kernel, TransformOptions::default())
        .expect("reflection reports the ceiling rather than enforcing it");
    let ceiling = reflection
        .max_work_group_size
        .expect("the kernel declares a ceiling");
    assert_eq!(ceiling, 64);

    let at = [8, ceiling / 8, 1];
    let past = [8, ceiling / 8, 2];
    dispatch(
        KERNEL,
        ", !6}\n!6 = !{!\"air.max_work_group_size\", i32 64",
        at,
    )
    .expect("a dispatch of exactly the reported ceiling translates");
    let error = dispatch(
        KERNEL,
        ", !6}\n!6 = !{!\"air.max_work_group_size\", i32 64",
        past,
    )
    .expect_err("a dispatch past the reported ceiling is refused");
    assert!(
        error.contains(&ceiling.to_string()),
        "the refusal must cite the ceiling reflection reported: {error}"
    );

    let plain = with_attribute(KERNEL, "");
    let plain_reflection = reflect_sanitized(&plain, Stage::Kernel, TransformOptions::default())
        .expect("a kernel with no ceiling still reflects");
    assert_eq!(plain_reflection.max_work_group_size, None);
    dispatch(KERNEL, "", [16, 16, 4]).expect("an undeclared ceiling bounds nothing");
}

#[test]
fn an_attribute_no_stage_models_is_refused_by_name() {
    let cases = [
        (FRAGMENT, Stage::Fragment, "fragment"),
        (KERNEL, Stage::Kernel, "kernel"),
        (VERTEX, Stage::Vertex, "vertex"),
    ];
    for (template, stage, label) in cases {
        translate(template, "", stage)
            .unwrap_or_else(|error| panic!("{label} must translate with a bare root: {error}"));

        let error = match translate(template, ", !\"air.invented_stage_attribute\"", stage) {
            Ok(asm) => panic!("{label} accepted an unmodelled attribute:\n{asm}"),
            Err(error) => error,
        };
        assert!(
            error.contains("air.invented_stage_attribute"),
            "the {label} refusal must name the attribute it could not read: {error}"
        );
    }
}

#[test]
fn an_attribute_belonging_to_another_stage_is_not_silently_accepted() {
    let error = translate(KERNEL, ", !\"early_fragment_tests\"", Stage::Kernel)
        .expect_err("a kernel has no fragment tests to run early");
    assert!(
        error.contains("early_fragment_tests"),
        "the refusal must name the attribute: {error}"
    );
}
