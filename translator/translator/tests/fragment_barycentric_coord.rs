use metal2vulkan::passes::Stage;
use metal2vulkan::{disassemble, translate_sanitized_native};
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("m2v_fragment_barycentric_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

const BARYCENTRIC_FRAGMENT: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define <4 x float> @frag(<4 x float> %pos, <3 x float> %bary) {
entry:
  %x = extractelement <3 x float> %bary, i32 0
  %v0 = insertelement <4 x float> undef, float %x, i32 0
  %v1 = insertelement <4 x float> %v0, float 0.000000e+00, i32 1
  %v2 = insertelement <4 x float> %v1, float 0.000000e+00, i32 2
  %v3 = insertelement <4 x float> %v2, float 1.000000e+00, i32 3
  ret <4 x float> %v3
}

!air.fragment = !{!0}
!0 = !{ptr @frag, !1, !3}
!1 = !{!2}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!4, !5}
!4 = !{i32 0, !"air.position", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
!5 = !{i32 1, !"air.barycentric_coord", !"air.center", !"air.PERSPECTIVE", !"air.arg_type_name", !"float3", !"air.arg_name", !"bary"}
"#;

fn translate_to_asm(perspective: &str) -> String {
    let ll = BARYCENTRIC_FRAGMENT.replace("air.PERSPECTIVE", &format!("air.{perspective}"));
    assert_ne!(ll, BARYCENTRIC_FRAGMENT, "the marker must be substituted");
    let spv = translate_sanitized_native(&ll, Stage::Fragment, &tmp()).expect("translate");
    disassemble(&spv).expect("disassemble")
}

fn barycentric_builtin(asm: &str) -> Option<String> {
    asm.lines().map(str::trim).find_map(|line| {
        let rest = line.strip_prefix("OpDecorate ")?;
        let (_, tail) = rest.split_once(' ')?;
        let builtin = tail.strip_prefix("BuiltIn ")?;
        matches!(builtin, "BaryCoordKHR" | "BaryCoordNoPerspKHR").then(|| builtin.to_string())
    })
}

#[test]
fn the_perspective_marker_picks_the_builtin() {
    assert_eq!(
        barycentric_builtin(&translate_to_asm("perspective")).as_deref(),
        Some("BaryCoordKHR"),
        "perspective-correct barycentrics are the plain builtin"
    );
    assert_eq!(
        barycentric_builtin(&translate_to_asm("no_perspective")).as_deref(),
        Some("BaryCoordNoPerspKHR"),
        "screen-space barycentrics are a different builtin, not a decoration on the same one"
    );
}

#[test]
fn the_builtin_brings_its_capability_and_its_extension() {
    for perspective in ["perspective", "no_perspective"] {
        let asm = translate_to_asm(perspective);
        let has = |needle: &str| asm.lines().any(|line| line.trim() == needle);
        assert!(
            has("OpCapability FragmentBarycentricKHR"),
            "missing capability for {perspective}:\n{asm}"
        );
        assert!(
            has(r#"OpExtension "SPV_KHR_fragment_shader_barycentric""#),
            "missing extension for {perspective}:\n{asm}"
        );
    }
}

#[test]
fn a_fragment_without_barycentrics_declares_neither() {
    let plain = BARYCENTRIC_FRAGMENT
        .replace("air.PERSPECTIVE", "air.perspective")
        .replace(
            r#"!"air.barycentric_coord", !"air.center", !"air.perspective""#,
            r#"!"air.fragment_input", !"generated(bary)", !"air.center", !"air.perspective""#,
        );
    let spv = translate_sanitized_native(&plain, Stage::Fragment, &tmp()).expect("translate");
    let asm = disassemble(&spv).expect("disassemble");
    assert!(barycentric_builtin(&asm).is_none(), "{asm}");
    assert!(
        !asm.contains("FragmentBarycentricKHR") && !asm.contains("fragment_shader_barycentric"),
        "{asm}"
    );
}

#[test]
fn a_barycentric_parameter_of_the_wrong_shape_fallbacks() {
    let ll = BARYCENTRIC_FRAGMENT
        .replace("air.PERSPECTIVE", "air.perspective")
        .replace("<3 x float> %bary", "<2 x float> %bary")
        .replace("extractelement <3 x float>", "extractelement <2 x float>")
        .replace(
            r#"!"float3", !"air.arg_name", !"bary""#,
            r#"!"float2", !"air.arg_name", !"bary""#,
        );
    match translate_sanitized_native(&ll, Stage::Fragment, &tmp()) {
        Ok(spv) => panic!("expected a FALLBACK, got {} bytes", spv.len()),
        Err(e) => assert!(
            e.contains("barycentric_coord") && e.contains("float3"),
            "the diagnostic should say what shape is required; got: {e}"
        ),
    }
}
