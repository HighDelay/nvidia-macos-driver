use metal2vulkan::passes::Stage;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const NOT_ENFORCED_BY_SPIRV_VAL: &[&str] = &[
    "ClipDistance",
    "GroupNonUniformArithmetic",
    "GroupNonUniformClustered",
    "GroupNonUniformPartitionedEXT",
    "Sampled1D",
    "Image1D",
    "SampledBuffer",
    "ImageBuffer",
];

const IMAGE_CAPABILITIES: &[(&str, &str, bool)] = &[
    ("Sampled1D", "1D", false),
    ("Image1D", "1D", true),
    ("SampledBuffer", "Buffer", false),
    ("ImageBuffer", "Buffer", true),
];

#[test]
fn every_declared_image_capability_has_an_image_that_needs_it() {
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
        let Ok(spirv) = metal2vulkan::translate_sanitized_native(&source, stage, &scratch(&label))
        else {
            continue;
        };
        let text = metal2vulkan::disassemble(&spirv).expect("disassemble");
        let declared = text
            .lines()
            .filter_map(
                |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                    ["OpCapability", name] => Some((*name).to_string()),
                    _ => None,
                },
            )
            .collect::<HashSet<_>>();
        for (capability, dim, storage) in IMAGE_CAPABILITIES {
            if !declared.contains(*capability) {
                continue;
            }
            let present = text.lines().any(|line| {
                match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                    [_, "=", "OpTypeImage", _, image_dim, _, _, _, sampled, ..] => {
                        image_dim == dim && (*sampled == "2") == *storage
                    }
                    _ => false,
                }
            });
            assert!(
                present,
                "{label} declares OpCapability {capability}, but no OpTypeImage in it is a                  {} {dim} image",
                if *storage { "storage" } else { "sampled" }
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 2,
        "only {checked} declared image capabilities were inspected, so this swept almost nothing"
    );
}

const IMPLIED_BY_ANOTHER: &[&str] = &["GroupNonUniform"];

#[test]
fn no_public_fixture_declares_a_capability_it_does_not_need() {
    let mut checked = 0;
    let mut stripped = 0;
    for path in public_fixtures() {
        let source = std::fs::read_to_string(&path).expect("read fixture");
        let label = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(stage) = stage_of(&source) else {
            continue;
        };
        let scratch = scratch(&label);
        let Ok(spirv) = metal2vulkan::translate_sanitized_native(&source, stage, &scratch) else {
            continue;
        };
        let exempt = NOT_ENFORCED_BY_SPIRV_VAL
            .iter()
            .chain(IMPLIED_BY_ANOTHER)
            .copied()
            .collect::<HashSet<_>>();
        for (index, name) in declared_capabilities(&spirv) {
            if name == "Shader" || exempt.contains(name.as_str()) {
                continue;
            }
            let without = without_instruction(&spirv, index);
            if metal2vulkan::tools::spirv_val_bytes(&without, &scratch).is_ok() {
                panic!(
                    "{label} declares OpCapability {name}, but the module still validates without \
                     it. A declared capability is a demand on every consumer -- it must name \
                     something the module actually does."
                );
            }
            stripped += 1;
        }
        checked += 1;
    }
    assert!(
        checked >= 20 && stripped >= 20,
        "only {checked} fixtures carrying {stripped} strippable capabilities were inspected, so \
         this swept almost nothing"
    );
}

const GROUP_ARITHMETIC_OPCODES: &[&str] = &[
    "OpGroupNonUniformIAdd",
    "OpGroupNonUniformFAdd",
    "OpGroupNonUniformSMin",
    "OpGroupNonUniformUMin",
    "OpGroupNonUniformFMin",
    "OpGroupNonUniformSMax",
    "OpGroupNonUniformUMax",
    "OpGroupNonUniformFMax",
    "OpGroupNonUniformBitwiseAnd",
    "OpGroupNonUniformBitwiseOr",
    "OpGroupNonUniformBitwiseXor",
];

#[test]
fn group_arithmetic_capability_follows_the_group_operation() {
    let mut checked = 0;
    for (label, text) in translated_fixtures() {
        let declared = capability_names(&text);
        let non_clustered = text.lines().any(|line| {
            let tokens = line.split_whitespace().collect::<Vec<_>>();
            let [_, "=", opcode, rest @ ..] = tokens.as_slice() else {
                return false;
            };
            GROUP_ARITHMETIC_OPCODES.contains(opcode) && !rest.contains(&"ClusteredReduce")
        });
        if declared.contains("GroupNonUniformArithmetic") {
            assert!(
                non_clustered,
                "{label} declares OpCapability GroupNonUniformArithmetic, but every group \
                 arithmetic instruction in it is a ClusteredReduce, which takes \
                 GroupNonUniformClustered instead"
            );
            checked += 1;
        } else {
            assert!(
                !non_clustered,
                "{label} performs a non-clustered group arithmetic operation without declaring \
                 GroupNonUniformArithmetic"
            );
        }
    }
    assert!(
        checked >= 1,
        "no fixture declared GroupNonUniformArithmetic, so this swept almost nothing"
    );
}

#[test]
fn variable_pointers_is_not_declared_for_addresses() {
    let mut checked = 0;
    for (label, text) in translated_fixtures() {
        let declared = capability_names(&text);
        if !declared.contains("VariablePointers") {
            checked += usize::from(declared.contains("PhysicalStorageBufferAddresses"));
            continue;
        }
        let logical = pointer_types(&text);
        let needs = text.lines().any(|line| {
            let tokens = line.split_whitespace().collect::<Vec<_>>();
            let [_, "=", opcode, result_type, ..] = tokens.as_slice() else {
                return false;
            };
            matches!(
                *opcode,
                "OpPhi" | "OpSelect" | "OpPtrAccessChain" | "OpInBoundsPtrAccessChain"
            ) && logical.contains(*result_type)
        });
        assert!(
            needs,
            "{label} declares OpCapability VariablePointers, but no pointer merge in it produces a \
             pointer the capability governs"
        );
    }
    assert!(
        checked >= 1,
        "no fixture used PhysicalStorageBufferAddresses without VariablePointers, so this swept \
         almost nothing"
    );
}

fn pointer_types(text: &str) -> HashSet<String> {
    text.lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                [result, "=", "OpTypePointer", storage, ..]
                    if *storage != "StorageBuffer" && *storage != "PhysicalStorageBuffer" =>
                {
                    Some((*result).to_string())
                }
                _ => None,
            },
        )
        .collect()
}

fn capability_names(text: &str) -> HashSet<String> {
    text.lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["OpCapability", name] => Some((*name).to_string()),
                _ => None,
            },
        )
        .collect()
}

fn translated_fixtures() -> Vec<(String, String)> {
    public_fixtures()
        .into_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(&path).ok()?;
            let label = path.file_name()?.to_string_lossy().into_owned();
            let stage = stage_of(&source)?;
            let spirv =
                metal2vulkan::translate_sanitized_native(&source, stage, &scratch(&label)).ok()?;
            Some((label, metal2vulkan::disassemble(&spirv).ok()?))
        })
        .collect()
}

fn declared_capabilities(spirv: &[u8]) -> Vec<(usize, String)> {
    let words = words_of(spirv);
    let text = metal2vulkan::disassemble(spirv).expect("disassemble");
    let names = text
        .lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["OpCapability", name] => Some((*name).to_string()),
                _ => None,
            },
        )
        .collect::<Vec<_>>();
    let mut found = Vec::new();
    let mut offset = 5;
    while offset < words.len() {
        let count = (words[offset] >> 16) as usize;
        if count == 0 {
            break;
        }
        if words[offset] & 0xFFFF == 17 {
            found.push(offset);
        }
        offset += count;
    }
    assert_eq!(
        found.len(),
        names.len(),
        "the binary and the disassembly disagree on how many capabilities there are"
    );
    found.into_iter().zip(names).collect()
}

fn without_instruction(spirv: &[u8], offset: usize) -> Vec<u8> {
    let words = words_of(spirv);
    let count = (words[offset] >> 16) as usize;
    words
        .iter()
        .enumerate()
        .filter(|(index, _)| *index < offset || *index >= offset + count)
        .flat_map(|(_, word)| word.to_le_bytes())
        .collect()
}

fn words_of(spirv: &[u8]) -> Vec<u32> {
    spirv
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

fn scratch(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "m2v_declared_capabilities_{}_{}",
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
