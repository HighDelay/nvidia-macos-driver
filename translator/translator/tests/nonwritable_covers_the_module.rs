use metal2vulkan::passes::{Stage, TransformOptions};
use metal2vulkan::reflect::{ResourceAccess, ResourceKind, ShaderReflection};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const MIXED_ACCESS: &str = r#"target triple = "spirv-unknown-vulkan1.2"
define void @k(ptr addrspace(1) %read_only, ptr addrspace(1) %declared_read, ptr addrspace(1) %written) {
entry:
  %v = load i32, ptr addrspace(1) %read_only, align 4
  store i32 %v, ptr addrspace(1) %declared_read, align 4
  store i32 %v, ptr addrspace(1) %written, align 4
  ret void
}
!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !4, !5}
!3 = !{i32 0, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"read_only"}
!4 = !{i32 1, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 1, i32 1, !"air.read", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"declared_read"}
!5 = !{i32 2, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 2, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"written"}
"#;

#[test]
fn only_the_buffer_the_body_never_writes_is_decorated() {
    let (spirv, reflection) = translate(MIXED_ACCESS, Stage::Kernel, "mixed_access");
    assert_nonwritable_covers_the_module("the mixed-access kernel", &spirv);

    let decorated = decorated_bindings(&spirv);
    for (metal_index, name, expected) in [
        (0u32, "read_only", true),
        (1, "declared_read", false),
        (2, "written", false),
    ] {
        let binding = reflection
            .binding_at(ResourceKind::Buffer, metal_index)
            .and_then(|resource| resource.descriptor)
            .unwrap_or_else(|| {
                panic!("buffer {metal_index} ({name}) is reflected with a descriptor")
            })
            .binding;
        assert_eq!(
            decorated.contains(&binding),
            expected,
            "{name} at binding {binding}: NonWritable present={}, expected={expected}",
            decorated.contains(&binding)
        );
    }
}

#[test]
fn a_declared_read_the_body_writes_is_not_decorated() {
    let (spirv, reflection) = translate(MIXED_ACCESS, Stage::Kernel, "declared_read_written");
    let binding = reflection
        .binding_at(ResourceKind::Buffer, 1)
        .and_then(|resource| resource.descriptor)
        .expect("declared_read is reflected with a descriptor")
        .binding;
    assert!(
        !decorated_bindings(&spirv).contains(&binding),
        "declared_read carries air.read but the body stores through it; decorating it NonWritable \
         would license a driver to place a written buffer in read-only memory"
    );
}

const MIXED_IMAGE_ACCESS: &str = r#"target triple = "air64_v28-apple-macosx26.0.0"
define void @k(ptr addrspace(1) captures(none) %0, ptr addrspace(1) captures(none) %1, ptr addrspace(1) captures(none) %2, <2 x i32> noundef %3) local_unnamed_addr #0 {
  %s = tail call ptr addrspace(2) @air.get_read_sampler() #1
  %a = tail call { <4 x float>, i8 } @air.read_texture_2d.v4f32(ptr addrspace(1) captures(none) %0, ptr addrspace(2) %s, <2 x i32> %3, <2 x i32> zeroinitializer, i32 0, i32 0) #1
  %av = extractvalue { <4 x float>, i8 } %a, 0
  %b = tail call { <4 x float>, i8 } @air.read_texture_2d.v4f32(ptr addrspace(1) captures(none) %1, ptr addrspace(2) %s, <2 x i32> %3, <2 x i32> zeroinitializer, i32 0, i32 0) #1
  %bv = extractvalue { <4 x float>, i8 } %b, 0
  %sum = fadd <4 x float> %av, %bv
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) captures(none) %1, <2 x i32> %3, <4 x float> %sum, i32 0, i32 2) #1
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) captures(none) %2, <2 x i32> %3, <4 x float> %av, i32 0, i32 2) #1
  ret void
}
declare ptr addrspace(2) @air.get_read_sampler() local_unnamed_addr #1
declare { <4 x float>, i8 } @air.read_texture_2d.v4f32(ptr addrspace(1) captures(none), ptr addrspace(2), <2 x i32>, <2 x i32>, i32, i32) local_unnamed_addr #1
declare void @air.write_texture_2d.v4f32(ptr addrspace(1) captures(none), <2 x i32>, <4 x float>, i32, i32) local_unnamed_addr #1
attributes #0 = { nounwind }
attributes #1 = { nounwind }
!air.kernel = !{!0}
!air.version = !{!10}
!air.language_version = !{!11}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !4, !5, !6}
!3 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.arg_type_name", !"texture2d<float, read_write>", !"air.arg_name", !"never_written"}
!4 = !{i32 1, !"air.texture", !"air.location_index", i32 1, i32 1, !"air.read_write", !"air.arg_type_name", !"texture2d<float, read_write>", !"air.arg_name", !"read_then_written"}
!5 = !{i32 2, !"air.texture", !"air.location_index", i32 2, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"written"}
!6 = !{i32 3, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint2", !"air.arg_name", !"gid"}
!10 = !{i32 2, i32 8, i32 0}
!11 = !{!"Metal", i32 4, i32 0, i32 0}
"#;

#[test]
fn only_the_storage_image_the_body_never_writes_is_decorated() {
    let (spirv, reflection) = translate(MIXED_IMAGE_ACCESS, Stage::Kernel, "mixed_image_access");
    assert_nonwritable_covers_the_module("the mixed-access image kernel", &spirv);

    let decorated = decorated_bindings(&spirv);
    for (metal_index, name, expected) in [
        (0u32, "never_written", true),
        (1, "read_then_written", false),
        (2, "written", false),
    ] {
        let binding = reflection
            .binding_at(ResourceKind::StorageImage, metal_index)
            .and_then(|resource| resource.descriptor)
            .unwrap_or_else(|| {
                panic!("storage image {metal_index} ({name}) is reflected with a descriptor")
            })
            .binding;
        assert_eq!(
            decorated.contains(&binding),
            expected,
            "{name} at binding {binding}: NonWritable present={}, expected={expected}",
            decorated.contains(&binding)
        );
    }
}

#[test]
fn a_read_write_image_the_body_only_reads_stops_demanding_stores() {
    let (spirv, _) = translate(MIXED_IMAGE_ACCESS, Stage::Kernel, "image_demand");
    let text = metal2vulkan::disassemble(&spirv).expect("disassemble");
    let written = written_images(&text)
        .into_iter()
        .map(|(_, root)| root)
        .collect::<HashSet<_>>();
    let decorated = decorated_ids(&text);
    let images = text
        .lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                [result, "=", "OpTypeImage", ..] => Some((*result).to_string()),
                _ => None,
            },
        )
        .collect::<HashSet<_>>();
    let pointers = text
        .lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                [result, "=", "OpTypePointer", _, pointee] if images.contains(*pointee) => {
                    Some((*result).to_string())
                }
                _ => None,
            },
        )
        .collect::<HashSet<_>>();
    let mut unwritten = 0;
    for line in text.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let [result, "=", "OpVariable", ty, ..] = tokens.as_slice() else {
            continue;
        };
        if !pointers.contains(*ty) || written.contains(*result) {
            continue;
        }
        unwritten += 1;
        assert!(
            decorated.contains(*result),
            "the storage image {result} is never written yet carries no NonWritable, so the module \
             still demands fragmentStoresAndAtomics of every consumer"
        );
    }
    assert_eq!(
        unwritten, 1,
        "the fixture is supposed to leave exactly one storage image unwritten"
    );
}

#[test]
fn a_decorated_buffer_never_reflects_a_write() {
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
        let Ok((spirv, reflection)) = metal2vulkan::translate_sanitized_native_reflected(
            &source,
            stage,
            &scratch(&label),
            TransformOptions::default(),
        ) else {
            continue;
        };
        let decorated = decorated_bindings(&spirv);
        for resource in &reflection.bindings {
            let Some(descriptor) = resource.descriptor else {
                continue;
            };
            if !decorated.contains(&descriptor.binding) {
                continue;
            }
            assert!(
                !matches!(
                    resource.access,
                    Some(ResourceAccess::ReadWrite) | Some(ResourceAccess::WriteOnly)
                ),
                "{label} decorates binding {} NonWritable but reflects {:?} for it",
                descriptor.binding,
                resource.access
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 10,
        "only {checked} decorated bindings were inspected, so this swept almost nothing"
    );
}

#[test]
fn no_public_fixture_writes_a_decorated_buffer() {
    let mut checked = 0;
    let mut decorated = 0;
    for path in public_fixtures() {
        let source = std::fs::read_to_string(&path).expect("read fixture");
        let label = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(stage) = stage_of(&source) else {
            continue;
        };
        let Ok((spirv, _)) = metal2vulkan::translate_sanitized_native_reflected(
            &source,
            stage,
            &scratch(&label),
            TransformOptions::default(),
        ) else {
            continue;
        };
        assert_nonwritable_covers_the_module(&label, &spirv);
        decorated += decorated_bindings(&spirv).len();
        checked += 1;
    }
    assert!(
        checked >= 20 && decorated >= 10,
        "only {checked} fixtures carrying {decorated} NonWritable buffers were inspected, so this \
         swept almost nothing"
    );
}

fn assert_nonwritable_covers_the_module(label: &str, spirv: &[u8]) {
    let text = metal2vulkan::disassemble(spirv).expect("disassemble the translated module");
    let decorated = decorated_ids(&text);
    if decorated.is_empty() {
        return;
    }
    for (pointer, root) in written_pointers(&text) {
        assert!(
            !decorated.contains(&root),
            "{label} writes through {pointer}, rooted at the NonWritable variable {root}"
        );
    }
    for (image, root) in written_images(&text) {
        assert!(
            !decorated.contains(&root),
            "{label} writes the image {image}, loaded from the NonWritable variable {root}"
        );
    }
}

fn written_images(text: &str) -> Vec<(String, String)> {
    let image_types = text
        .lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                [result, "=", "OpTypeImage", ..] => Some((*result).to_string()),
                _ => None,
            },
        )
        .collect::<HashSet<_>>();
    let image_pointers = text
        .lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                [result, "=", "OpTypePointer", _, pointee] if image_types.contains(*pointee) => {
                    Some((*result).to_string())
                }
                _ => None,
            },
        )
        .collect::<HashSet<_>>();
    let mut root: HashMap<String, String> = text
        .lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                [result, "=", "OpVariable", ty, ..] if image_pointers.contains(*ty) => {
                    Some((*result).to_string())
                }
                _ => None,
            },
        )
        .map(|id| (id.clone(), id))
        .collect();
    loop {
        let mut changed = false;
        for line in text.lines() {
            let tokens = line.split_whitespace().collect::<Vec<_>>();
            let [result, "=", _opcode, ty, operands @ ..] = tokens.as_slice() else {
                continue;
            };
            if root.contains_key(*result) || !image_types.contains(*ty) {
                continue;
            }
            let Some(source) = operands
                .iter()
                .find_map(|operand| root.get(*operand).cloned())
            else {
                continue;
            };
            root.insert((*result).to_string(), source);
            changed = true;
        }
        if !changed {
            break;
        }
    }
    text.lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["OpImageWrite", image, ..] => Some((*image).to_string()),
                [_, "=", "OpImageTexelPointer", _, image, ..] => Some((*image).to_string()),
                _ => None,
            },
        )
        .filter_map(|image| root.get(&image).cloned().map(|root| (image, root)))
        .collect()
}

fn decorated_ids(text: &str) -> HashSet<String> {
    text.lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["OpDecorate", target, "NonWritable"] => Some((*target).to_string()),
                _ => None,
            },
        )
        .collect()
}

fn decorated_bindings(spirv: &[u8]) -> HashSet<u32> {
    let text = metal2vulkan::disassemble(spirv).expect("disassemble the translated module");
    let decorated = decorated_ids(&text);
    text.lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["OpDecorate", target, "Binding", value] if decorated.contains(*target) => {
                    value.parse::<u32>().ok()
                }
                _ => None,
            },
        )
        .collect()
}

fn written_pointers(text: &str) -> Vec<(String, String)> {
    let mut root: HashMap<String, String> = text
        .lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                [result, "=", "OpVariable", ..] => Some((*result).to_string()),
                _ => None,
            },
        )
        .map(|id| (id.clone(), id))
        .collect();
    loop {
        let mut changed = false;
        for line in text.lines() {
            let tokens = line.split_whitespace().collect::<Vec<_>>();
            let [result, "=", opcode, rest @ ..] = tokens.as_slice() else {
                continue;
            };
            if root.contains_key(*result) {
                continue;
            }
            let bases = match *opcode {
                "OpAccessChain"
                | "OpInBoundsAccessChain"
                | "OpPtrAccessChain"
                | "OpInBoundsPtrAccessChain"
                | "OpCopyObject"
                | "OpBitcast"
                | "OpSelect"
                | "OpPhi" => &rest[1.min(rest.len())..],
                _ => continue,
            };
            let Some(base_root) = bases.iter().find_map(|base| root.get(*base).cloned()) else {
                continue;
            };
            root.insert((*result).to_string(), base_root);
            changed = true;
        }
        if !changed {
            break;
        }
    }
    text.lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["OpStore", pointer, ..] => Some((*pointer).to_string()),
                ["OpCopyMemory", pointer, ..] | ["OpCopyMemorySized", pointer, ..] => {
                    Some((*pointer).to_string())
                }
                [_, "=", opcode, _, pointer, ..]
                    if opcode.starts_with("OpAtomic") && *opcode != "OpAtomicLoad" =>
                {
                    Some((*pointer).to_string())
                }
                _ => None,
            },
        )
        .filter_map(|pointer| root.get(&pointer).cloned().map(|root| (pointer, root)))
        .collect()
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
        "m2v_nonwritable_{}_{}",
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
