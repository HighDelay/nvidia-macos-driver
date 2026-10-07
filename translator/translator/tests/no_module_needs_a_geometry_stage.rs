use metal2vulkan::passes::Stage;
use std::path::{Path, PathBuf};

#[test]
fn no_public_fixture_declares_the_geometry_capability() {
    let mut checked = 0;
    let mut primitive_id = 0;
    for path in public_fixtures() {
        let source = std::fs::read_to_string(&path).expect("read fixture");
        let label = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(stage) = stage_of(&source) else {
            continue;
        };
        let Ok(spirv) = metal2vulkan::translate_sanitized_native(&source, stage, &scratch(&label))
        else {
            continue;
        };
        let asm = metal2vulkan::disassemble(&spirv).expect("disassemble");
        assert!(
            !asm.contains("OpCapability Geometry"),
            "{label} declares Capability Geometry, which demands `geometryShader` -- a feature no \
             Metal-backed Vulkan implementation has, for a stage the source language cannot express"
        );
        if asm.contains("BuiltIn PrimitiveId") {
            primitive_id += 1;
            assert!(
                asm.contains("OpCapability Tessellation"),
                "{label} reads PrimitiveId but declares none of the capabilities that enable it"
            );
        }
        checked += 1;
    }
    assert!(
        checked >= 20,
        "only {checked} fixtures were inspected ({primitive_id} reading PrimitiveId), so this \
         swept almost nothing"
    );
}

fn scratch(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "m2v_geometry_capability_{}_{}",
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
