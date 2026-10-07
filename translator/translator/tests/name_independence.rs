use metal2vulkan::passes::Stage;
use metal2vulkan::translate_sanitized_native;
use std::path::{Path, PathBuf};

const CONTRACT_NAME_PREFIXES: &[&str] = &[
    "air.",
    "llvm.",
    "__air",
    "mtl.",
    "_GLOBAL__sub_I",
    "metal2vulkan.",
];

const CONTRACT_NAME_INFIXES: &[&str] = &["MTL_"];

const RENAMED_SYMBOL: &str = "renamed.symbol.";
const RENAMED_TYPE: &str = "renamed.type.";
const RENAMED_LOCAL: &str = "renamed.local.";

fn owned_by_contract(name: &str) -> bool {
    CONTRACT_NAME_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
        || CONTRACT_NAME_INFIXES
            .iter()
            .any(|infix| name.contains(infix))
}

fn identifier_at(source: &str, start: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    if bytes.get(start) == Some(&b'"') {
        let mut index = start + 1;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' => index += 2,
                b'"' => return Some((source.get(start + 1..index)?.to_string(), index + 1)),
                _ => index += 1,
            }
        }
        return None;
    }
    let mut index = start;
    while index < bytes.len()
        && matches!(bytes[index], b'-' | b'$' | b'.' | b'_' | b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z')
    {
        index += 1;
    }
    (index > start).then(|| (source[start..index].to_string(), index))
}

fn looks_like_a_type(name: &str, was_quoted: bool) -> bool {
    was_quoted
        || name.starts_with("struct.")
        || name.starts_with("class.")
        || name.starts_with("union.")
}

fn placeholder_for(assigned: &mut Vec<(String, String)>, name: &str, kind: &str) -> String {
    if let Some((_, to)) = assigned.iter().find(|(from, _)| from == name) {
        return to.clone();
    }
    let to = format!("{kind}{}", assigned.len());
    assigned.push((name.to_string(), to.clone()));
    to
}

fn rename_names_the_contract_does_not_own(source: &str) -> (String, usize) {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut assigned: Vec<(String, String)> = Vec::new();
    let mut index = 0usize;
    let mut at_line_start = true;
    while index < bytes.len() {
        let from = index;
        if at_line_start {
            if let Some((name, end)) = identifier_at(source, index) {
                let numbered = name.starts_with(|character: char| character.is_ascii_digit());
                if bytes.get(end) == Some(&b':') && !numbered && !owned_by_contract(&name) {
                    out.push_str(&placeholder_for(&mut assigned, &name, RENAMED_LOCAL));
                    index = end;
                    at_line_start = false;
                    continue;
                }
            }
        }
        match bytes[index] {
            b';' => {
                let end = source[index..]
                    .find('\n')
                    .map_or(bytes.len(), |offset| index + offset);
                out.push_str(&source[index..end]);
                index = end;
            }
            b'"' => {
                let end = identifier_at(source, index).map_or(index + 1, |(_, end)| end);
                out.push_str(&source[index..end]);
                index = end;
            }
            sigil @ (b'@' | b'%') => {
                let was_quoted = bytes.get(index + 1) == Some(&b'"');
                let Some((name, end)) = identifier_at(source, index + 1) else {
                    out.push(sigil as char);
                    index += 1;
                    continue;
                };
                let numbered = name.starts_with(|character: char| character.is_ascii_digit());
                if numbered || owned_by_contract(&name) {
                    out.push_str(&source[index..end]);
                    index = end;
                    continue;
                }
                let kind = if sigil == b'@' {
                    RENAMED_SYMBOL
                } else if looks_like_a_type(&name, was_quoted) {
                    RENAMED_TYPE
                } else {
                    RENAMED_LOCAL
                };
                out.push(sigil as char);
                out.push_str(&placeholder_for(&mut assigned, &name, kind));
                index = end;
            }
            _ => {
                let character = source[index..].chars().next().expect("in-bounds character");
                out.push(character);
                index += character.len_utf8();
            }
        }
        let consumed = &source[from..index];
        at_line_start = match consumed.rfind('\n') {
            Some(position) => consumed[position + 1..].chars().all(char::is_whitespace),
            None => at_line_start && consumed.chars().all(char::is_whitespace),
        };
    }
    (out, assigned.len())
}

fn without_debug_names(spirv: &[u8]) -> Vec<u32> {
    const OP_SOURCE_CONTINUED: u16 = 2;
    const OP_SOURCE: u16 = 3;
    const OP_SOURCE_EXTENSION: u16 = 4;
    const OP_NAME: u16 = 5;
    const OP_MEMBER_NAME: u16 = 6;
    const OP_STRING: u16 = 7;
    const OP_LINE: u16 = 8;
    const OP_NO_LINE: u16 = 317;
    const OP_MODULE_PROCESSED: u16 = 330;
    const DEBUG_NAME_OPCODES: &[u16] = &[
        OP_SOURCE_CONTINUED,
        OP_SOURCE,
        OP_SOURCE_EXTENSION,
        OP_NAME,
        OP_MEMBER_NAME,
        OP_STRING,
        OP_LINE,
        OP_NO_LINE,
        OP_MODULE_PROCESSED,
    ];
    const HEADER_WORDS: usize = 5;

    let words = spirv
        .chunks_exact(4)
        .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .collect::<Vec<_>>();
    assert!(
        words.len() >= HEADER_WORDS,
        "a translated module is at least a SPIR-V header, got {} words",
        words.len()
    );
    let mut kept = words[..HEADER_WORDS].to_vec();
    let mut index = HEADER_WORDS;
    while index < words.len() {
        let length = (words[index] >> 16) as usize;
        let opcode = (words[index] & 0xffff) as u16;
        assert!(
            length > 0 && index + length <= words.len(),
            "instruction at word {index} declares {length} words"
        );
        if !DEBUG_NAME_OPCODES.contains(&opcode) {
            kept.extend_from_slice(&words[index..index + length]);
        }
        index += length;
    }
    kept
}

fn scratch(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "m2v_name_independence_{}_{}",
        std::process::id(),
        label.replace(['/', '.'], "_")
    ));
    std::fs::create_dir_all(&directory).expect("scratch directory");
    directory
}

fn assert_name_independent(label: &str, source: &str, stage: Stage) -> Vec<u8> {
    for prefix in [RENAMED_SYMBOL, RENAMED_TYPE, RENAMED_LOCAL] {
        assert!(
            !source.contains(prefix),
            "{label} already contains the placeholder prefix {prefix}, so renaming could collide \
             with a name the module already uses"
        );
    }
    let (renamed, count) = rename_names_the_contract_does_not_own(source);
    assert!(
        count > 0,
        "{label} has no identifier outside the AIR/LLVM contract, so translating it twice proves \
         nothing about name independence"
    );

    let original = translate_sanitized_native(source, stage, &scratch(label))
        .unwrap_or_else(|error| panic!("{label} must translate to be compared: {error}"));
    let perturbed =
        translate_sanitized_native(&renamed, stage, &scratch(&format!("{label}_named")))
            .unwrap_or_else(|error| {
                panic!(
                    "{label} translates, but the same module with its {count} non-contract \
                 identifiers renamed does not: {error}"
                )
            });

    assert_eq!(
        without_debug_names(&original),
        without_debug_names(&perturbed),
        "{label} translated to a different module after {count} identifiers outside the AIR/LLVM \
         contract were renamed; some decision is reading a name instead of the structure"
    );
    original
}

const LOOK_ALIKE_CONTRACT_NAMES: &str = r#"source_filename = "air_get_width_texture_2d.metal"

define void @air_get_width_texture_2d(ptr addrspace(1) %llvm_memcpy_p0_p0_i64, ptr addrspace(1) %texture_buffer) {
  %_cube_array = call i32 @llvm_fabs_f32(ptr addrspace(1) %llvm_memcpy_p0_p0_i64)
  %_1d_array = call i32 @air_get_num_samples_texture_2d_ms(i32 %_cube_array)
  store i32 %_1d_array, ptr addrspace(1) %texture_buffer, align 4
  ret void
}

define internal i32 @llvm_fabs_f32(ptr addrspace(1) %source) {
  %value = load i32, ptr addrspace(1) %source, align 4
  ret i32 %value
}

define internal i32 @air_get_num_samples_texture_2d_ms(i32 %value) {
  %sum = add i32 %value, %value
  ret i32 %sum
}

!air.kernel = !{!0}
!0 = !{ptr @air_get_width_texture_2d, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 1, !"air.arg_type_name", !"uint", !"air.arg_name", !"input"}
!4 = !{i32 1, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 1, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_name", !"uint", !"air.arg_name", !"output"}
"#;

const MISLEADING_HANDLE_TYPE_NAME: &str = r#"%"struct.metal::texture_buffer<float, access::write>" = type { ptr addrspace(1) }

define void @width_of_second_texture(ptr readonly captures(none) %textures, ptr addrspace(1) noundef writeonly captures(none) %out) local_unnamed_addr #0 {
entry:
  %element = getelementptr %"struct.metal::texture_buffer<float, access::write>", ptr %textures, i64 1, i32 0
  %handle = load ptr addrspace(1), ptr %element, align 8
  %width = tail call i32 @air.get_width_texture_2d(ptr addrspace(1) readonly captures(none) %handle, i32 0) #1
  store i32 %width, ptr addrspace(1) %out, align 4
  ret void
}

declare i32 @air.get_width_texture_2d(ptr addrspace(1) readonly captures(none), i32) local_unnamed_addr #1

attributes #0 = { convergent nounwind }
attributes #1 = { convergent nounwind memory(none) }

!air.kernel = !{!0}
!0 = !{ptr @width_of_second_texture, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 2, !"air.sample", !"air.arg_type_name", !"array<texture2d<float, sample>, 2>", !"air.arg_name", !"textures"}
!4 = !{i32 1, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 0, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
"#;

const ENTRY_ALREADY_NAMED_MAIN: &str = r#"source_filename = "main.metal"

define void @main(ptr addrspace(1) %input, ptr addrspace(1) %output) {
  %value = load i32, ptr addrspace(1) %input, align 4
  store i32 %value, ptr addrspace(1) %output, align 4
  ret void
}

!air.kernel = !{!0}
!0 = !{ptr @main, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 1, !"air.arg_type_name", !"uint", !"air.arg_name", !"input"}
!4 = !{i32 1, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 1, i32 1, !"air.write", !"air.address_space", i32 1, !"air.arg_type_name", !"uint", !"air.arg_name", !"output"}
"#;

#[test]
fn identifiers_that_look_like_contract_names_are_still_just_identifiers() {
    assert_name_independent(
        "the look-alike-named kernel",
        LOOK_ALIKE_CONTRACT_NAMES,
        Stage::Kernel,
    );
}

#[test]
fn the_texture_shape_comes_from_the_metadata_not_the_handle_type_name() {
    let module = assert_name_independent(
        "the misleadingly typed texture array",
        MISLEADING_HANDLE_TYPE_NAME,
        Stage::Kernel,
    );
    let text = metal2vulkan::disassemble(&module).expect("disassemble the translated module");
    let images = text
        .lines()
        .filter(|line| line.contains("OpTypeImage"))
        .collect::<Vec<_>>();
    assert_eq!(
        images.len(),
        1,
        "the kernel declares one texture, so one image type is expected:\n{text}"
    );
    assert!(
        images[0].contains(" 2D "),
        "the handle type is named texture_buffer and the metadata declares texture2d; the metadata \
         is the ABI, so the emitted image must be 2D, not `{}`",
        images[0].trim()
    );
}

#[test]
fn an_entry_point_already_named_main_translates_like_any_other() {
    assert_name_independent(
        "the kernel whose entry is named main",
        ENTRY_ALREADY_NAMED_MAIN,
        Stage::Kernel,
    );
}

#[test]
fn every_public_fixture_translates_the_same_under_renamed_identifiers() {
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
        if translate_sanitized_native(&source, stage, &scratch(&label)).is_err() {
            continue;
        }
        assert_name_independent(&label, &source, stage);
        checked += 1;
    }
    assert!(
        checked >= 20,
        "only {checked} public fixtures translated, so this swept almost nothing"
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

mod renamer_tests {
    use super::*;

    #[test]
    fn contract_families_survive_and_everything_else_does_not() {
        let source = concat!(
            "%\"struct.metal::texture2d\" = type { i32 }\n",
            "@fc.MTL_FC_INIT_3_b = internal constant i8 undef\n",
            "define void @kernel_main(ptr %in) {\n",
            "  %v = call i32 @air.get_width_texture_2d(ptr %in, i32 0)\n",
            "  %w = call i32 @llvm.smax.i32(i32 %v, i32 0)\n",
            "  %x = call i32 @air_get_width(i32 %w)\n",
            "  ret void\n",
            "}\n",
        );
        let (renamed, count) = rename_names_the_contract_does_not_own(source);
        assert!(renamed.contains("@air.get_width_texture_2d"));
        assert!(renamed.contains("@llvm.smax.i32"));
        assert!(renamed.contains("@fc.MTL_FC_INIT_3_b"));
        assert!(!renamed.contains("@kernel_main"));
        assert!(!renamed.contains("@air_get_width"));
        assert!(!renamed.contains("%\"struct.metal::texture2d\""));
        assert_eq!(count, 7, "{renamed}");
    }

    #[test]
    fn numbered_values_keep_their_numbers() {
        let source = "define void @f(i32 %0) {\n  %2 = add i32 %0, 1\n  br label %3\n}\n";
        let (renamed, count) = rename_names_the_contract_does_not_own(source);
        assert!(renamed.contains("%0"), "{renamed}");
        assert!(renamed.contains("%2"), "{renamed}");
        assert!(renamed.contains("%3"), "{renamed}");
        assert_eq!(count, 1, "only @f is renamable: {renamed}");
    }

    #[test]
    fn a_label_is_renamed_where_it_is_defined_as_well_as_where_it_is_used() {
        let source = "define void @f() {\nentry:\n  br label %body\nbody:\n  ret void\n}\n";
        let (renamed, count) = rename_names_the_contract_does_not_own(source);
        assert_eq!(
            renamed,
            "define void @renamed.symbol.0() {\nrenamed.local.1:\n  br label %renamed.local.2\n\
             renamed.local.2:\n  ret void\n}\n"
        );
        assert_eq!(count, 3, "@f, entry, and body");
    }

    #[test]
    fn quoted_data_is_not_an_identifier() {
        let source = "; @commented_out\n@g = global i8 0, section \"air.fc_initializer\"\n!0 = !{!\"air.buffer\", !\"input\"}\n";
        let (renamed, _) = rename_names_the_contract_does_not_own(source);
        assert!(renamed.contains("; @commented_out"), "{renamed}");
        assert!(
            renamed.contains("section \"air.fc_initializer\""),
            "{renamed}"
        );
        assert!(renamed.contains("!\"air.buffer\", !\"input\""), "{renamed}");
    }
}
