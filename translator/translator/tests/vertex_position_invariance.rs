use metal2vulkan::passes::Stage;
use metal2vulkan::{disassemble, translate_sanitized_native};
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("m2v_position_invariance_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

const INVARIANT_VERTEX: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define <{ <4 x float>, <2 x float> }> @vert(<4 x float> %p, <2 x float> %uv) {
entry:
  %r0 = insertvalue <{ <4 x float>, <2 x float> }> undef, <4 x float> %p, 0
  %r1 = insertvalue <{ <4 x float>, <2 x float> }> %r0, <2 x float> %uv, 1
  ret <{ <4 x float>, <2 x float> }> %r1
}

!air.vertex = !{!0}
!0 = !{ptr @vert, !1, !4}
!1 = !{!2, !3}
!2 = !{!"air.position", !"air.invariant", !"air.arg_type_name", !"float4", !"air.arg_name", !"position"}
!3 = !{!"air.vertex_output", !"generated(uv)", !"air.arg_type_name", !"float2", !"air.arg_name", !"uv"}
!4 = !{!5, !6}
!5 = !{i32 0, !"air.vertex_input", !"air.location_index", i32 0, !"air.arg_type_name", !"float4", !"air.arg_name", !"p"}
!6 = !{i32 1, !"air.vertex_input", !"air.location_index", i32 1, !"air.arg_type_name", !"float2", !"air.arg_name", !"uv"}
"#;

const BARE_INVARIANT_VERTEX: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define <4 x float> @vert(<4 x float> %p) {
entry:
  ret <4 x float> %p
}

!air.vertex = !{!0}
!0 = !{ptr @vert, !1, !3}
!1 = !{!2}
!2 = !{!"air.position", !"air.invariant", !"air.arg_type_name", !"float4", !"air.arg_name", !"position"}
!3 = !{!4}
!4 = !{i32 0, !"air.vertex_input", !"air.location_index", i32 0, !"air.arg_type_name", !"float4", !"air.arg_name", !"p"}
"#;

fn translate_to_asm(ll: &str) -> String {
    let spv = translate_sanitized_native(ll, Stage::Vertex, &tmp()).expect("translate");
    disassemble(&spv).expect("disassemble")
}

fn position_is_invariant(asm: &str) -> bool {
    let position = asm
        .lines()
        .map(str::trim)
        .find_map(|line| {
            let rest = line.strip_prefix("OpDecorate ")?;
            let (id, tail) = rest.split_once(' ')?;
            (tail == "BuiltIn Position").then(|| id.to_string())
        })
        .unwrap_or_else(|| panic!("no variable decorated BuiltIn Position:\n{asm}"));
    asm.lines()
        .map(str::trim)
        .any(|line| line == format!("OpDecorate {position} Invariant"))
}

#[test]
fn an_invariant_position_is_decorated_invariant() {
    assert!(
        position_is_invariant(&translate_to_asm(INVARIANT_VERTEX)),
        "`air.invariant` on a struct member must reach the Position variable"
    );
    assert!(
        position_is_invariant(&translate_to_asm(BARE_INVARIANT_VERTEX)),
        "`air.invariant` on a bare position return must reach it too"
    );
}

#[test]
fn a_position_metal_did_not_declare_invariant_is_not_decorated() {
    for source in [INVARIANT_VERTEX, BARE_INVARIANT_VERTEX] {
        let plain = source.replace(r#", !"air.invariant""#, "");
        assert_ne!(plain, source, "the marker must be removed");
        assert!(
            !position_is_invariant(&translate_to_asm(&plain)),
            "a position without `air.invariant` must not be decorated:\n{plain}"
        );
    }
}

#[test]
fn invariance_does_not_leak_to_the_other_outputs() {
    let asm = translate_to_asm(INVARIANT_VERTEX);
    let invariant_ids = asm
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_suffix(" Invariant")?.strip_prefix("OpDecorate "))
        .count();
    assert_eq!(
        invariant_ids, 1,
        "only the position is invariant; the `uv` varying is not:\n{asm}"
    );
}
