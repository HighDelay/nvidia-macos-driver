use metal2vulkan::meta::{TextureComponent, TextureDimension, TextureShape};
use metal2vulkan::passes::{Stage, TransformOptions};
use metal2vulkan::reflect::ShaderReflection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CUBE_TEXEL_COPY: &str = r#"target triple = "spirv-unknown-vulkan1.2"

define void @copy_cube_face(ptr addrspace(1) %src, ptr addrspace(1) %dst, <2 x i32> %coord, i32 %face) {
entry:
  %sampler = tail call ptr addrspace(2) @air.get_read_sampler()
  %read = tail call { <4 x float>, i8 } @air.read_texture_cube.v4f32(ptr addrspace(1) %src, ptr addrspace(2) %sampler, <2 x i32> %coord, i32 %face, i32 0, i32 1)
  %texel = extractvalue { <4 x float>, i8 } %read, 0
  tail call void @air.write_texture_cube.v4f32(ptr addrspace(1) %dst, <2 x i32> %coord, i32 %face, <4 x float> %texel, i32 0, i32 2)
  ret void
}

declare ptr addrspace(2) @air.get_read_sampler()
declare { <4 x float>, i8 } @air.read_texture_cube.v4f32(ptr addrspace(1), ptr addrspace(2), <2 x i32>, i32, i32, i32)
declare void @air.write_texture_cube.v4f32(ptr addrspace(1), <2 x i32>, i32, <4 x float>, i32, i32)

!air.kernel = !{!0}
!0 = !{ptr @copy_cube_face, !1, !2}
!1 = !{}
!2 = !{!3, !4, !5, !6}
!3 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.read", !"air.arg_type_name", !"texturecube<float, read>", !"air.arg_name", !"src"}
!4 = !{i32 1, !"air.texture", !"air.location_index", i32 1, i32 1, !"air.write", !"air.arg_type_name", !"texturecube<float, write>", !"air.arg_name", !"dst"}
!5 = !{i32 2, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint2", !"air.arg_name", !"coord"}
!6 = !{i32 3, !"air.threadgroup_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"face"}
"#;

const TEXTURE_SHAPE_GRAMMAR: &str = r#"target triple = "spirv-unknown-vulkan1.2"

define void @probe(ptr addrspace(1) %a, ptr addrspace(1) %b, ptr addrspace(1) %c, ptr addrspace(1) %d, ptr addrspace(1) %out) {
entry:
  %w0 = call i32 @air.get_width_texture_2d_array(ptr addrspace(1) %a, i32 0)
  %w1 = call i32 @air.get_width_texture_3d(ptr addrspace(1) %b, i32 0)
  %w2 = call i32 @air.get_width_texture_1d(ptr addrspace(1) %c, i32 0)
  call void @air.write_texture_2d.v4f32(ptr addrspace(1) %d, <2 x i32> zeroinitializer, <4 x float> zeroinitializer, i32 0, i32 2)
  %s0 = add i32 %w0, %w1
  %s1 = add i32 %s0, %w2
  store i32 %s1, ptr addrspace(1) %out, align 4
  ret void
}

declare i32 @air.get_width_texture_2d_array(ptr addrspace(1), i32)
declare i32 @air.get_width_texture_3d(ptr addrspace(1), i32)
declare i32 @air.get_width_texture_1d(ptr addrspace(1), i32)
declare void @air.write_texture_2d.v4f32(ptr addrspace(1), <2 x i32>, <4 x float>, i32, i32)

!air.kernel = !{!0}
!0 = !{ptr @probe, !1, !2}
!1 = !{}
!2 = !{!3, !4, !5, !6, !7}
!3 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.sample", !"air.arg_type_name", !"texture2d_array<half, sample>", !"air.arg_name", !"a"}
!4 = !{i32 1, !"air.texture", !"air.location_index", i32 1, i32 1, !"air.read", !"air.arg_type_name", !"texture3d<uint, read>", !"air.arg_name", !"b"}
!5 = !{i32 2, !"air.texture", !"air.location_index", i32 2, i32 1, !"air.read", !"air.arg_type_name", !"texture1d<int, read>", !"air.arg_name", !"c"}
!6 = !{i32 3, !"air.texture", !"air.location_index", i32 3, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"d"}
!7 = !{i32 4, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 0, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
"#;

#[test]
fn every_shape_the_type_name_grammar_produces_matches_its_emitted_image() {
    let (spirv, reflection) = translate(TEXTURE_SHAPE_GRAMMAR, Stage::Kernel, "shape_grammar");
    let checked = assert_shapes_match_the_module("the shape-grammar probe", &spirv, &reflection);
    assert_eq!(checked, 4, "each of the four textures has a shape to check");

    let mut emitted = reflection
        .bindings
        .iter()
        .filter_map(|resource| resource.texture_shape)
        .map(|shape| {
            (
                shape.dimension,
                shape.arrayed,
                shape.component,
                shape.writable,
            )
        })
        .collect::<Vec<_>>();
    emitted.sort_by_key(|shape| format!("{shape:?}"));
    assert_eq!(
        emitted,
        vec![
            (TextureDimension::D1, false, TextureComponent::Sint, false),
            (TextureDimension::D2, false, TextureComponent::Float, true),
            (TextureDimension::D2, true, TextureComponent::Float, false),
            (TextureDimension::D3, false, TextureComponent::Uint, false),
        ]
    );
}

#[test]
fn a_texel_read_cube_reflects_the_two_dimensional_array_it_binds() {
    let (spirv, reflection) = translate(CUBE_TEXEL_COPY, Stage::Kernel, "cube_texel_copy");
    assert_shapes_match_the_module("the cube texel copy", &spirv, &reflection);

    let shapes = reflection
        .bindings
        .iter()
        .filter_map(|resource| Some((resource.descriptor?.binding, resource.texture_shape?)))
        .collect::<HashMap<_, _>>();
    let source = shapes[&32];
    assert_eq!(
        (source.dimension, source.arrayed),
        (TextureDimension::D2, true),
        "the texel-read cube binds as a 2D array, so a consumer must create a 2D-array view"
    );
    let destination = shapes[&481];
    assert_eq!(
        (destination.dimension, destination.arrayed),
        (TextureDimension::Cube, false),
        "the written cube keeps Dim Cube, so the same AIR type name must not decide both"
    );
    assert!(destination.writable && !source.writable);
    assert_eq!(source.component, TextureComponent::Float);
}

#[test]
fn every_public_fixture_reflects_the_images_it_binds() {
    let mut checked = 0;
    let mut shapes = 0;
    for path in public_fixtures() {
        let source = std::fs::read_to_string(&path).expect("read fixture");
        let label = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(stage) = stage_of(&source) else {
            continue;
        };
        let Ok((spirv, reflection)) = metal2vulkan::translate_sanitized_native_reflected(
            &source,
            stage,
            &scratch(&label),
            TransformOptions::default(),
        ) else {
            continue;
        };
        shapes += assert_shapes_match_the_module(&label, &spirv, &reflection);
        checked += 1;
    }
    assert!(
        checked >= 20 && shapes >= 1,
        "only {checked} fixtures carrying {shapes} texture shapes were inspected, so this swept \
         almost nothing"
    );
}

fn assert_shapes_match_the_module(
    label: &str,
    spirv: &[u8],
    reflection: &ShaderReflection,
) -> usize {
    let declared = images_the_module_declares(spirv);
    let mut checked = 0;
    for resource in &reflection.bindings {
        let (Some(shape), Some(location)) = (resource.texture_shape, resource.descriptor) else {
            continue;
        };
        let Some(Some(image)) = declared.get(&location.binding) else {
            continue;
        };
        assert_eq!(
            (
                shape.dimension,
                shape.arrayed,
                shape.multisampled,
                shape.component,
                shape.writable
            ),
            (
                image.dimension,
                image.arrayed,
                image.multisampled,
                image.component,
                image.writable
            ),
            "{label} reflects {:?}({}) at binding {} with a shape the module's OpTypeImage \
             contradicts; a view built from it would not match the image variable",
            resource.kind,
            resource.metal_index,
            location.binding
        );
        checked += 1;
    }
    checked
}

fn images_the_module_declares(spirv: &[u8]) -> HashMap<u32, Option<TextureShape>> {
    let text = metal2vulkan::disassemble(spirv).expect("disassemble the translated module");
    let mut floats = Vec::new();
    let mut signed_integers = Vec::new();
    let mut unsigned_integers = Vec::new();
    let mut images: HashMap<String, TextureShape> = HashMap::new();
    let mut pointees: HashMap<String, String> = HashMap::new();
    let mut elements: HashMap<String, String> = HashMap::new();
    let mut variables: HashMap<String, String> = HashMap::new();
    let mut bindings: Vec<(String, u32)> = Vec::new();
    for line in text.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        match tokens.as_slice() {
            ["OpDecorate", target, "Binding", value] => {
                if let Ok(value) = value.parse::<u32>() {
                    bindings.push(((*target).to_string(), value));
                }
            }
            [result, "=", "OpTypeFloat", ..] => floats.push((*result).to_string()),
            [result, "=", "OpTypeInt", _, signedness] => {
                if *signedness == "1" {
                    signed_integers.push((*result).to_string());
                } else {
                    unsigned_integers.push((*result).to_string());
                }
            }
            [result, "=", "OpTypePointer", _, pointee] => {
                pointees.insert((*result).to_string(), (*pointee).to_string());
            }
            [result, "=", "OpTypeArray", element, _]
            | [result, "=", "OpTypeRuntimeArray", element] => {
                elements.insert((*result).to_string(), (*element).to_string());
            }
            [result, "=", "OpVariable", pointer, _] => {
                variables.insert((*result).to_string(), (*pointer).to_string());
            }
            [result, "=", "OpTypeImage", sampled, dimension, _, arrayed, multisampled, sampled_operand, format] =>
            {
                let component = if floats.iter().any(|id| id == sampled) {
                    TextureComponent::Float
                } else if signed_integers.iter().any(|id| id == sampled) {
                    TextureComponent::Sint
                } else if unsigned_integers.iter().any(|id| id == sampled) {
                    TextureComponent::Uint
                } else {
                    continue;
                };
                let dimension = match *dimension {
                    "1D" => TextureDimension::D1,
                    "2D" => TextureDimension::D2,
                    "3D" => TextureDimension::D3,
                    "Cube" => TextureDimension::Cube,
                    "Buffer" => TextureDimension::Buffer,
                    _ => continue,
                };
                images.insert(
                    (*result).to_string(),
                    TextureShape {
                        dimension,
                        arrayed: *arrayed != "0",
                        multisampled: *multisampled != "0",
                        component,
                        writable: *sampled_operand == "2",
                        array_ref: false,
                        array_length: None,
                        storage_format: metal2vulkan::meta::TextureFormat::ALL.into_iter().find(
                            |candidate| format!("{:?}", candidate.to_spirv_format()) == *format,
                        ),
                    },
                );
            }
            _ => {}
        }
    }
    let mut declared: HashMap<u32, Option<TextureShape>> = HashMap::new();
    for (variable, binding) in bindings {
        let Some(pointer) = variables.get(&variable) else {
            continue;
        };
        let Some(mut pointee) = pointees.get(pointer).cloned() else {
            continue;
        };
        while let Some(element) = elements.get(&pointee) {
            pointee = element.clone();
        }
        let Some(image) = images.get(&pointee).copied() else {
            continue;
        };
        declared
            .entry(binding)
            .and_modify(|seen| {
                if *seen != Some(image) {
                    *seen = None;
                }
            })
            .or_insert(Some(image));
    }
    declared
}

fn translate(source: &str, stage: Stage, label: &str) -> (Vec<u8>, ShaderReflection) {
    metal2vulkan::translate_sanitized_native_reflected(
        source,
        stage,
        &scratch(label),
        TransformOptions::default(),
    )
    .unwrap_or_else(|error| panic!("{label} translates: {error}"))
}

fn scratch(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "m2v_reflected_images_{}_{}",
        std::process::id(),
        label.replace(['/', '.'], "_")
    ));
    std::fs::create_dir_all(&directory).expect("scratch directory");
    directory
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
