use metal2vulkan::passes::{Stage, TransformOptions};
use metal2vulkan::reflect::{ResourceKind, DEFAULT_DESCRIPTOR_LAYOUT, SAMPLER_BINDING_RANGE};
use metal2vulkan::{disassemble, translate_sanitized_native, translate_sanitized_native_reflected};
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("m2v_gated_system_values_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

const GATED_VIEWPORT: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

@layered.MTL_FC_INIT_0_b = internal addrspace(2) externally_initialized constant i8 undef, section "air.fc_initializer", align 1

define <4 x float> @frag(<4 x float> %pos, i32 %viewport) {
entry:
  %f = uitofp i32 %viewport to float
  %v0 = insertelement <4 x float> undef, float %f, i32 0
  %v1 = insertelement <4 x float> %v0, float 0.000000e+00, i32 1
  %v2 = insertelement <4 x float> %v1, float 0.000000e+00, i32 2
  %v3 = insertelement <4 x float> %v2, float 1.000000e+00, i32 3
  ret <4 x float> %v3
}

!air.fragment = !{!0}
!air.function_constants = !{!7}
!0 = !{ptr @frag, !1, !3}
!1 = !{!2}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!4, !5}
!4 = !{i32 0, !"air.position", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
!5 = !{i32 1, GATE!"air.viewport_array_index", !"air.arg_type_name", !"uint", !"air.arg_name", !"viewport"}
!7 = !{ptr addrspace(2) @layered.MTL_FC_INIT_0_b, !"bool", !"layered", i32 0, i1 false}
"#;

fn translate_to_asm(gate: &str) -> String {
    let ll = GATED_VIEWPORT.replace("GATE", gate);
    assert_ne!(
        ll, GATED_VIEWPORT,
        "the gate placeholder must be substituted"
    );
    let spv = translate_sanitized_native(&ll, Stage::Fragment, &tmp()).expect("translate");
    disassemble(&spv).expect("disassemble")
}

fn mentions(asm: &str, needle: &str) -> bool {
    asm.lines().any(|line| line.contains(needle))
}

#[test]
fn a_gated_off_system_value_declares_no_builtin_and_no_capability() {
    let asm = translate_to_asm(r#"!"air.function_constant", !7, "#);
    assert!(
        !mentions(&asm, "BuiltIn ViewportIndex"),
        "an absent parameter must not claim its builtin:\n{asm}"
    );
    assert!(
        !mentions(&asm, "ShaderViewportIndex"),
        "and must not make the module require the device feature:\n{asm}"
    );
}

#[test]
fn the_same_system_value_ungated_is_still_wired() {
    let asm = translate_to_asm("");
    assert!(
        mentions(&asm, "BuiltIn ViewportIndex"),
        "a present parameter must be wired to its builtin:\n{asm}"
    );
    assert!(
        mentions(&asm, "OpCapability ShaderViewportIndex"),
        "and the module must declare what reading it needs:\n{asm}"
    );
}

#[test]
fn a_gated_off_resource_keeps_its_descriptor() {
    const GATED_BUFFER: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

@enabled.MTL_FC_INIT_0_b = internal addrspace(2) externally_initialized constant i8 undef, section "air.fc_initializer", align 1

define <4 x float> @frag(<4 x float> %pos, ptr addrspace(2) %cfg) {
entry:
  %n = load i32, ptr addrspace(2) %cfg
  %f = uitofp i32 %n to float
  %v0 = insertelement <4 x float> undef, float %f, i32 0
  %v1 = insertelement <4 x float> %v0, float 0.000000e+00, i32 1
  %v2 = insertelement <4 x float> %v1, float 0.000000e+00, i32 2
  %v3 = insertelement <4 x float> %v2, float 1.000000e+00, i32 3
  ret <4 x float> %v3
}

!air.fragment = !{!0}
!air.function_constants = !{!7}
!0 = !{ptr @frag, !1, !3}
!1 = !{!2}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!4, !5}
!4 = !{i32 0, !"air.position", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
!5 = !{i32 1, !"air.function_constant", !7, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 3, i32 1, !"air.read", !"air.address_space", i32 2, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"cfg"}
!7 = !{ptr addrspace(2) @enabled.MTL_FC_INIT_0_b, !"bool", !"enabled", i32 0, i1 false}
"#;

    let spv = translate_sanitized_native(GATED_BUFFER, Stage::Fragment, &tmp()).expect("translate");
    let asm = disassemble(&spv).expect("disassemble");
    assert!(
        mentions(&asm, "Binding"),
        "a gated resource keeps the descriptor the pipeline layout declares:\n{asm}"
    );
}

#[test]
fn a_gated_sampler_is_a_descriptor_in_every_stage() {
    const SAMPLE_CALL: &str = r#"  %s = call { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1) %tex, ptr addrspace(2) %smp, <2 x float> zeroinitializer, i1 true, <2 x i32> zeroinitializer, i1 false, float 0.000000e+00, float 0.000000e+00, i32 0)
  %c = extractvalue { <4 x float>, i8 } %s, 0
"#;
    const DECLARE: &str = r#"declare { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1), ptr addrspace(2), <2 x float>, i1, <2 x i32>, i1, float, float, i32)
"#;
    const GATED_PAIR: &str = r#"!8 = !{i32 IDX, !"air.function_constant", !9, !"air.texture", !"air.location_index", i32 2, i32 1, !"air.sample", !"air.arg_type_name", !"texture2d<float, sample>", !"air.arg_name", !"tex"}
!9 = !{ptr addrspace(2) @enabled.MTL_FC_INIT_0_b, !"bool", !"enabled", i32 0, i1 false}
!10 = !{i32 IDX1, !"air.function_constant", !9, !"air.sampler", !"air.location_index", i32 3, i32 1, !"air.arg_type_name", !"sampler", !"air.arg_name", !"smp"}
"#;
    const PROLOGUE: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

@enabled.MTL_FC_INIT_0_b = internal addrspace(2) externally_initialized constant i8 undef, section "air.fc_initializer", align 1

"#;

    let fragment = format!(
        "{PROLOGUE}define <4 x float> @frag(<4 x float> %pos, ptr addrspace(1) %tex, ptr addrspace(2) %smp) {{\nentry:\n{SAMPLE_CALL}  ret <4 x float> %c\n}}\n{DECLARE}\n!air.fragment = !{{!0}}\n!air.function_constants = !{{!9}}\n!0 = !{{ptr @frag, !1, !3}}\n!1 = !{{!2}}\n!2 = !{{!\"air.render_target\", i32 0, i32 0, !\"air.arg_type_name\", !\"float4\"}}\n!3 = !{{!4, !8, !10}}\n!4 = !{{i32 0, !\"air.position\", !\"air.center\", !\"air.arg_type_name\", !\"float4\", !\"air.arg_name\", !\"pos\"}}\n{}",
        GATED_PAIR.replace("IDX1", "2").replace("IDX", "1")
    );
    let vertex = format!(
        "{PROLOGUE}define <4 x float> @vert(ptr addrspace(1) %tex, ptr addrspace(2) %smp) {{\nentry:\n{SAMPLE_CALL}  ret <4 x float> %c\n}}\n{DECLARE}\n!air.vertex = !{{!0}}\n!air.function_constants = !{{!9}}\n!0 = !{{ptr @vert, !1, !3}}\n!1 = !{{!2}}\n!2 = !{{!\"air.position\", !\"air.arg_type_name\", !\"float4\"}}\n!3 = !{{!8, !10}}\n{}",
        GATED_PAIR.replace("IDX1", "1").replace("IDX", "0")
    );
    let kernel = format!(
        "{PROLOGUE}define void @k(ptr addrspace(1) %tex, ptr addrspace(2) %smp, ptr addrspace(1) %out) {{\nentry:\n{SAMPLE_CALL}  store <4 x float> %c, ptr addrspace(1) %out, align 16\n  ret void\n}}\n{DECLARE}\n!air.kernel = !{{!0}}\n!air.function_constants = !{{!9}}\n!0 = !{{ptr @k, !1, !3}}\n!1 = !{{}}\n!3 = !{{!8, !10, !11}}\n!11 = !{{i32 2, !\"air.buffer\", !\"air.location_index\", i32 0, i32 1, !\"air.write\", !\"air.address_space\", i32 1, !\"air.arg_type_size\", i32 16, !\"air.arg_type_align_size\", i32 16, !\"air.arg_type_name\", !\"float4\", !\"air.arg_name\", !\"out\"}}\n{}",
        GATED_PAIR.replace("IDX1", "1").replace("IDX", "0")
    );

    for (stage, ll) in [
        (Stage::Fragment, &fragment),
        (Stage::Vertex, &vertex),
        (Stage::Kernel, &kernel),
    ] {
        let (_, reflection) = translate_sanitized_native_reflected(
            ll,
            stage,
            &tmp(),
            TransformOptions {
                descriptor_layout: DEFAULT_DESCRIPTOR_LAYOUT,
                ..TransformOptions::default()
            },
        )
        .unwrap_or_else(|error| panic!("{stage:?}: translate: {error}\n{ll}"));
        let samplers = reflection
            .bindings
            .iter()
            .filter(|binding| binding.kind == ResourceKind::Sampler)
            .collect::<Vec<_>>();
        assert_eq!(
            samplers.len(),
            1,
            "{stage:?}: the gated sampler argument is one descriptor; got {:?}",
            reflection.bindings
        );
        assert_eq!(
            samplers[0].metal_index, 3,
            "{stage:?}: at the Metal slot AIR declared"
        );
        let location = samplers[0]
            .descriptor
            .expect("a reported sampler binds a descriptor");
        assert!(
            SAMPLER_BINDING_RANGE.contains(location.binding),
            "{stage:?}: in the sampler band; got {location:?}"
        );
        assert!(
            !reflection
                .bindings
                .iter()
                .any(|binding| binding.kind == ResourceKind::SynthesizedReadSampler),
            "{stage:?}: the declared sampler is the one the sample takes, so the translator has no \
             reason to invent one: {:?}",
            reflection.bindings
        );
    }
}
