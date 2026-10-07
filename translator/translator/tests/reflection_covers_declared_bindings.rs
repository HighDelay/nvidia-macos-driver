use metal2vulkan::meta::TextureDimension;
use metal2vulkan::passes::{Stage, TransformOptions};
use metal2vulkan::reflect::{
    DescriptorBindingRange, DescriptorLayout, ResourceKind, ShaderReflection,
    DEFAULT_DESCRIPTOR_LAYOUT, SAMPLER_BINDING_RANGE, TEXTURE_BINDING_RANGE,
};
use metal2vulkan::translate_sanitized_native_reflected;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn bindings_the_module_declares(spirv: &[u8]) -> BTreeSet<(u32, u32)> {
    let text = metal2vulkan::disassemble(spirv).expect("disassemble the translated module");
    let mut sets: Vec<(String, u32)> = Vec::new();
    let mut bindings: Vec<(String, u32)> = Vec::new();
    for line in text.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let ["OpDecorate", target, decoration, value] = tokens.as_slice() else {
            continue;
        };
        let Ok(value) = value.parse::<u32>() else {
            continue;
        };
        match *decoration {
            "DescriptorSet" => sets.push(((*target).to_string(), value)),
            "Binding" => bindings.push(((*target).to_string(), value)),
            _ => {}
        }
    }
    bindings
        .into_iter()
        .filter_map(|(target, binding)| {
            sets.iter()
                .find(|(other, _)| *other == target)
                .map(|(_, set)| (*set, binding))
        })
        .collect()
}

fn bindings_reflection_reports(reflection: &ShaderReflection) -> BTreeSet<(u32, u32)> {
    let set = reflection.descriptor_layout.set;
    let mut reported = BTreeSet::new();
    for resource in &reflection.bindings {
        if let Some(location) = &resource.descriptor {
            reported.insert((location.set, location.binding));
        }
    }
    for attachment in &reflection.implicit_imageblock_attachments {
        reported.insert((set, attachment.binding));
    }
    for member in reflection
        .fragment_imageblock
        .iter()
        .flat_map(|imageblock| &imageblock.members)
    {
        if let Some(binding) = member.binding {
            reported.insert((set, binding));
        }
    }
    reported
}

fn scratch(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "m2v_reflected_bindings_{}_{}",
        std::process::id(),
        label.replace(['/', '.'], "_")
    ));
    std::fs::create_dir_all(&directory).expect("scratch directory");
    directory
}

fn assert_reflection_covers_declarations(
    label: &str,
    spirv: &[u8],
    reflection: &ShaderReflection,
) -> BTreeSet<(u32, u32)> {
    let declared = bindings_the_module_declares(spirv);
    let reported = bindings_reflection_reports(reflection);
    let unreported = declared.difference(&reported).collect::<Vec<_>>();
    assert!(
        unreported.is_empty(),
        "{label} decorates {unreported:?} on a variable but reflection does not report those \
         bindings, so a descriptor-set layout built from the reflection would not cover the module"
    );
    declared
}

const SAMPLER_LESS_TEXTURE_READ: &str = r#"target triple = "spirv-unknown-vulkan1.2"

define void @copy_texel(ptr addrspace(1) %src, ptr addrspace(1) %dst, <2 x i32> %gid) {
entry:
  %sampler = tail call ptr addrspace(2) @air.get_read_sampler()
  %read = tail call { <4 x half>, i8 } @air.read_texture_2d_array.v4f16(ptr addrspace(1) %src, ptr addrspace(2) %sampler, <2 x i32> %gid, i32 0, <2 x i32> zeroinitializer, i32 0, i32 0)
  %texel = extractvalue { <4 x half>, i8 } %read, 0
  tail call void @air.write_texture_2d_array.v4f16(ptr addrspace(1) %dst, <2 x i32> %gid, i32 0, <4 x half> %texel, i32 0, i32 2)
  ret void
}

declare ptr addrspace(2) @air.get_read_sampler()
declare { <4 x half>, i8 } @air.read_texture_2d_array.v4f16(ptr addrspace(1), ptr addrspace(2), <2 x i32>, i32, <2 x i32>, i32, i32)
declare void @air.write_texture_2d_array.v4f16(ptr addrspace(1), <2 x i32>, i32, <4 x half>, i32, i32)

!air.kernel = !{!0}
!0 = !{ptr @copy_texel, !1, !2}
!1 = !{}
!2 = !{!3, !4, !5}
!3 = !{i32 0, !"air.texture", !"air.location_index", i32 1, i32 1, !"air.sample", !"air.arg_type_name", !"texture2d_array<half, sample>", !"air.arg_name", !"src"}
!4 = !{i32 1, !"air.texture", !"air.location_index", i32 2, i32 1, !"air.write", !"air.arg_type_name", !"texture2d_array<half, write>", !"air.arg_name", !"dst"}
!5 = !{i32 2, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint2", !"air.arg_name", !"gid"}
"#;

#[test]
fn a_sampler_less_texture_read_needs_no_sampler_descriptor() {
    let (spirv, reflection) = translate_sanitized_native_reflected(
        SAMPLER_LESS_TEXTURE_READ,
        Stage::Kernel,
        &scratch("sampler_less_texture_read"),
        TransformOptions::default(),
    )
    .expect("the read kernel translates");

    let declared = assert_reflection_covers_declarations("the read kernel", &spirv, &reflection);
    let samplers = declared
        .iter()
        .filter(|(_, binding)| SAMPLER_BINDING_RANGE.contains(*binding))
        .collect::<Vec<_>>();
    assert!(
        samplers.is_empty(),
        "the kernel reads a texture and never samples one, so it must not demand a sampler \
         descriptor; it declares {samplers:?}"
    );
    let text = metal2vulkan::disassemble(&spirv).expect("disassemble");
    assert!(
        !text.contains("OpTypeSampler"),
        "no sampler is bound, so no sampler type should survive:\n{text}"
    );
    assert_eq!(declared.len(), 2, "one sampled and one write texture");
}

const UNCONSUMED_NULL_TEXTURE: &str = r#"target triple = "spirv-unknown-vulkan1.2"

define void @probe_optional_attachment(ptr addrspace(1) %out) {
entry:
  %tex = call ptr addrspace(1) @air.get_null_texture_2d()
  %isnull = call i1 @air.is_null_texture_2d(ptr addrspace(1) %tex)
  %flag = zext i1 %isnull to i32
  store i32 %flag, ptr addrspace(1) %out, align 4
  ret void
}

declare ptr addrspace(1) @air.get_null_texture_2d()
declare i1 @air.is_null_texture_2d(ptr addrspace(1))

!air.kernel = !{!0}
!0 = !{ptr @probe_optional_attachment, !1, !2}
!1 = !{}
!2 = !{!3}
!3 = !{i32 0, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 0, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
"#;

#[test]
fn a_placeholder_texture_nothing_reads_needs_no_texture_descriptor() {
    let (spirv, reflection) = translate_sanitized_native_reflected(
        UNCONSUMED_NULL_TEXTURE,
        Stage::Kernel,
        &scratch("unconsumed_null_texture"),
        TransformOptions::default(),
    )
    .expect("the optional-attachment probe translates");

    let declared =
        assert_reflection_covers_declarations("the optional-attachment probe", &spirv, &reflection);
    let textures = declared
        .iter()
        .filter(|(_, binding)| TEXTURE_BINDING_RANGE.contains(*binding))
        .collect::<Vec<_>>();
    assert!(
        textures.is_empty(),
        "the kernel only asks whether the placeholder is bound and never reads it, so it must not \
         demand a texture descriptor; it declares {textures:?}"
    );
    let text = metal2vulkan::disassemble(&spirv).expect("disassemble");
    assert!(
        !text.contains("OpTypeImage"),
        "no image is bound, so no image type should survive:\n{text}"
    );
    assert_eq!(declared.len(), 1, "the one output buffer");
}

const CONSUMED_NULL_TEXTURE: &str = r#"target triple = "spirv-unknown-vulkan1.2"

define void @measure_optional_attachment(ptr addrspace(1) %out) {
entry:
  %tex = call ptr addrspace(1) @air.get_null_texture_2d()
  %width = call i32 @air.get_width_texture_2d(ptr addrspace(1) %tex, i32 0)
  store i32 %width, ptr addrspace(1) %out, align 4
  ret void
}

declare ptr addrspace(1) @air.get_null_texture_2d()
declare i32 @air.get_width_texture_2d(ptr addrspace(1), i32)

!air.kernel = !{!0}
!0 = !{ptr @measure_optional_attachment, !1, !2}
!1 = !{}
!2 = !{!3}
!3 = !{i32 0, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 0, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
"#;

#[test]
fn a_placeholder_texture_the_shader_reads_is_reported_as_a_descriptor() {
    let (spirv, reflection) = translate_sanitized_native_reflected(
        CONSUMED_NULL_TEXTURE,
        Stage::Kernel,
        &scratch("consumed_null_texture"),
        TransformOptions::default(),
    )
    .expect("the optional-attachment measurement translates");

    let declared = assert_reflection_covers_declarations(
        "the optional-attachment measurement",
        &spirv,
        &reflection,
    );
    let placeholder = reflection
        .bindings
        .iter()
        .find(|resource| resource.kind == ResourceKind::SynthesizedNullTexture)
        .expect("the placeholder the module reads through is reported");
    let location = placeholder.descriptor.expect("it consumes a descriptor");
    assert!(
        TEXTURE_BINDING_RANGE.contains(location.binding),
        "a placeholder image belongs in the sampled-texture band, not at {}",
        location.binding
    );
    assert!(
        declared.contains(&(location.set, location.binding)),
        "the reported binding is one the module decorates; module declares {declared:?}"
    );
    let shape = placeholder
        .texture_shape
        .expect("a consumer needs the shape of the image it has to bind");
    assert_eq!(shape.dimension, TextureDimension::D2);
    assert!(!shape.arrayed && !shape.writable);
}

const FRAGMENT_IMPLICIT_IMAGEBLOCK: &str = r#"define <2 x half> @read_back_render_target(<4 x float> %position) {
entry:
  %value = call <2 x half> @air.load.implicit_imageblock.v2f16(i32 0, <2 x i16> zeroinitializer, i32 0, i16 0)
  ret <2 x half> %value
}

declare <2 x half> @air.load.implicit_imageblock.v2f16(i32, <2 x i16>, i32, i16)

!air.fragment = !{!0}
!0 = !{ptr @read_back_render_target, !1, !2}
!1 = !{!3}
!2 = !{!4}
!3 = !{i32 0, !"air.render_target", !"air.location_index", i32 0, i32 0, !"air.arg_type_name", !"half2"}
!4 = !{i32 0, !"air.position", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"position"}
"#;

#[test]
fn a_fragment_shader_reflects_the_imageblock_plane_it_declares() {
    let (spirv, reflection) = translate_sanitized_native_reflected(
        FRAGMENT_IMPLICIT_IMAGEBLOCK,
        Stage::Fragment,
        &scratch("fragment_implicit_imageblock"),
        TransformOptions::default(),
    )
    .expect("the render-target read-back translates");

    let declared = assert_reflection_covers_declarations(
        "the render-target read-back fragment",
        &spirv,
        &reflection,
    );
    assert_eq!(
        reflection.implicit_imageblock_attachments.len(),
        1,
        "the shader loads one implicit imageblock plane"
    );
    assert!(
        declared.contains(&(
            reflection.descriptor_layout.set,
            reflection.implicit_imageblock_attachments[0].binding
        )),
        "the reported plane is the binding the module decorates; module declares {declared:?}"
    );
}

const RUNTIME_TEXTURE_ARRAY: &str = r#"target triple = "spirv-unknown-vulkan1.2"
%"struct.metal::texture2d" = type { ptr addrspace(1) }

define void @k(ptr readonly captures(none) %imgs, ptr addrspace(1) %out) {
entry:
  %tex = load ptr addrspace(1), ptr %imgs, align 8
  %w = tail call i32 @air.get_width_texture_2d(ptr addrspace(1) %tex, i32 0)
  store i32 %w, ptr addrspace(1) %out, align 4
  ret void
}
declare i32 @air.get_width_texture_2d(ptr addrspace(1), i32)
!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.sample", !"air.arg_type_name", !"array_ref<texture2d<float, sample>>", !"air.arg_name", !"imgs"}
!4 = !{i32 1, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 0, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
"#;

#[test]
fn a_runtime_descriptor_array_reports_the_capacity_the_module_declares() {
    let (spirv, reflection) = translate_sanitized_native_reflected(
        RUNTIME_TEXTURE_ARRAY,
        Stage::Kernel,
        &scratch("runtime_texture_array"),
        TransformOptions::default(),
    )
    .expect("the descriptor-array kernel translates");

    assert_reflection_covers_declarations("the descriptor-array kernel", &spirv, &reflection);
    let declared = array_lengths_the_module_declares(&spirv);
    assert!(
        declared.values().any(|length| *length > 1),
        "this fixture is only meaningful if the module declares a descriptor array; got {declared:?}"
    );
    assert_counts_cover_declared_arrays("the descriptor-array kernel", &spirv, &reflection);
}

const FIXED_TEXTURE_ARRAY: &str = r#"target triple = "spirv-unknown-vulkan1.2"
%"struct.metal::texture2d" = type { ptr addrspace(1) }

define void @k(ptr readonly captures(none) %imgs, ptr addrspace(1) %out) {
entry:
  %slot = getelementptr inbounds ptr addrspace(1), ptr %imgs, i32 3
  %tex = load ptr addrspace(1), ptr %slot, align 8
  %w = tail call i32 @air.get_width_texture_2d(ptr addrspace(1) %tex, i32 0)
  store i32 %w, ptr addrspace(1) %out, align 4
  ret void
}
declare i32 @air.get_width_texture_2d(ptr addrspace(1), i32)
!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 4, !"air.sample", !"air.arg_type_name", !"array<texture2d<float, sample>, 4>", !"air.arg_name", !"imgs"}
!4 = !{i32 1, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 0, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
"#;

const FIXED_DEPTH_TEXTURE_ARRAY: &str = r#"target triple = "spirv-unknown-vulkan1.2"
%"struct.metal::depth2d" = type { ptr addrspace(1) }

define void @k(ptr readonly captures(none) %imgs, ptr addrspace(1) %out) {
entry:
  %slot = getelementptr inbounds ptr addrspace(1), ptr %imgs, i32 2
  %tex = load ptr addrspace(1), ptr %slot, align 8
  %w = tail call i32 @air.get_width_depth_2d(ptr addrspace(1) %tex, i32 0)
  store i32 %w, ptr addrspace(1) %out, align 4
  ret void
}
declare i32 @air.get_width_depth_2d(ptr addrspace(1), i32)
!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 3, !"air.sample", !"air.arg_type_name", !"array<depth2d<float, sample>, 3>", !"air.arg_name", !"imgs"}
!4 = !{i32 1, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 0, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
"#;

#[test]
fn a_fixed_descriptor_array_reports_its_declared_length_not_the_ceiling() {
    for (label, source, declared_length) in [
        ("the fixed texture-array kernel", FIXED_TEXTURE_ARRAY, 4),
        ("the fixed depth-array kernel", FIXED_DEPTH_TEXTURE_ARRAY, 3),
    ] {
        let (spirv, reflection) = translate_sanitized_native_reflected(
            source,
            Stage::Kernel,
            &scratch(label),
            TransformOptions::default(),
        )
        .unwrap_or_else(|error| panic!("{label} translates: {error}"));

        assert_reflection_covers_declarations(label, &spirv, &reflection);

        let declared = array_lengths_the_module_declares(&spirv);
        assert!(
            declared.values().any(|length| *length == declared_length),
            "{label} should declare an OpTypeArray of {declared_length}; got {declared:?}"
        );

        assert_eq!(
            assert_counts_cover_declared_arrays(label, &spirv, &reflection),
            declared.len(),
            "{label} left a declared binding uncompared"
        );

        let array_counts = reflection
            .bindings
            .iter()
            .filter(|resource| resource.kind == metal2vulkan::reflect::ResourceKind::TextureArray)
            .filter_map(|resource| resource.descriptor.map(|location| location.count))
            .collect::<Vec<_>>();
        assert_eq!(
            array_counts,
            vec![declared_length],
            "{label} should report exactly the declared array length"
        );
    }
}

fn assert_counts_cover_declared_arrays(
    label: &str,
    spirv: &[u8],
    reflection: &ShaderReflection,
) -> usize {
    let mut reported = std::collections::BTreeMap::<u32, u32>::new();
    for location in reflection
        .bindings
        .iter()
        .filter_map(|resource| resource.descriptor)
    {
        let slot = reported.entry(location.binding).or_default();
        *slot = (*slot).max(location.count);
    }
    let declared = array_lengths_the_module_declares(spirv);
    let mut checked = 0;
    for (binding, length) in &declared {
        let Some(count) = reported.get(binding) else {
            continue;
        };
        assert_eq!(
            count, length,
            "{label} reports {count} descriptor(s) at binding {binding} but the module declares an \
             array of {length} there; a layout built from the reflection is the wrong size for the \
             array the shader indexes"
        );
        checked += 1;
    }
    checked
}

fn array_lengths_the_module_declares(spirv: &[u8]) -> std::collections::BTreeMap<u32, u32> {
    let text = metal2vulkan::disassemble(spirv).expect("disassemble the translated module");
    let mut constants: Vec<(String, u32)> = Vec::new();
    let mut arrays: Vec<(String, String)> = Vec::new();
    let mut pointees: Vec<(String, String)> = Vec::new();
    let mut variables: Vec<(String, String)> = Vec::new();
    let mut bindings: Vec<(String, u32)> = Vec::new();
    for line in text.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        match tokens.as_slice() {
            ["OpDecorate", target, "Binding", value] => {
                if let Ok(value) = value.parse::<u32>() {
                    bindings.push(((*target).to_string(), value));
                }
            }
            [result, "=", "OpConstant", _, value] => {
                if let Ok(value) = value.parse::<u32>() {
                    constants.push(((*result).to_string(), value));
                }
            }
            [result, "=", "OpTypeArray", _, length] => {
                arrays.push(((*result).to_string(), (*length).to_string()));
            }
            [result, "=", "OpTypePointer", _, pointee] => {
                pointees.push(((*result).to_string(), (*pointee).to_string()));
            }
            [result, "=", "OpVariable", pointer, _] => {
                variables.push(((*result).to_string(), (*pointer).to_string()));
            }
            _ => {}
        }
    }
    let find = |pairs: &Vec<(String, String)>, key: &str| {
        pairs
            .iter()
            .find(|(id, _)| id == key)
            .map(|(_, value)| value.clone())
    };
    let mut declared = std::collections::BTreeMap::<u32, u32>::new();
    for (variable, binding) in bindings {
        let Some(pointer) = find(&variables, &variable) else {
            continue;
        };
        let Some(pointee) = find(&pointees, &pointer) else {
            continue;
        };
        let length = match find(&arrays, &pointee) {
            Some(length) => match constants.iter().find(|(id, _)| *id == length) {
                Some((_, value)) => *value,
                None => continue,
            },
            None => 1,
        };
        let slot = declared.entry(binding).or_insert(1);
        *slot = (*slot).max(length);
    }
    declared
}

#[test]
fn every_public_fixture_reflects_every_descriptor_it_declares() {
    let mut declared = 0;
    let mut checked = 0;
    for path in public_fixtures() {
        let source = std::fs::read_to_string(&path).expect("read fixture");
        let label = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(stage) = stage_of(&source) else {
            continue;
        };
        let Ok((spirv, reflection)) = translate_sanitized_native_reflected(
            &source,
            stage,
            &scratch(&label),
            TransformOptions::default(),
        ) else {
            continue;
        };
        declared += assert_reflection_covers_declarations(&label, &spirv, &reflection).len();
        assert_counts_cover_declared_arrays(&label, &spirv, &reflection);
        assert_invented_descriptors_are_declared(&label, &spirv, &reflection);
        checked += 1;
    }
    assert!(
        checked >= 20 && declared >= 20,
        "only {checked} fixtures declaring {declared} bindings were inspected, so this swept \
         almost nothing"
    );
}

const CONSTEXPR_SAMPLERS: &str = r#"target triple = "spirv-unknown-vulkan1.2"

@__air_sampler_state.118 = internal addrspace(2) constant i64 -9188470239253755319, align 8
@__air_sampler_state.119 = internal addrspace(2) constant i64 -9188470239253757806, align 8

define <4 x float> @frag(<4 x float> %position, <2 x float> %coord, ptr addrspace(1) %tex, ptr addrspace(2) %runtime_sampler) {
entry:
  %sample0 = tail call { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1) %tex, ptr addrspace(2) @__air_sampler_state.118, <2 x float> %coord, i1 true, <2 x i32> zeroinitializer, i1 false, float 0.000000e+00, float 0.000000e+00, i32 0)
  %value0 = extractvalue { <4 x float>, i8 } %sample0, 0
  %sample1 = tail call { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1) %tex, ptr addrspace(2) @__air_sampler_state.119, <2 x float> %coord, i1 true, <2 x i32> zeroinitializer, i1 false, float 0.000000e+00, float 0.000000e+00, i32 0)
  %value1 = extractvalue { <4 x float>, i8 } %sample1, 0
  %value = fadd <4 x float> %value0, %value1
  ret <4 x float> %value
}

declare { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1), ptr addrspace(2), <2 x float>, i1, <2 x i32>, i1, float, float, i32)

!air.fragment = !{!0}
!air.sampler_states = !{!9, !8}
!0 = !{ptr @frag, !1, !3}
!1 = !{!2}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!4, !5, !6, !7}
!4 = !{i32 0, !"air.position", !"air.center", !"air.arg_type_name", !"float4"}
!5 = !{i32 1, !"air.fragment_input", !"generated(coord)", !"air.center", !"air.perspective", !"air.arg_type_name", !"float2"}
!6 = !{i32 2, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.sample", !"air.arg_type_name", !"texture2d<float, sample>"}
!7 = !{i32 3, !"air.sampler", !"air.location_index", i32 0, i32 1}
!8 = !{!"air.sampler_state", ptr addrspace(2) @__air_sampler_state.118}
!9 = !{!"air.sampler_state", ptr addrspace(2) @__air_sampler_state.119}
"#;

#[test]
fn constexpr_samplers_are_reported_on_the_bindings_the_module_gives_them() {
    for (label, layout) in [
        ("default", DEFAULT_DESCRIPTOR_LAYOUT),
        ("shifted", shifted_layout()),
    ] {
        let options = TransformOptions::default()
            .with_descriptor_layout(layout)
            .expect("layout");
        let (spirv, reflection) = translate_sanitized_native_reflected(
            CONSTEXPR_SAMPLERS,
            Stage::Fragment,
            &scratch(&format!("constexpr_samplers_{label}")),
            options,
        )
        .expect("translate constexpr samplers");

        assert_reflection_covers_declarations(label, &spirv, &reflection);
        let static_samplers = reflection
            .bindings
            .iter()
            .filter(|binding| binding.kind == ResourceKind::StaticSampler)
            .filter_map(|binding| binding.descriptor.map(|descriptor| descriptor.binding))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            static_samplers.len(),
            2,
            "{label}: both constexpr samplers must be reported, got {static_samplers:?}"
        );
        for binding in static_samplers {
            assert!(
                layout.samplers.contains(binding),
                "{label}: constexpr sampler reported at {binding}, outside the selected sampler \
                 band [{}, {})",
                layout.samplers.start,
                layout.samplers.end
            );
        }
    }
}

#[test]
fn every_public_fixture_reflects_every_descriptor_it_declares_under_a_shifted_layout() {
    let layout = shifted_layout();
    let mut checked = 0;
    let mut declared = 0;
    for path in public_fixtures() {
        let source = std::fs::read_to_string(&path).expect("read fixture");
        let label = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(stage) = stage_of(&source) else {
            continue;
        };
        let options = TransformOptions::default()
            .with_descriptor_layout(layout)
            .expect("shifted layout");
        let Ok((spirv, reflection)) = translate_sanitized_native_reflected(
            &source,
            stage,
            &scratch(&format!("shifted_{label}")),
            options,
        ) else {
            continue;
        };
        assert_eq!(
            reflection.descriptor_layout, layout,
            "{label} reports a layout other than the one it was translated with"
        );
        let module = assert_reflection_covers_declarations(&label, &spirv, &reflection);
        assert_counts_cover_declared_arrays(&label, &spirv, &reflection);
        assert_invented_descriptors_are_declared(&label, &spirv, &reflection);
        for (set, binding) in module
            .iter()
            .copied()
            .chain(bindings_reflection_reports(&reflection))
        {
            assert_eq!(
                set, layout.set,
                "{label} places binding {binding} on set {set}, not the selected set"
            );
            assert!(
                selected_bands(layout)
                    .iter()
                    .any(|band| band.contains(binding))
                    || reflection.bindings.iter().any(|resource| {
                        resource.kind == ResourceKind::RayInstanceUserIdTable
                            && binding == metal2vulkan::reflect::RAY_INSTANCE_USER_ID_TABLE_BINDING
                            && resource.descriptor.is_some_and(|location| {
                                location.set == set && location.binding == binding
                            })
                    }),
                "{label} uses binding {binding}, which is in no band of the selected layout -- a \
                 default-layout number that survived the shift"
            );
        }
        declared += module.len();
        checked += 1;
    }
    assert!(
        checked >= 20 && declared >= 20,
        "only {checked} fixtures declaring {declared} bindings were inspected, so this swept \
         almost nothing"
    );
}

fn shifted_layout() -> DescriptorLayout {
    const OFFSET: u32 = 4096;
    let shift = |range: DescriptorBindingRange| DescriptorBindingRange {
        start: range.start + OFFSET,
        end: range.end + OFFSET,
    };
    let default = DEFAULT_DESCRIPTOR_LAYOUT;
    DescriptorLayout {
        set: default.set + 3,
        buffers: shift(default.buffers),
        sampled_textures: shift(default.sampled_textures),
        samplers: shift(default.samplers),
        color_inputs: shift(default.color_inputs),
        imageblocks: shift(default.imageblocks),
        fragment_imageblocks: shift(default.fragment_imageblocks),
        storage_textures: shift(default.storage_textures),
        synthetic: shift(default.synthetic),
        ..default
    }
}

fn selected_bands(layout: DescriptorLayout) -> [DescriptorBindingRange; 8] {
    [
        layout.buffers,
        layout.sampled_textures,
        layout.samplers,
        layout.color_inputs,
        layout.imageblocks,
        layout.fragment_imageblocks,
        layout.storage_textures,
        layout.synthetic,
    ]
}

fn stage_of(source: &str) -> Option<Stage> {
    if source.contains("!air.vertex =") {
        Some(Stage::Vertex)
    } else if source.contains("!air.fragment =") {
        Some(Stage::Fragment)
    } else if source.contains("!air.kernel =") {
        Some(Stage::Kernel)
    } else {
        None
    }
}

fn public_fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("validation/fixtures/public");
    let mut paths = std::fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("read {}: {error}", root.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ll"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

const RE_UNIQUED_CONSTEXPR_SAMPLERS: &str = r#"target triple = "spirv-unknown-vulkan1.2"

@__air_sampler_state = internal addrspace(2) constant i64 -9188470239253755319, align 8
@__air_sampler_state.118.9 = internal addrspace(2) constant i64 -9188470239253757806, align 8

define <4 x float> @frag(<4 x float> %position, <2 x float> %coord, ptr addrspace(1) %tex, ptr addrspace(2) %runtime_sampler) {
entry:
  %sample0 = tail call { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1) %tex, ptr addrspace(2) @__air_sampler_state, <2 x float> %coord, i1 true, <2 x i32> zeroinitializer, i1 false, float 0.000000e+00, float 0.000000e+00, i32 0)
  %value0 = extractvalue { <4 x float>, i8 } %sample0, 0
  %sample1 = tail call { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1) %tex, ptr addrspace(2) @__air_sampler_state.118.9, <2 x float> %coord, i1 true, <2 x i32> zeroinitializer, i1 false, float 0.000000e+00, float 0.000000e+00, i32 0)
  %value1 = extractvalue { <4 x float>, i8 } %sample1, 0
  %value = fadd <4 x float> %value0, %value1
  ret <4 x float> %value
}
declare { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1), ptr addrspace(2), <2 x float>, i1, <2 x i32>, i1, float, float, i32)

!air.fragment = !{!0}
!air.sampler_states = !{!9, !8}
!0 = !{ptr @frag, !1, !3}
!1 = !{!2}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!4, !5, !6, !7}
!4 = !{i32 0, !"air.position", !"air.center", !"air.arg_type_name", !"float4"}
!5 = !{i32 1, !"air.fragment_input", !"generated(coord)", !"air.center", !"air.perspective", !"air.arg_type_name", !"float2"}
!6 = !{i32 2, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.sample", !"air.arg_type_name", !"texture2d<float, sample>"}
!7 = !{i32 3, !"air.sampler", !"air.location_index", i32 0, i32 1}
!8 = !{!"air.sampler_state", ptr addrspace(2) @__air_sampler_state}
!9 = !{!"air.sampler_state", ptr addrspace(2) @__air_sampler_state.118.9}
"#;

const CONSTEXPR_SAMPLER_WORDS: [u64; 2] = [
    (-9188470239253755319_i64) as u64,
    (-9188470239253757806_i64) as u64,
];

fn sampler_bindings_in_sample_order(spirv: &[u8]) -> Vec<u32> {
    let text = metal2vulkan::disassemble(spirv).expect("disassemble the translated module");
    let mut bindings = std::collections::HashMap::<String, u32>::new();
    let mut loaded_from = std::collections::HashMap::<String, String>::new();
    let mut order = Vec::new();
    for line in text.lines() {
        match line.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["OpDecorate", target, "Binding", value] => {
                if let Ok(value) = value.parse::<u32>() {
                    bindings.insert((*target).to_string(), value);
                }
            }
            [result, "=", "OpLoad", _, source] => {
                loaded_from.insert((*result).to_string(), (*source).to_string());
            }
            [_, "=", "OpSampledImage", _, _, sampler] => {
                let variable = loaded_from.get(*sampler).cloned().unwrap_or_default();
                if let Some(binding) = bindings.get(&variable) {
                    order.push(*binding);
                }
            }
            _ => {}
        }
    }
    order
}

#[test]
fn each_constexpr_sampler_binding_reports_the_state_its_sample_reads() {
    for (label, source) in [
        ("uniquely suffixed", CONSTEXPR_SAMPLERS),
        ("re-uniqued", RE_UNIQUED_CONSTEXPR_SAMPLERS),
    ] {
        let (spirv, reflection) = translate_sanitized_native_reflected(
            source,
            Stage::Fragment,
            &scratch(&format!(
                "sampler_state_pairing_{}",
                label.replace(' ', "_")
            )),
            TransformOptions::default(),
        )
        .unwrap_or_else(|error| panic!("{label} constexpr samplers translate: {error}"));

        let reported = reflection
            .bindings
            .iter()
            .filter(|binding| binding.kind == ResourceKind::StaticSampler)
            .filter_map(|binding| {
                Some((
                    binding.descriptor?.binding,
                    binding.static_sampler?.raw_words[0],
                ))
            })
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(reported.len(), 2, "{label}: both samplers reported");

        let sampled = sampler_bindings_in_sample_order(&spirv);
        assert_eq!(
            sampled.len(),
            2,
            "{label}: both samples must reach a static-sampler binding, got {sampled:?}"
        );
        for (position, (binding, expected)) in sampled
            .iter()
            .zip(CONSTEXPR_SAMPLER_WORDS)
            .map(|(binding, word)| (*binding, word))
            .enumerate()
        {
            assert_eq!(
                reported.get(&binding).copied(),
                Some(expected),
                "{label}: sample {position} reads binding {binding}, where AIR put \
                 {expected:#x}, but reflection describes that binding as \
                 {:#x?}",
                reported.get(&binding)
            );
        }
    }
}

const UNREAD_CONSTEXPR_SAMPLER: &str = r#"target triple = "spirv-unknown-vulkan1.2"

@__air_sampler_state.117 = internal addrspace(2) constant i64 -9188470239253747127, align 8
@__air_sampler_state.118 = internal addrspace(2) constant i64 -9188470239253755319, align 8
@__air_sampler_state.119 = internal addrspace(2) constant i64 -9188470239253757806, align 8

define <4 x float> @frag(<4 x float> %position, <2 x float> %coord, ptr addrspace(1) %tex, ptr addrspace(2) %runtime_sampler) {
entry:
  %sample0 = tail call { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1) %tex, ptr addrspace(2) @__air_sampler_state.118, <2 x float> %coord, i1 true, <2 x i32> zeroinitializer, i1 false, float 0.000000e+00, float 0.000000e+00, i32 0)
  %value0 = extractvalue { <4 x float>, i8 } %sample0, 0
  %sample1 = tail call { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1) %tex, ptr addrspace(2) @__air_sampler_state.119, <2 x float> %coord, i1 true, <2 x i32> zeroinitializer, i1 false, float 0.000000e+00, float 0.000000e+00, i32 0)
  %value1 = extractvalue { <4 x float>, i8 } %sample1, 0
  %value = fadd <4 x float> %value0, %value1
  ret <4 x float> %value
}
declare { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1), ptr addrspace(2), <2 x float>, i1, <2 x i32>, i1, float, float, i32)

!air.fragment = !{!0}
!air.sampler_states = !{!8, !9, !10}
!0 = !{ptr @frag, !1, !3}
!1 = !{!2}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!3 = !{!4, !5, !6, !7}
!4 = !{i32 0, !"air.position", !"air.center", !"air.arg_type_name", !"float4"}
!5 = !{i32 1, !"air.fragment_input", !"generated(coord)", !"air.center", !"air.perspective", !"air.arg_type_name", !"float2"}
!6 = !{i32 2, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.sample", !"air.arg_type_name", !"texture2d<float, sample>"}
!7 = !{i32 3, !"air.sampler", !"air.location_index", i32 0, i32 1}
!8 = !{!"air.sampler_state", ptr addrspace(2) @__air_sampler_state.117}
!9 = !{!"air.sampler_state", ptr addrspace(2) @__air_sampler_state.118}
!10 = !{!"air.sampler_state", ptr addrspace(2) @__air_sampler_state.119}
"#;

const TRANSLATOR_INVENTED_KINDS: [ResourceKind; 4] = [
    ResourceKind::StaticSampler,
    ResourceKind::BufferAddressTable,
    ResourceKind::SynthesizedNullTexture,
    ResourceKind::SynthesizedReadSampler,
];

fn assert_invented_descriptors_are_declared(
    label: &str,
    spirv: &[u8],
    reflection: &ShaderReflection,
) -> usize {
    let declared = bindings_the_module_declares(spirv);
    let mut checked = 0;
    for resource in reflection
        .bindings
        .iter()
        .filter(|resource| TRANSLATOR_INVENTED_KINDS.contains(&resource.kind))
    {
        let Some(location) = resource.descriptor else {
            continue;
        };
        assert!(
            declared.contains(&(location.set, location.binding)),
            "{label} reports {:?} at ({}, {}), which the module declares no variable at, so a \
             consumer would create a descriptor nothing can read",
            resource.kind,
            location.set,
            location.binding
        );
        checked += 1;
    }
    checked
}

#[test]
fn a_constexpr_sampler_no_sample_reads_is_not_reported() {
    let (spirv, reflection) = translate_sanitized_native_reflected(
        UNREAD_CONSTEXPR_SAMPLER,
        Stage::Fragment,
        &scratch("unread_constexpr_sampler"),
        TransformOptions::default(),
    )
    .expect("the three-sampler fragment translates");

    assert_reflection_covers_declarations("the unread-sampler fragment", &spirv, &reflection);
    assert_eq!(
        assert_invented_descriptors_are_declared(
            "the unread-sampler fragment",
            &spirv,
            &reflection
        ),
        2,
        "the two sampled states are reported and the unread one is not"
    );

    let reported = reflection
        .bindings
        .iter()
        .filter(|binding| binding.kind == ResourceKind::StaticSampler)
        .filter_map(|binding| {
            Some((
                binding.descriptor?.binding,
                binding.static_sampler?.raw_words[0],
            ))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let sampled = sampler_bindings_in_sample_order(&spirv);
    assert_eq!(sampled.len(), 2, "both samples reach a static sampler");
    for (binding, expected) in sampled.iter().zip(CONSTEXPR_SAMPLER_WORDS) {
        assert_eq!(reported.get(binding).copied(), Some(expected));
    }
}

#[test]
fn native_ray_instance_user_id_table_has_its_own_reserved_contract() {
    let ll = include_str!("../validation/fixtures/public/kernel_instance_as_intersect.ll");
    for layout in [DEFAULT_DESCRIPTOR_LAYOUT, shifted_layout()] {
        let options = TransformOptions::default()
            .with_descriptor_layout(layout)
            .unwrap();
        let (spv, reflection) = translate_sanitized_native_reflected(
            ll,
            Stage::Kernel,
            &scratch(&format!("ray_user_ids_{}", layout.set)),
            options,
        )
        .unwrap();
        let tables = reflection
            .bindings
            .iter()
            .filter(|r| r.kind == ResourceKind::RayInstanceUserIdTable)
            .collect::<Vec<_>>();
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0]
                .descriptor
                .as_ref()
                .map(|d| (d.set, d.binding, d.count)),
            Some((layout.set, 31, 1))
        );
        assert_eq!(tables[0].param_index, None);
        assert_eq!(
            tables[0].access,
            Some(metal2vulkan::reflect::ResourceAccess::ReadOnly)
        );
        assert!(!reflection
            .bindings
            .iter()
            .any(|r| r.kind == ResourceKind::BufferAddressTable
                && r.descriptor.is_some_and(|d| d.binding == 31)));
        assert_reflection_covers_declarations("ray user-ID table", &spv, &reflection);
    }
}

#[test]
fn native_ray_instance_user_id_table_refuses_a_real_buffer_collision() {
    let ll = include_str!("../validation/fixtures/public/kernel_instance_as_intersect.ll").replace(
        "!\"air.location_index\", i32 0, i32 1",
        "!\"air.location_index\", i32 31, i32 1",
    );
    let error = translate_sanitized_native_reflected(
        &ll,
        Stage::Kernel,
        &scratch("ray_user_ids_collision"),
        TransformOptions::default(),
    )
    .unwrap_err();
    assert!(error.contains("31") && error.contains("collid"), "{error}");
}
