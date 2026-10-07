use std::collections::HashSet;
use std::path::{Path, PathBuf};

use metal2vulkan::env_vars::REGISTRY;

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn literals_after(src: &str, helper: &str) -> Vec<String> {
    src.split(helper)
        .skip(1)
        .filter_map(|chunk| chunk.find('"').map(|end| chunk[..end].to_string()))
        .collect()
}

#[test]
fn every_accessor_knob_is_registered() {
    let registry: HashSet<&str> = REGISTRY.iter().map(|v| v.name).collect();
    let env_vars_src = std::fs::read_to_string(src_dir().join("env_vars.rs")).unwrap();

    let mut knobs = Vec::new();
    for helper in ["present(\"", "std::env::var(\"", "std::env::var_os(\""] {
        knobs.extend(literals_after(&env_vars_src, helper));
    }

    let missing: Vec<&String> = knobs
        .iter()
        .filter(|k| k.starts_with("METAL2VULKAN_") && !registry.contains(k.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "env_vars.rs accessors read knobs absent from REGISTRY (add a registry entry so \
         `--help` documents them): {missing:?}"
    );
}

#[test]
fn all_knob_reads_are_centralized_in_env_vars() {
    let mut files = Vec::new();
    rs_files(&src_dir(), &mut files);

    let mut offenders = Vec::new();
    for f in &files {
        if f.ends_with("env_vars.rs") {
            continue;
        }
        let src = std::fs::read_to_string(f).unwrap();
        for line in src.lines() {
            if line.contains("env::var") && line.contains("\"METAL2VULKAN_") {
                offenders.push(format!("{}: {}", f.display(), line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "raw `METAL2VULKAN_*` env reads outside env_vars.rs (route through an env_vars accessor \
         instead):\n{}",
        offenders.join("\n")
    );
}

#[test]
fn registry_names_are_unique_and_well_formed() {
    let mut seen = HashSet::new();
    for v in REGISTRY {
        assert!(
            v.name.starts_with("METAL2VULKAN_") && v.name.len() > "METAL2VULKAN_".len(),
            "malformed registry name: {:?}",
            v.name
        );
        assert!(seen.insert(v.name), "duplicate registry name: {:?}", v.name);
    }
}
