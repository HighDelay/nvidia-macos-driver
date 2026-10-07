use metal2vulkan::passes::Stage;
use metal2vulkan::{disassemble, translate_sanitized_native};
use std::path::{Path, PathBuf};

fn global_variables_no_instruction_references(spirv: &[u8]) -> Vec<String> {
    let text = disassemble(spirv).expect("disassemble the translated module");
    let mut declared: Vec<(String, String)> = Vec::new();
    let mut referenced: Vec<String> = Vec::new();
    let mut inside_a_function = false;
    for line in text.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        if tokens.contains(&"OpFunction") {
            inside_a_function = true;
        }
        if inside_a_function {
            referenced.extend(
                tokens
                    .iter()
                    .filter(|token| is_id(token))
                    .map(|token| (*token).to_string()),
            );
            continue;
        }
        if let ([id, "=", "OpVariable", ..], Some(_)) = (tokens.as_slice(), tokens.get(4)) {
            if is_id(id) {
                declared.push(((*id).to_string(), line.trim().to_string()));
            }
        }
    }
    declared
        .into_iter()
        .filter(|(id, _)| !referenced.contains(id))
        .map(|(_, line)| line)
        .collect()
}

fn is_id(token: &str) -> bool {
    token.strip_prefix('%').is_some_and(|digits| {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn scratch(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "m2v_live_globals_{}_{}",
        std::process::id(),
        label.replace(['/', '.'], "_")
    ));
    std::fs::create_dir_all(&directory).expect("scratch directory");
    directory
}

fn assert_no_unused_globals(label: &str, spirv: &[u8]) -> usize {
    let unused = global_variables_no_instruction_references(spirv);
    assert!(
        unused.is_empty(),
        "{label} declares {} module-scope variable(s) no instruction references:\n  {}",
        unused.len(),
        unused.join("\n  ")
    );
    disassemble(spirv)
        .expect("disassemble the translated module")
        .lines()
        .take_while(|line| !line.split_whitespace().any(|token| token == "OpFunction"))
        .filter(|line| line.split_whitespace().any(|token| token == "OpVariable"))
        .count()
}

#[test]
fn no_public_fixture_translates_to_a_module_with_an_unused_global_variable() {
    let mut variables = 0;
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
        let Ok(spirv) = translate_sanitized_native(&source, stage, &scratch(&label)) else {
            continue;
        };
        variables += assert_no_unused_globals(&label, &spirv);
        checked += 1;
    }
    assert!(
        checked >= 20 && variables >= 40,
        "only {checked} fixtures with {variables} global variables were inspected, so this swept \
         almost nothing"
    );
}

#[test]
fn no_fixture_leaves_an_unused_global_variable_under_any_stage_it_translates_under() {
    let mut variables = 0;
    let mut checked = 0;
    for path in public_fixtures() {
        let source = std::fs::read_to_string(&path).expect("read fixture");
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        for stage in [Stage::Vertex, Stage::Fragment, Stage::Kernel] {
            let label = format!("{name}-{stage:?}");
            let Ok(spirv) = translate_sanitized_native(&source, stage, &scratch(&label)) else {
                continue;
            };
            variables += assert_no_unused_globals(&label, &spirv);
            checked += 1;
        }
    }
    assert!(
        checked >= 30 && variables >= 60,
        "only {checked} fixture/stage pairs with {variables} global variables were inspected, so \
         this swept almost nothing"
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
