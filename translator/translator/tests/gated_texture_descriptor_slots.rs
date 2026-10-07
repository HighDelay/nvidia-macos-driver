use metal2vulkan::passes::Stage;
use metal2vulkan::reflect::ResourceKind;
use metal2vulkan::{disassemble, passes::TransformOptions, translate_sanitized_native_reflected};
use std::path::PathBuf;

const SUMMED_SLOTS: &str = r#"target triple = "spirv-unknown-vulkan1.2"

@live_enabled = internal addrspace(2) global i8 1, align 1
@gated_enabled = internal addrspace(2) global i8 0, align 1
@live_location = internal addrspace(2) global i32 0, align 4
@gated_location = internal addrspace(2) global i32 0, align 4

define internal void @_GLOBAL__sub_I_alternatives.metal() section "air.static_init" {
  store i32 0, ptr addrspace(2) @live_location, align 4
  store i32 0, ptr addrspace(2) @gated_location, align 4
  ret void
}

define void @blit(ptr addrspace(1) %live, ptr addrspace(1) %gated) {
entry:
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) %live, <2 x i32> zeroinitializer, <4 x float> zeroinitializer, i32 0, i32 2)
  tail call void @air.write_texture_2d_array.v4f32(ptr addrspace(1) %gated, <2 x i32> zeroinitializer, i32 0, <4 x float> zeroinitializer, i32 0, i32 2)
  ret void
}

declare void @air.write_texture_2d.v4f32(ptr addrspace(1), <2 x i32>, <4 x float>, i32, i32)
declare void @air.write_texture_2d_array.v4f32(ptr addrspace(1), <2 x i32>, i32, <4 x float>, i32, i32)

!air.kernel = !{!0}
!0 = !{ptr @blit, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.function_constant", !5, !"air.texture", !"air.location_index", ptr addrspace(2) @live_location, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"live"}
!4 = !{i32 1, !"air.function_constant", !6, !"air.texture", !"air.location_index", ptr addrspace(2) @gated_location, i32 1, !"air.write", !"air.arg_type_name", !"texture2d_array<float, write>", !"air.arg_name", !"gated"}
!5 = !{ptr addrspace(2) @live_enabled, !"bool", !"live_enabled"}
!6 = !{ptr addrspace(2) @gated_enabled, !"bool", !"gated_enabled"}
"#;

const LITERAL_SLOTS: &str = r#"target triple = "spirv-unknown-vulkan1.2"

@live_enabled = internal addrspace(2) global i8 1, align 1
@gated_enabled = internal addrspace(2) global i8 0, align 1

define void @blit(ptr addrspace(1) %live, ptr addrspace(1) %gated) {
entry:
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) %live, <2 x i32> zeroinitializer, <4 x float> zeroinitializer, i32 0, i32 2)
  tail call void @air.write_texture_2d_array.v4f32(ptr addrspace(1) %gated, <2 x i32> zeroinitializer, i32 0, <4 x float> zeroinitializer, i32 0, i32 2)
  ret void
}

declare void @air.write_texture_2d.v4f32(ptr addrspace(1), <2 x i32>, <4 x float>, i32, i32)
declare void @air.write_texture_2d_array.v4f32(ptr addrspace(1), <2 x i32>, i32, <4 x float>, i32, i32)

!air.kernel = !{!0}
!0 = !{ptr @blit, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.function_constant", !5, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"live"}
!4 = !{i32 1, !"air.function_constant", !6, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.write", !"air.arg_type_name", !"texture2d_array<float, write>", !"air.arg_name", !"gated"}
!5 = !{ptr addrspace(2) @live_enabled, !"bool", !"live_enabled"}
!6 = !{ptr addrspace(2) @gated_enabled, !"bool", !"gated_enabled"}
"#;

const LOADED_HANDLE: &str = r#"target triple = "spirv-unknown-vulkan1.2"

@live_enabled = internal addrspace(2) global i8 1, align 1
@gated_enabled = internal addrspace(2) global i8 0, align 1
@live_location = internal addrspace(2) global i32 0, align 4
@gated_count = internal addrspace(2) global i32 0, align 4
@gated_location = internal addrspace(2) global i32 0, align 4

define internal void @_GLOBAL__sub_I_alternatives.metal() section "air.static_init" {
  store i32 0, ptr addrspace(2) @live_location, align 4
  store i32 0, ptr addrspace(2) @gated_location, align 4
  store i32 0, ptr addrspace(2) @gated_count, align 4
  ret void
}

define void @probe(ptr addrspace(1) %live, ptr %gated, ptr addrspace(1) %out) {
entry:
  %handle = load ptr addrspace(1), ptr %gated, align 8
  %layers = tail call i32 @air.get_array_size_texture_2d_array(ptr addrspace(1) %handle)
  %w = tail call i32 @air.get_width_texture_2d(ptr addrspace(1) %live, i32 0)
  %sum = add i32 %layers, %w
  %f = sitofp i32 %sum to float
  %v = insertelement <4 x float> zeroinitializer, float %f, i32 0
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) %out, <2 x i32> zeroinitializer, <4 x float> %v, i32 0, i32 2)
  ret void
}

declare i32 @air.get_array_size_texture_2d_array(ptr addrspace(1))
declare i32 @air.get_width_texture_2d(ptr addrspace(1), i32)
declare void @air.write_texture_2d.v4f32(ptr addrspace(1), <2 x i32>, <4 x float>, i32, i32)

!air.kernel = !{!0}
!0 = !{ptr @probe, !1, !2}
!1 = !{}
!2 = !{!3, !4, !7}
!3 = !{i32 0, !"air.function_constant", !5, !"air.texture", !"air.location_index", ptr addrspace(2) @live_location, i32 1, !"air.sample", !"air.arg_type_name", !"texture2d<float, sample>", !"air.arg_name", !"live"}
!4 = !{i32 1, !"air.function_constant", !6, !"air.texture", !"air.location_index", ptr addrspace(2) @gated_location, ptr addrspace(2) @gated_count, !"air.sample", !"air.arg_type_name", !"array_ref<texture2d_array<float, sample>>", !"air.arg_name", !"gated"}
!5 = !{ptr addrspace(2) @live_enabled, !"bool", !"live_enabled"}
!6 = !{ptr addrspace(2) @gated_enabled, !"bool", !"gated_enabled"}
!7 = !{i32 2, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"out"}
"#;

const ABSENT_BESIDE_ONE_LIVE: &str = r#"target triple = "spirv-unknown-vulkan1.2"

@absent_enabled = internal addrspace(2) global i8 0, align 1
@absent_read_location = internal addrspace(2) global i32 0, align 4
@absent_write_location = internal addrspace(2) global i32 0, align 4

define internal void @_GLOBAL__sub_I_absent.metal() section "air.static_init" {
  store i32 0, ptr addrspace(2) @absent_read_location, align 4
  store i32 0, ptr addrspace(2) @absent_write_location, align 4
  ret void
}

define void @copy(ptr addrspace(1) %absent_source, ptr addrspace(1) %absent_sink, ptr addrspace(1) %present, ptr addrspace(1) %dest) {
entry:
  %sampler = tail call ptr addrspace(2) @air.get_read_sampler()
  %gone = tail call { <4 x float>, i8 } @air.read_texture_2d.v4f32(ptr addrspace(1) %absent_source, ptr addrspace(2) %sampler, <2 x i32> zeroinitializer, <2 x i32> zeroinitializer, i32 0, i32 1)
  %gone_color = extractvalue { <4 x float>, i8 } %gone, 0
  %live = tail call { <4 x float>, i8 } @air.read_texture_2d.v4f32(ptr addrspace(1) %present, ptr addrspace(2) %sampler, <2 x i32> zeroinitializer, <2 x i32> zeroinitializer, i32 0, i32 1)
  %live_color = extractvalue { <4 x float>, i8 } %live, 0
  %sum = fadd <4 x float> %live_color, %gone_color
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) %dest, <2 x i32> zeroinitializer, <4 x float> %sum, i32 0, i32 2)
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) %absent_sink, <2 x i32> zeroinitializer, <4 x float> zeroinitializer, i32 0, i32 2)
  ret void
}

declare ptr addrspace(2) @air.get_read_sampler()
declare { <4 x float>, i8 } @air.read_texture_2d.v4f32(ptr addrspace(1), ptr addrspace(2), <2 x i32>, <2 x i32>, i32, i32)
declare void @air.write_texture_2d.v4f32(ptr addrspace(1), <2 x i32>, <4 x float>, i32, i32)

!air.kernel = !{!0}
!0 = !{ptr @copy, !1, !2}
!1 = !{}
!2 = !{!3, !4, !6, !7}
!3 = !{i32 0, !"air.function_constant", !5, !"air.texture", !"air.location_index", ptr addrspace(2) @absent_read_location, i32 1, !"air.read", !"air.arg_type_name", !"texture2d<float, read>", !"air.arg_name", !"absent_source"}
!4 = !{i32 1, !"air.function_constant", !5, !"air.texture", !"air.location_index", ptr addrspace(2) @absent_write_location, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"absent_sink"}
!5 = !{ptr addrspace(2) @absent_enabled, !"bool", !"absent_enabled"}
!6 = !{i32 2, !"air.texture", !"air.location_index", i32 2, i32 1, !"air.read", !"air.arg_type_name", !"texture2d<float, read>", !"air.arg_name", !"present"}
!7 = !{i32 3, !"air.texture", !"air.location_index", i32 3, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"dest"}
"#;

const ABSENT_BESIDE_ONE_LIVE_SPECIALIZED: &str = r#"target triple = "spirv-unknown-vulkan1.2"

define void @copy(ptr addrspace(1) %absent_source, ptr addrspace(1) %absent_sink, ptr addrspace(1) %present, ptr addrspace(1) %dest) {
entry:
  %sampler = tail call ptr addrspace(2) @air.get_read_sampler()
  %gone = tail call { <4 x float>, i8 } @air.read_texture_2d.v4f32(ptr addrspace(1) %absent_source, ptr addrspace(2) %sampler, <2 x i32> zeroinitializer, <2 x i32> zeroinitializer, i32 0, i32 1)
  %gone_color = extractvalue { <4 x float>, i8 } %gone, 0
  %live = tail call { <4 x float>, i8 } @air.read_texture_2d.v4f32(ptr addrspace(1) %present, ptr addrspace(2) %sampler, <2 x i32> zeroinitializer, <2 x i32> zeroinitializer, i32 0, i32 1)
  %live_color = extractvalue { <4 x float>, i8 } %live, 0
  %sum = fadd <4 x float> %live_color, %gone_color
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) %dest, <2 x i32> zeroinitializer, <4 x float> %sum, i32 0, i32 2)
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) %absent_sink, <2 x i32> zeroinitializer, <4 x float> zeroinitializer, i32 0, i32 2)
  ret void
}

declare ptr addrspace(2) @air.get_read_sampler()
declare { <4 x float>, i8 } @air.read_texture_2d.v4f32(ptr addrspace(1), ptr addrspace(2), <2 x i32>, <2 x i32>, i32, i32)
declare void @air.write_texture_2d.v4f32(ptr addrspace(1), <2 x i32>, <4 x float>, i32, i32)

!air.kernel = !{!0}
!0 = !{ptr @copy, !1, !2}
!1 = !{}
!2 = !{!3, !4, !6, !7}
!3 = !{i32 0, !"air.function_constant_disabled", !5, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.read", !"air.arg_type_name", !"texture2d<float, read>", !"air.arg_name", !"absent_source"}
!4 = !{i32 1, !"air.function_constant_disabled", !5, !"air.texture", !"air.location_index", i32 1, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"absent_sink"}
!5 = !{ptr addrspace(2) null, !"bool", !"absent_enabled"}
!6 = !{i32 2, !"air.texture", !"air.location_index", i32 2, i32 1, !"air.read", !"air.arg_type_name", !"texture2d<float, read>", !"air.arg_name", !"present"}
!7 = !{i32 3, !"air.texture", !"air.location_index", i32 3, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"dest"}
"#;

fn scratch(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "m2v_gated_slots_{}_{}",
        std::process::id(),
        label.replace(['/', '.'], "_")
    ));
    std::fs::create_dir_all(&directory).expect("scratch directory");
    directory
}

fn descriptor_slots(spirv: &[u8]) -> Vec<(String, (u32, u32))> {
    let text = disassemble(spirv).expect("disassemble the translated module");
    let mut set = Vec::new();
    let mut binding = Vec::new();
    let mut uniform_constants = Vec::new();
    for line in text.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        match tokens.as_slice() {
            ["OpDecorate", id, "DescriptorSet", value] => {
                set.push(((*id).to_string(), value.parse::<u32>().expect("set")));
            }
            ["OpDecorate", id, "Binding", value] => {
                binding.push(((*id).to_string(), value.parse::<u32>().expect("binding")));
            }
            [id, "=", "OpVariable", _, "UniformConstant"] => {
                uniform_constants.push((*id).to_string());
            }
            _ => {}
        }
    }
    uniform_constants
        .into_iter()
        .filter_map(|id| {
            let set = set.iter().find(|(decorated, _)| *decorated == id)?.1;
            let binding = binding.iter().find(|(decorated, _)| *decorated == id)?.1;
            Some((id, (set, binding)))
        })
        .collect()
}

fn aliased_slots(spirv: &[u8]) -> Vec<(u32, u32)> {
    let slots = descriptor_slots(spirv);
    let mut aliased = Vec::new();
    for (index, (_, slot)) in slots.iter().enumerate() {
        if slots[..index].iter().any(|(_, earlier)| earlier == slot) && !aliased.contains(slot) {
            aliased.push(*slot);
        }
    }
    aliased
}

fn opcode_count(spirv: &[u8], opcode: &str) -> usize {
    disassemble(spirv)
        .expect("disassemble the translated module")
        .lines()
        .filter(|line| line.split_whitespace().any(|token| token == opcode))
        .count()
}

#[test]
fn an_absent_texture_operand_does_not_take_the_only_live_texture() {
    for (label, air) in [
        ("summed", ABSENT_BESIDE_ONE_LIVE),
        ("specialized", ABSENT_BESIDE_ONE_LIVE_SPECIALIZED),
    ] {
        let (spirv, _) = translate_sanitized_native_reflected(
            air,
            Stage::Kernel,
            &scratch(&format!("absent_{label}")),
            TransformOptions::default(),
        )
        .unwrap_or_else(|error| panic!("the {label} absent-texture kernel translates: {error}"));

        assert_eq!(
            opcode_count(&spirv, "OpImageFetch"),
            1,
            "{label}: the absent read was answered by an image instead of zero:\n{}",
            disassemble(&spirv).expect("disassemble")
        );
        assert_eq!(
            opcode_count(&spirv, "OpImageWrite"),
            1,
            "{label}: the absent store landed on an image instead of nowhere:\n{}",
            disassemble(&spirv).expect("disassemble")
        );
    }
}

#[test]
fn a_summed_slot_the_variant_does_not_reach_takes_no_descriptor() {
    let (spirv, reflection) = translate_sanitized_native_reflected(
        SUMMED_SLOTS,
        Stage::Kernel,
        &scratch("summed"),
        TransformOptions::default(),
    )
    .expect("the summed-slot kernel translates");

    assert_eq!(
        aliased_slots(&spirv),
        Vec::new(),
        "two resources share a descriptor slot: {:?}",
        descriptor_slots(&spirv)
    );
    let images = reflection
        .bindings
        .iter()
        .filter(|resource| {
            matches!(
                resource.kind,
                ResourceKind::Texture | ResourceKind::TextureArray | ResourceKind::StorageImage
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        images.len(),
        1,
        "only the enabled alternative is a descriptor, got {images:?}"
    );
    assert_eq!(images[0].metal_index, 0);
}

#[test]
fn a_store_through_a_texture_the_variant_lacks_is_dropped() {
    let (spirv, _reflection) = translate_sanitized_native_reflected(
        SUMMED_SLOTS,
        Stage::Kernel,
        &scratch("summed_write"),
        TransformOptions::default(),
    )
    .expect("the summed-slot kernel translates");

    assert_eq!(
        opcode_count(&spirv, "OpImageWrite"),
        1,
        "the enabled alternative's store is the only one left:\n{}",
        disassemble(&spirv).expect("disassemble")
    );
}

#[test]
fn a_literal_slot_keeps_its_descriptor_when_the_gate_is_off() {
    let (spirv, reflection) = translate_sanitized_native_reflected(
        LITERAL_SLOTS,
        Stage::Kernel,
        &scratch("literal"),
        TransformOptions::default(),
    )
    .expect("the literal-slot kernel translates");

    let images = reflection
        .bindings
        .iter()
        .filter(|resource| {
            matches!(
                resource.kind,
                ResourceKind::Texture | ResourceKind::TextureArray | ResourceKind::StorageImage
            )
        })
        .count();
    assert_eq!(
        images, 2,
        "both stated alternatives are descriptors, got {:?}",
        reflection.bindings
    );
    assert_eq!(
        aliased_slots(&spirv).len(),
        1,
        "the two alternatives share the slot Metal gave them: {:?}",
        descriptor_slots(&spirv)
    );
}

#[test]
fn a_query_on_a_loaded_handle_the_variant_lacks_answers_for_an_absent_resource() {
    let (spirv, _reflection) = translate_sanitized_native_reflected(
        LOADED_HANDLE,
        Stage::Kernel,
        &scratch("loaded_handle"),
        TransformOptions::default(),
    )
    .expect("the loaded-handle kernel translates");

    assert_eq!(
        opcode_count(&spirv, "OpImageQuerySizeLod"),
        1,
        "only the live texture is queried:\n{}",
        disassemble(&spirv).expect("disassemble")
    );
    assert_eq!(
        aliased_slots(&spirv),
        Vec::new(),
        "two resources share a descriptor slot: {:?}",
        descriptor_slots(&spirv)
    );
}
