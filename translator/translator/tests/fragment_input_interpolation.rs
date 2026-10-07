use metal2vulkan::passes::Stage;
use metal2vulkan::{disassemble, translate_sanitized_native};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("m2v_fragment_interpolation_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

const INTERPOLATED_FRAGMENT: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define <4 x float> @frag(<4 x float> %pos, float %a, float %b, float %c, float %d, i32 %e) {
entry:
  %s0 = fadd float %a, %b
  %s1 = fadd float %s0, %c
  %s2 = fadd float %s1, %d
  %ef = sitofp i32 %e to float
  %s3 = fadd float %s2, %ef
  %v0 = insertelement <4 x float> undef, float %s3, i32 0
  %v1 = insertelement <4 x float> %v0, float 0.000000e+00, i32 1
  %v2 = insertelement <4 x float> %v1, float 0.000000e+00, i32 2
  %v3 = insertelement <4 x float> %v2, float 1.000000e+00, i32 3
  ret <4 x float> %v3
}

!air.fragment = !{!0}
!0 = !{ptr @frag, !1, !3}
!1 = !{!2}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!4, !5, !6, !7, !8, !9}
!4 = !{i32 0, !"air.position", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
!5 = !{i32 1, !"air.fragment_input", !"generated(a)", !"air.center", !"air.perspective", !"air.arg_type_name", !"float", !"air.arg_name", !"a"}
!6 = !{i32 2, !"air.fragment_input", !"generated(b)", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float", !"air.arg_name", !"b"}
!7 = !{i32 3, !"air.fragment_input", !"generated(c)", !"air.centroid", !"air.perspective", !"air.arg_type_name", !"float", !"air.arg_name", !"c"}
!8 = !{i32 4, !"air.fragment_input", !"generated(d)", !"air.sample", !"air.no_perspective", !"air.arg_type_name", !"float", !"air.arg_name", !"d"}
!9 = !{i32 5, !"air.fragment_input", !"generated(e)", !"air.flat", !"air.arg_type_name", !"int", !"air.arg_name", !"e"}
"#;

fn translate_to_asm(ll: &str) -> String {
    let spv = translate_sanitized_native(ll, Stage::Fragment, &tmp()).expect("translate");
    disassemble(&spv).expect("disassemble")
}

fn interpolation_at(asm: &str, location: u32) -> BTreeSet<String> {
    let target = asm
        .lines()
        .map(str::trim)
        .find_map(|line| {
            let rest = line.strip_prefix("OpDecorate ")?;
            let (id, tail) = rest.split_once(' ')?;
            (tail == format!("Location {location}")).then(|| id.to_string())
        })
        .unwrap_or_else(|| panic!("no Input variable decorated Location {location}:\n{asm}"));
    asm.lines()
        .map(str::trim)
        .filter_map(|line| {
            let rest = line.strip_prefix("OpDecorate ")?;
            let (id, tail) = rest.split_once(' ')?;
            (id == target && matches!(tail, "Flat" | "NoPerspective" | "Centroid" | "Sample"))
                .then(|| tail.to_string())
        })
        .collect()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[test]
fn each_air_interpolation_attribute_reaches_the_input_variable() {
    let asm = translate_to_asm(INTERPOLATED_FRAGMENT);

    assert_eq!(
        interpolation_at(&asm, 0),
        set(&[]),
        "`air.center` + `air.perspective` are Vulkan's defaults and must decorate nothing"
    );
    assert_eq!(
        interpolation_at(&asm, 1),
        set(&["NoPerspective"]),
        "`air.no_perspective` is screen-space linear interpolation"
    );
    assert_eq!(
        interpolation_at(&asm, 2),
        set(&["Centroid"]),
        "`air.centroid` samples inside the covered area of the primitive"
    );
    assert_eq!(
        interpolation_at(&asm, 3),
        set(&["NoPerspective", "Sample"]),
        "the two axes are independent and both must survive"
    );
    assert_eq!(
        interpolation_at(&asm, 4),
        set(&["Flat"]),
        "`air.flat` replaces the pair rather than joining it"
    );
}

#[test]
fn per_sample_interpolation_requests_its_capability() {
    let asm = translate_to_asm(INTERPOLATED_FRAGMENT);
    assert!(
        asm.lines()
            .any(|line| line.trim() == "OpCapability SampleRateShading"),
        "a `Sample`-decorated input must declare SampleRateShading:\n{asm}"
    );

    let no_sample = INTERPOLATED_FRAGMENT.replace(r#"!"air.sample", "#, r#"!"air.center", "#);
    assert_ne!(no_sample, INTERPOLATED_FRAGMENT);
    let asm = translate_to_asm(&no_sample);
    assert!(
        !asm.contains("Sample"),
        "with no per-sample varying the decoration must be gone:\n{asm}"
    );
    assert!(
        !asm.lines()
            .any(|line| line.trim() == "OpCapability SampleRateShading"),
        "and the capability must not be requested speculatively:\n{asm}"
    );
}

#[test]
fn an_uninterpolatable_type_is_flat_over_what_air_asked_for() {
    let ll = INTERPOLATED_FRAGMENT.replace(
        r#"!"generated(e)", !"air.flat""#,
        r#"!"generated(e)", !"air.centroid", !"air.no_perspective""#,
    );
    assert_ne!(ll, INTERPOLATED_FRAGMENT);
    let asm = translate_to_asm(&ll);
    assert_eq!(
        interpolation_at(&asm, 4),
        set(&["Flat"]),
        "an integer fragment input cannot be interpolated (VUID-StandaloneSpirv-Flat-04744)"
    );
}
