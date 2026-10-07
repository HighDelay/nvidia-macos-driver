use metal2vulkan::passes::{Stage, TransformOptions};
use metal2vulkan::reflect::{ResourceKind, ShaderReflection};
use metal2vulkan::translate_sanitized_native_reflected;
use std::path::{Path, PathBuf};

const FRAGMENT_ARGUMENT_BUFFER: &str = r#"target triple = "spirv-unknown-vulkan1.2"
%Args = type <{ %"struct.metal::texture2d", ptr addrspace(1) }>
%"struct.metal::texture2d" = type { ptr addrspace(1) }

define <4 x float> @shade(ptr addrspace(2) %args) {
entry:
  %field = getelementptr inbounds %Args, ptr addrspace(2) %args, i64 0, i32 0, i32 0
  %tex = load ptr addrspace(1), ptr addrspace(2) %field, align 8
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) %tex, <2 x i32> zeroinitializer, <4 x float> zeroinitializer, i32 0, i32 2)
  ret <4 x float> zeroinitializer
}

declare void @air.write_texture_2d.v4f32(ptr addrspace(1), <2 x i32>, <4 x float>, i32, i32)

!air.fragment = !{!0}
!0 = !{ptr @shade, !1, !2}
!1 = !{!3}
!2 = !{!4}
!3 = !{i32 0, !"air.render_target", !"air.location_index", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!4 = !{i32 0, !"air.indirect_buffer", !"air.buffer_size", i32 16, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.struct_type_info", !5, !"air.arg_type_name", !"Args", !"air.arg_name", !"args"}
!5 = !{i32 0, i32 8, i32 0, !"texture2d<float, write>", !"output", !"air.indirect_argument", !6, i32 8, i32 8, i32 0, !"float", !"values", !"air.indirect_argument", !7}
!6 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"output"}
!7 = !{i32 1, !"air.buffer", !"air.buffer_size", i32 64, !"air.location_index", i32 3, i32 1, !"air.read", !"air.address_space", i32 1, !"air.arg_type_name", !"float", !"air.arg_name", !"values"}
"#;

fn scratch(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "m2v_resource_once_{}_{}",
        std::process::id(),
        label.replace(['/', '.'], "_")
    ));
    std::fs::create_dir_all(&directory).expect("scratch directory");
    directory
}

fn repeated_bindings(reflection: &ShaderReflection) -> Vec<(ResourceKind, u32)> {
    let mut repeated = Vec::new();
    for (index, resource) in reflection.bindings.iter().enumerate() {
        if reflection.bindings[..index].contains(resource) {
            repeated.push((resource.kind, resource.metal_index));
        }
    }
    repeated
}

#[test]
fn a_fragment_argument_buffer_reports_each_resident_once() {
    let (_spirv, reflection) = translate_sanitized_native_reflected(
        FRAGMENT_ARGUMENT_BUFFER,
        Stage::Fragment,
        &scratch("fragment_argument_buffer"),
        TransformOptions::default(),
    )
    .expect("the argument-buffer fragment translates");

    assert_eq!(
        repeated_bindings(&reflection),
        Vec::new(),
        "reflection repeats a resource: {:?}",
        reflection.bindings
    );
    for kind in [
        ResourceKind::EmbeddedArgBufferTexture,
        ResourceKind::EmbeddedArgBufferBuffer,
    ] {
        let count = reflection
            .bindings
            .iter()
            .filter(|resource| resource.kind == kind)
            .count();
        assert_eq!(
            count, 1,
            "the argument buffer holds exactly one {kind:?}, reflection reports {count}"
        );
    }
}

#[test]
fn every_public_fixture_reports_each_resource_once() {
    let mut checked = 0;
    let mut reported = 0;
    for path in public_fixtures() {
        let source = std::fs::read_to_string(&path).expect("read fixture");
        let label = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(stage) = stage_of(&source) else {
            continue;
        };
        let Ok((_spirv, reflection)) = translate_sanitized_native_reflected(
            &source,
            stage,
            &scratch(&label),
            TransformOptions::default(),
        ) else {
            continue;
        };
        assert_eq!(
            repeated_bindings(&reflection),
            Vec::new(),
            "{label} repeats a resource: {:?}",
            reflection.bindings
        );
        reported += reflection.bindings.len();
        checked += 1;
    }
    assert!(
        checked >= 20 && reported >= 20,
        "only {checked} fixtures reporting {reported} resources were inspected, so this swept \
         almost nothing"
    );
}

#[test]
fn the_descriptor_abi_gate_rejects_a_repeated_resource() {
    let (_spirv, reflection) = translate_sanitized_native_reflected(
        FRAGMENT_ARGUMENT_BUFFER,
        Stage::Fragment,
        &scratch("gate_rejects_repeat"),
        TransformOptions::default(),
    )
    .expect("the argument-buffer fragment translates");
    reflection
        .validate_descriptor_abi()
        .expect("the reflection it produced is valid");

    let mut doubled = reflection.clone();
    let resident = doubled
        .bindings
        .iter()
        .find(|resource| resource.kind == ResourceKind::EmbeddedArgBufferTexture)
        .expect("the argument-buffer texture")
        .clone();
    doubled.bindings.push(resident);
    let error = doubled
        .validate_descriptor_abi()
        .expect_err("a repeated resource is not a valid descriptor ABI");
    assert!(
        error.contains("twice"),
        "the rejection should name the repeat, got {error}"
    );
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
