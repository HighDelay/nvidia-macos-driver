use metal2vulkan::passes::Stage;
use metal2vulkan::{disassemble, translate_sanitized_native};
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("m2v_declared_out_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

const VERTEX: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define <{ <4 x float>, <2 x float>, <2 x float> }> @vert(<4 x float> %p, <2 x float> %uv) {
entry:
  %r0 = insertvalue <{ <4 x float>, <2 x float>, <2 x float> }> undef, <4 x float> %p, 0
  %r1 = insertvalue <{ <4 x float>, <2 x float>, <2 x float> }> %r0, <2 x float> %uv, 1
  ret <{ <4 x float>, <2 x float>, <2 x float> }> %r1
}

!air.vertex = !{!0}
!0 = !{ptr @vert, !1, !5}
!1 = !{!2, !3, !4}
!2 = !{!"air.position", !"air.arg_type_name", !"float4", !"air.arg_name", !"position"}
!3 = !{!"air.vertex_output", !"generated(uv)", !"air.arg_type_name", !"float2", !"air.arg_name", !"uv"}
!4 = !{!"air.vertex_output", !"generated(unwritten)", !"air.arg_type_name", !"float2", !"air.arg_name", !"unwritten"}
!5 = !{!6, !7}
!6 = !{i32 0, !"air.vertex_input", !"air.location_index", i32 0, !"air.arg_type_name", !"float4", !"air.arg_name", !"p"}
!7 = !{i32 1, !"air.vertex_input", !"air.location_index", i32 1, !"air.arg_type_name", !"float2", !"air.arg_name", !"uv"}
"#;

const FRAGMENT: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define <4 x float> @frag(<4 x float> %pos, <2 x float> %uv, <2 x float> %unwritten) {
entry:
  %x = extractelement <2 x float> %uv, i32 0
  %y = extractelement <2 x float> %unwritten, i32 1
  %v0 = insertelement <4 x float> undef, float %x, i32 0
  %v1 = insertelement <4 x float> %v0, float %y, i32 1
  %v2 = insertelement <4 x float> %v1, float 0.000000e+00, i32 2
  %v3 = insertelement <4 x float> %v2, float 1.000000e+00, i32 3
  ret <4 x float> %v3
}

!air.fragment = !{!0}
!0 = !{ptr @frag, !1, !3}
!1 = !{!2}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!4, !5, !6}
!4 = !{i32 0, !"air.position", !"air.center", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
!5 = !{i32 1, !"air.fragment_input", !"generated(uv)", !"air.center", !"air.arg_type_name", !"float2", !"air.arg_name", !"uv"}
!6 = !{i32 2, !"air.fragment_input", !"generated(unwritten)", !"air.center", !"air.arg_type_name", !"float2", !"air.arg_name", !"unwritten"}
"#;

const FRAGMENT_TWO_TARGETS: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define <{ <4 x float>, <4 x float> }> @frag(<4 x float> %pos) {
entry:
  %r0 = insertvalue <{ <4 x float>, <4 x float> }> undef, <4 x float> %pos, 0
  ret <{ <4 x float>, <4 x float> }> %r0
}

!air.fragment = !{!0}
!0 = !{ptr @frag, !1, !4}
!1 = !{!2, !3}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!"air.render_target", i32 1, i32 0, !"air.arg_type_name", !"float4"}
!4 = !{!5}
!5 = !{i32 0, !"air.position", !"air.center", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
"#;

fn interface_locations(asm: &str, storage_class: &str) -> Vec<u32> {
    let ids: Vec<&str> = asm
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            let (id, rest) = line.split_once(" = OpVariable ")?;
            rest.split_whitespace()
                .last()
                .filter(|class| *class == storage_class)
                .map(|_| id)
        })
        .collect();
    let mut out: Vec<u32> = asm
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            let rest = line.strip_prefix("OpDecorate ")?;
            let (id, rest) = rest.split_once(' ')?;
            let location = rest.strip_prefix("Location ")?;
            ids.contains(&id).then(|| location.trim().parse().ok())?
        })
        .collect();
    out.sort_unstable();
    out
}

fn asm(ll: &str, stage: Stage) -> String {
    let spv = translate_sanitized_native(ll, stage, &tmp()).expect("translate");
    disassemble(&spv).expect("disassemble")
}

#[test]
fn an_unwritten_vertex_varying_still_reaches_the_module() {
    let vertex = asm(VERTEX, Stage::Vertex);
    assert_eq!(
        interface_locations(&vertex, "Output"),
        vec![0, 1],
        "both declared varyings are Output variables, `unwritten` included:\n{vertex}"
    );
}

#[test]
fn the_fragment_reading_an_unwritten_varying_has_a_producer_for_it() {
    let vertex = asm(VERTEX, Stage::Vertex);
    let fragment = asm(FRAGMENT, Stage::Fragment);
    let written = interface_locations(&vertex, "Output");
    let read = interface_locations(&fragment, "Input");
    assert_eq!(
        written, read,
        "the vertex writes {written:?} and the fragment reads {read:?} for the same varying \
         struct\n--- vertex ---\n{vertex}\n--- fragment ---\n{fragment}"
    );
}

#[test]
fn an_unwritten_fragment_render_target_stays_unwritten() {
    let fragment = asm(FRAGMENT_TWO_TARGETS, Stage::Fragment);
    assert_eq!(
        interface_locations(&fragment, "Output"),
        vec![0],
        "only the render target the shader writes is emitted:\n{fragment}"
    );
}
