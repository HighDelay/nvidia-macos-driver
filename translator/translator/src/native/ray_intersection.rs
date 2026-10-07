use crate::meta::{AirIntersectionFamily, AirIntersectionInstancing, AirIntersectionResultField};

#[derive(Clone)]
struct Family {
    air_callee: String,
    lowered_callee: &'static str,
    air_result_type: String,
    lowered_result_type: String,
}

#[derive(Clone)]
struct ScopedLowering {
    scope: usize,
    result: String,
    family: Family,
}

fn aggregate_preserving_instruction(instruction: &str) -> bool {
    ["phi ", "select ", "freeze ", "insertvalue "]
        .iter()
        .any(|opcode| instruction.starts_with(opcode))
}

fn line_uses_llvm_value(line: &str, value: &str) -> bool {
    replace_llvm_value(line, value, "\u{0}").contains('\u{0}')
}

fn llvm_line_scopes(ll: &str) -> Vec<(&str, usize)> {
    let mut next_function = 1usize;
    let mut scope = 0usize;
    ll.lines()
        .map(|line| {
            if line.trim_start().starts_with("define ") {
                scope = next_function;
                next_function += 1;
            }
            let scoped = (line, scope);
            if line.trim() == "}" {
                scope = 0;
            }
            scoped
        })
        .collect()
}

const TRIANGLE_LOWERED_CALLEE: &str = "@metal2vulkan.intersect.callback_free.triangle_data(";
const PRIMITIVE_MOTION_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.primitive_motion(";
const TRIANGLE_PRIMITIVE_MOTION_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.triangle_data.primitive_motion(";
const INSTANCE_LOWERED_CALLEE: &str = "@metal2vulkan.intersect.callback_free.instancing(";
const INSTANCE_MOTION_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.instancing.motion(";
const INSTANCE_TRIANGLE_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.instancing.triangle_data(";
const INSTANCE_TRIANGLE_MOTION_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.instancing.triangle_data.motion(";
const INSTANCE_WORLD_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.instancing.world_space_data(";
const INSTANCE_WORLD_MOTION_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.instancing.world_space_data.motion(";
const INSTANCE_TRIANGLE_WORLD_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.instancing.triangle_data.world_space_data(";
const INSTANCE_TRIANGLE_WORLD_MOTION_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.instancing.triangle_data.world_space_data.motion(";
const MULTI_LEVEL_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.multi_level_instancing(";
const MULTI_LEVEL_MOTION_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.multi_level_instancing.motion(";
const MULTI_LEVEL_TRIANGLE_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.multi_level_instancing.triangle_data(";
const MULTI_LEVEL_TRIANGLE_MOTION_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.multi_level_instancing.triangle_data.motion(";
const MULTI_LEVEL_WORLD_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.multi_level_instancing.world_space_data(";
const MULTI_LEVEL_WORLD_MOTION_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.multi_level_instancing.world_space_data.motion(";
const MULTI_LEVEL_TRIANGLE_WORLD_LOWERED_CALLEE: &str =
    "@metal2vulkan.intersect.callback_free.multi_level_instancing.triangle_data.world_space_data(";
const MULTI_LEVEL_TRIANGLE_WORLD_MOTION_LOWERED_CALLEE: &str = "@metal2vulkan.intersect.callback_free.multi_level_instancing.triangle_data.world_space_data.motion(";

fn lowering_family(line: &str) -> Option<Family> {
    let start = line.find("@air.intersect.")?;
    let end = line[start..].find('(')? + start;
    let callee = &line[start + 1..end];
    let family = AirIntersectionFamily::parse(callee).ok()??;
    if family.intersection_function_buffer || family.user_data {
        return None;
    }
    let motion = family.primitive_motion || family.instance_motion;
    let lowered_callee = match (
        family.instancing,
        family.triangle_data,
        family.world_space_data,
        motion,
    ) {
        (AirIntersectionInstancing::None, true, false, false) => TRIANGLE_LOWERED_CALLEE,
        (AirIntersectionInstancing::None, false, false, true) => PRIMITIVE_MOTION_LOWERED_CALLEE,
        (AirIntersectionInstancing::None, true, false, true) => {
            TRIANGLE_PRIMITIVE_MOTION_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::SingleLevel, false, false, false) => INSTANCE_LOWERED_CALLEE,
        (AirIntersectionInstancing::SingleLevel, false, false, true) => {
            INSTANCE_MOTION_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::SingleLevel, true, false, false) => {
            INSTANCE_TRIANGLE_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::SingleLevel, true, false, true) => {
            INSTANCE_TRIANGLE_MOTION_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::SingleLevel, false, true, false) => {
            INSTANCE_WORLD_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::SingleLevel, false, true, true) => {
            INSTANCE_WORLD_MOTION_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::SingleLevel, true, true, false) => {
            INSTANCE_TRIANGLE_WORLD_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::SingleLevel, true, true, true) => {
            INSTANCE_TRIANGLE_WORLD_MOTION_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::MultiLevel, false, false, false) => MULTI_LEVEL_LOWERED_CALLEE,
        (AirIntersectionInstancing::MultiLevel, false, false, true) => {
            MULTI_LEVEL_MOTION_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::MultiLevel, true, false, false) => {
            MULTI_LEVEL_TRIANGLE_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::MultiLevel, true, false, true) => {
            MULTI_LEVEL_TRIANGLE_MOTION_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::MultiLevel, false, true, false) => {
            MULTI_LEVEL_WORLD_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::MultiLevel, false, true, true) => {
            MULTI_LEVEL_WORLD_MOTION_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::MultiLevel, true, true, false) => {
            MULTI_LEVEL_TRIANGLE_WORLD_LOWERED_CALLEE
        }
        (AirIntersectionInstancing::MultiLevel, true, true, true) => {
            MULTI_LEVEL_TRIANGLE_WORLD_MOTION_LOWERED_CALLEE
        }
        _ => return None,
    };
    let air_result_type = family.llvm_result_type();
    let lowered_result_type = format!(
        "{{ {} }}",
        family
            .result_fields()
            .into_iter()
            .map(|field| match field {
                AirIntersectionResultField::OpaquePointer => "i64",
                _ => field.llvm_type(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    );
    Some(Family {
        air_callee: format!("@{callee}("),
        lowered_callee,
        air_result_type,
        lowered_result_type,
    })
}

pub(in crate::native) fn lower_callback_free_triangle_queries(ll: &str) -> Option<String> {
    if !ll.contains("@air.intersect.") {
        return None;
    }

    let scoped_lines = llvm_line_scopes(ll);
    let lowered_results = scoped_lines
        .iter()
        .filter_map(|(line, scope)| {
            let family = lowering_family(line)?;
            line.contains(" = ").then_some((*scope, *line, family))
        })
        .filter_map(|(scope, line, family)| {
            line.split_once(" = ").map(|(result, _)| ScopedLowering {
                scope,
                result: result.trim().to_string(),
                family,
            })
        })
        .collect::<Vec<_>>();
    if lowered_results.is_empty() {
        return None;
    }

    let mut lowered_aggregates = lowered_results.clone();
    loop {
        let mut discovered = Vec::new();
        for &(line, scope) in &scoped_lines {
            let Some((defined, instruction)) = line.trim_start().split_once(" = ") else {
                continue;
            };
            if !aggregate_preserving_instruction(instruction)
                || lowered_aggregates
                    .iter()
                    .any(|lowering| lowering.scope == scope && lowering.result == defined.trim())
            {
                continue;
            }
            if let Some(source) = lowered_aggregates.iter().find(|lowering| {
                lowering.scope == scope
                    && instruction.contains(&lowering.family.air_result_type)
                    && line_uses_llvm_value(instruction, &lowering.result)
            }) {
                discovered.push(ScopedLowering {
                    scope,
                    result: defined.trim().to_string(),
                    family: source.family.clone(),
                });
            }
        }
        if discovered.is_empty() {
            break;
        }
        lowered_aggregates.extend(discovered);
    }

    let opaque_extractions = scoped_lines
        .iter()
        .filter_map(|(line, scope)| {
            let trimmed = line.trim_start();
            let (defined, instruction) = trimmed.split_once(" = ")?;
            instruction.starts_with("extractvalue ").then_some(())?;
            lowered_aggregates
                .iter()
                .any(|lowering| {
                    lowering.scope == *scope
                        && instruction.contains(&format!("{},", lowering.result))
                        && instruction
                            .split_once(&format!("{},", lowering.result))
                            .is_some_and(|(_, index)| index.trim() == "4")
                })
                .then(|| (*scope, defined.trim().to_string()))
        })
        .collect::<Vec<_>>();

    let mut output = String::with_capacity(ll.len() + TRIANGLE_QUERY_HELPER.len());
    for &(line, scope) in &scoped_lines {
        if opaque_extractions.iter().any(|(value_scope, value)| {
            *value_scope == scope && line.trim_start().starts_with(&format!("{value} ="))
        }) {
            continue;
        }
        let mut line = line.to_string();
        for (_, value) in opaque_extractions
            .iter()
            .filter(|(value_scope, _)| *value_scope == scope)
        {
            line = replace_llvm_value(&line, value, "null");
        }
        let lowered = lowered_results.iter().find(|lowering| {
            lowering.scope == scope
                && line
                    .trim_start()
                    .starts_with(&format!("{} =", lowering.result))
        });
        if let Some(lowering) = lowered {
            output.push_str(
                &line
                    .replacen(
                        &lowering.family.air_callee,
                        lowering.family.lowered_callee,
                        1,
                    )
                    .replacen(
                        &lowering.family.air_result_type,
                        &lowering.family.lowered_result_type,
                        1,
                    ),
            );
        } else if let Some(lowering) = lowered_aggregates.iter().find(|lowering| {
            lowering.scope == scope
                && line.contains(&lowering.family.air_result_type)
                && line_uses_llvm_value(&line, &lowering.result)
        }) {
            output.push_str(&line.replacen(
                &lowering.family.air_result_type,
                &lowering.family.lowered_result_type,
                1,
            ));
        } else {
            output.push_str(&line);
        }
        output.push('\n');
    }
    output.push_str(TRIANGLE_QUERY_HELPER);
    output.push_str(&world_space_query_helpers());
    output.push_str(&multi_level_query_helpers());
    Some(output)
}

fn replace_llvm_value(line: &str, old: &str, new: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut cursor = 0usize;
    while let Some(relative) = line[cursor..].find(old) {
        let start = cursor + relative;
        let end = start + old.len();
        let boundary = |byte: Option<&u8>| {
            byte.is_none_or(|byte| {
                !byte.is_ascii_alphanumeric() && !matches!(byte, b'_' | b'.' | b'$' | b'-')
            })
        };
        if boundary(
            start
                .checked_sub(1)
                .and_then(|index| line.as_bytes().get(index)),
        ) && boundary(line.as_bytes().get(end))
        {
            output.push_str(&line[cursor..start]);
            output.push_str(new);
            cursor = end;
        } else {
            output.push_str(&line[cursor..end]);
            cursor = end;
        }
    }
    output.push_str(&line[cursor..]);
    output
}

fn world_space_query_helpers() -> String {
    const PARAMETERS: &str = "<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a9, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i1 %a18, i1 %a19";
    const ARGUMENTS: &str = "<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a9, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i1 %a18, i1 %a19";
    const MOTION_PARAMETERS: &str = "<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, float %motion_time, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i1 %a19, i1 %a20";
    const MOTION_ARGUMENTS: &str = "<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, float %motion_time, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i1 %a19, i1 %a20";
    let base = ["i32", "float", "i32", "i32", "i64", "i32", "i32"];
    let triangle = [
        "i32",
        "float",
        "i32",
        "i32",
        "i64",
        "i32",
        "i32",
        "<2 x float>",
        "i1",
    ];
    [
        world_space_query_helper(
            "metal2vulkan.intersect.callback_free.instancing.world_space_data",
            "metal2vulkan.intersect.callback_free.instancing",
            &base,
            PARAMETERS,
            ARGUMENTS,
        ),
        world_space_query_helper(
            "metal2vulkan.intersect.callback_free.instancing.world_space_data.motion",
            "metal2vulkan.intersect.callback_free.instancing.motion",
            &base,
            MOTION_PARAMETERS,
            MOTION_ARGUMENTS,
        ),
        world_space_query_helper(
            "metal2vulkan.intersect.callback_free.instancing.triangle_data.world_space_data",
            "metal2vulkan.intersect.callback_free.instancing.triangle_data",
            &triangle,
            PARAMETERS,
            ARGUMENTS,
        ),
        world_space_query_helper(
            "metal2vulkan.intersect.callback_free.instancing.triangle_data.world_space_data.motion",
            "metal2vulkan.intersect.callback_free.instancing.triangle_data.motion",
            &triangle,
            MOTION_PARAMETERS,
            MOTION_ARGUMENTS,
        ),
    ]
    .concat()
}

fn world_space_query_helper(
    name: &str,
    base_name: &str,
    base_fields: &[&str],
    parameters: &str,
    arguments: &str,
) -> String {
    let base_type = format!("{{ {} }}", base_fields.join(", "));
    let result_type = format!(
        "{{ {}, {} }}",
        base_fields.join(", "),
        std::iter::repeat_n("<3 x float>", 8)
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut helper = format!(
        "\ndefine internal {result_type} @{name}({parameters}) {{\nentry:\n  %base = call {base_type} @{base_name}({arguments})\n"
    );
    for (index, field_type) in base_fields.iter().enumerate() {
        helper.push_str(&format!(
            "  %field{index} = extractvalue {base_type} %base, {index}\n"
        ));
        let previous = if index == 0 {
            "poison".to_string()
        } else {
            format!("%result{}", index - 1)
        };
        helper.push_str(&format!(
            "  %result{index} = insertvalue {result_type} {previous}, {field_type} %field{index}, {index}\n"
        ));
    }
    let identity_columns = [
        "<float 1.000000e+00, float 0.000000e+00, float 0.000000e+00>",
        "<float 0.000000e+00, float 1.000000e+00, float 0.000000e+00>",
        "<float 0.000000e+00, float 0.000000e+00, float 1.000000e+00>",
        "<float 0x8000000000000000, float 0x8000000000000000, float 0x8000000000000000>",
        "<float 1.000000e+00, float 0.000000e+00, float 0.000000e+00>",
        "<float 0.000000e+00, float 1.000000e+00, float 0.000000e+00>",
        "<float 0.000000e+00, float 0.000000e+00, float 1.000000e+00>",
        "zeroinitializer",
    ];
    let mut previous = format!("%result{}", base_fields.len() - 1);
    for (relative, column) in identity_columns.iter().enumerate() {
        let index = base_fields.len() + relative;
        let result = format!("%result{index}");
        helper.push_str(&format!(
            "  {result} = insertvalue {result_type} {previous}, <3 x float> {column}, {index}\n"
        ));
        previous = result;
    }
    helper.push_str(&format!("  ret {result_type} {previous}\n}}\n"));
    helper
}

fn multi_level_query_helpers() -> String {
    const PARAMETERS: &str = "<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i8 %max_levels, ptr %instance_ids, ptr %user_instance_ids, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i32 %a19, i32 %a20, i1 %a21, i1 %a22";
    const ARGUMENTS: &str = "<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i8 %max_levels, ptr %instance_ids, ptr %user_instance_ids, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i32 %a19, i32 %a20, i1 %a21, i1 %a22";
    const SINGLE_ARGUMENTS: &str = "<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i32 %a19, i32 %a20, i1 %a21, i1 %a22";
    const MOTION_PARAMETERS: &str = "<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, float %motion_time, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i8 %max_levels, ptr %instance_ids, ptr %user_instance_ids, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i32 %a19, i32 %a20, i32 %a21, i1 %a22, i1 %a23";
    const MOTION_ARGUMENTS: &str = "<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, float %motion_time, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i8 %max_levels, ptr %instance_ids, ptr %user_instance_ids, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i32 %a19, i32 %a20, i32 %a21, i1 %a22, i1 %a23";
    const SINGLE_MOTION_ARGUMENTS: &str = "<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, float %motion_time, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i32 %a19, i32 %a20, i32 %a21, i1 %a22, i1 %a23";
    let base = ["i32", "float", "i32", "i32", "i64", "i8"];
    let triangle = [
        "i32",
        "float",
        "i32",
        "i32",
        "i64",
        "i8",
        "<2 x float>",
        "i1",
    ];
    let plain = multi_level_query_helper(
        "metal2vulkan.intersect.callback_free.multi_level_instancing",
        "metal2vulkan.intersect.callback_free.instancing",
        false,
        PARAMETERS,
        SINGLE_ARGUMENTS,
    );
    let motion = multi_level_query_helper(
        "metal2vulkan.intersect.callback_free.multi_level_instancing.motion",
        "metal2vulkan.intersect.callback_free.instancing.motion",
        false,
        MOTION_PARAMETERS,
        SINGLE_MOTION_ARGUMENTS,
    );
    let triangle_plain = multi_level_query_helper(
        "metal2vulkan.intersect.callback_free.multi_level_instancing.triangle_data",
        "metal2vulkan.intersect.callback_free.instancing.triangle_data",
        true,
        PARAMETERS,
        SINGLE_ARGUMENTS,
    );
    let triangle_motion = multi_level_query_helper(
        "metal2vulkan.intersect.callback_free.multi_level_instancing.triangle_data.motion",
        "metal2vulkan.intersect.callback_free.instancing.triangle_data.motion",
        true,
        MOTION_PARAMETERS,
        SINGLE_MOTION_ARGUMENTS,
    );
    [
        plain,
        motion,
        triangle_plain,
        triangle_motion,
        world_space_query_helper(
            "metal2vulkan.intersect.callback_free.multi_level_instancing.world_space_data",
            "metal2vulkan.intersect.callback_free.multi_level_instancing",
            &base,
            PARAMETERS,
            ARGUMENTS,
        ),
        world_space_query_helper(
            "metal2vulkan.intersect.callback_free.multi_level_instancing.world_space_data.motion",
            "metal2vulkan.intersect.callback_free.multi_level_instancing.motion",
            &base,
            MOTION_PARAMETERS,
            MOTION_ARGUMENTS,
        ),
        world_space_query_helper(
            "metal2vulkan.intersect.callback_free.multi_level_instancing.triangle_data.world_space_data",
            "metal2vulkan.intersect.callback_free.multi_level_instancing.triangle_data",
            &triangle,
            PARAMETERS,
            ARGUMENTS,
        ),
        world_space_query_helper(
            "metal2vulkan.intersect.callback_free.multi_level_instancing.triangle_data.world_space_data.motion",
            "metal2vulkan.intersect.callback_free.multi_level_instancing.triangle_data.motion",
            &triangle,
            MOTION_PARAMETERS,
            MOTION_ARGUMENTS,
        ),
    ]
    .concat()
}

fn multi_level_query_helper(
    name: &str,
    single_name: &str,
    triangle_data: bool,
    parameters: &str,
    single_arguments: &str,
) -> String {
    let single_fields = if triangle_data {
        vec![
            "i32",
            "float",
            "i32",
            "i32",
            "i64",
            "i32",
            "i32",
            "<2 x float>",
            "i1",
        ]
    } else {
        vec!["i32", "float", "i32", "i32", "i64", "i32", "i32"]
    };
    let mut result_fields = vec!["i32", "float", "i32", "i32", "i64", "i8"];
    if triangle_data {
        result_fields.extend(["<2 x float>", "i1"]);
    }
    let single_type = format!("{{ {} }}", single_fields.join(", "));
    let result_type = format!("{{ {} }}", result_fields.join(", "));
    let mut helper = format!(
        "\ndefine internal {result_type} @{name}({parameters}) {{\nentry:\n  %single = call {single_type} @{single_name}({single_arguments})\n  %type = extractvalue {single_type} %single, 0\n  %has_hit = icmp ne i32 %type, 0\n  br i1 %has_hit, label %write_path, label %finish\n\nwrite_path:\n  store i32 0, ptr %instance_ids, align 4\n  store i32 0, ptr %user_instance_ids, align 4\n  br label %finish\n\nfinish:\n"
    );
    let mut previous = "poison".to_string();
    for (source_index, field_type) in single_fields.iter().copied().take(5).enumerate() {
        helper.push_str(&format!(
            "  %field{source_index} = extractvalue {single_type} %single, {source_index}\n"
        ));
        let result = format!("%result{source_index}");
        helper.push_str(&format!(
            "  {result} = insertvalue {result_type} {previous}, {field_type} %field{source_index}, {source_index}\n"
        ));
        previous = result;
    }
    helper.push_str(&format!(
        "  %instance_count = select i1 %has_hit, i8 1, i8 0\n  %result5 = insertvalue {result_type} {previous}, i8 %instance_count, 5\n"
    ));
    previous = "%result5".into();
    if triangle_data {
        for (source_index, result_index) in [(7, 6), (8, 7)] {
            let field_type = single_fields[source_index];
            helper.push_str(&format!(
                "  %field{result_index} = extractvalue {single_type} %single, {source_index}\n"
            ));
            let result = format!("%result{result_index}");
            helper.push_str(&format!(
                "  {result} = insertvalue {result_type} {previous}, {field_type} %field{result_index}, {result_index}\n"
            ));
            previous = result;
        }
    }
    helper.push_str(&format!("  ret {result_type} {previous}\n}}\n"));
    helper
}

pub(crate) fn all_air_intersection_calls_are_lowerable(ll: &str) -> bool {
    let has_intersection = ll
        .lines()
        .any(|line| line.contains(" = ") && line.contains("@air.intersect."));
    if !has_intersection {
        return true;
    }
    lower_callback_free_triangle_queries(ll).is_some_and(|lowered| {
        !lowered
            .lines()
            .any(|line| line.contains(" = ") && line.contains("@air.intersect."))
    })
}

const TRIANGLE_QUERY_HELPER: &str = r#"
define internal { i32, float, i32, i32, i64, <2 x float>, i1 } @metal2vulkan.intersect.callback_free.triangle_data(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %front_facing_winding, i32 %triangle_cull_mode, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i1 %a17) {
entry:
  %triangle_count = load i32, ptr addrspace(1) %acceleration_structure, align 4
  %ox = extractelement <3 x float> %origin, i32 0
  %oy = extractelement <3 x float> %origin, i32 1
  %oz = extractelement <3 x float> %origin, i32 2
  %dx = extractelement <3 x float> %direction, i32 0
  %dy = extractelement <3 x float> %direction, i32 1
  %dz = extractelement <3 x float> %direction, i32 2
  br label %loop

loop:
  %index = phi i32 [ 0, %entry ], [ %next_index, %body ]
  %best_distance = phi float [ %max_distance, %entry ], [ %selected_distance, %body ]
  %best_primitive = phi i32 [ -1, %entry ], [ %selected_primitive, %body ]
  %best_u = phi float [ 0.000000e+00, %entry ], [ %selected_u, %body ]
  %best_v = phi float [ 0.000000e+00, %entry ], [ %selected_v, %body ]
  %best_front = phi i1 [ false, %entry ], [ %selected_front, %body ]
  %in_range = icmp ult i32 %index, %triangle_count
  br i1 %in_range, label %body, label %exit

body:
  %index64 = zext i32 %index to i64
  %triangle_offset = mul i64 %index64, 36
  %base_offset = add i64 %triangle_offset, 8
  %v0x_offset = add i64 %base_offset, 0
  %v0x_ptr = getelementptr i8, ptr addrspace(1) %acceleration_structure, i64 %v0x_offset
  %v0x = load float, ptr addrspace(1) %v0x_ptr, align 4
  %v0y_offset = add i64 %base_offset, 4
  %v0y_ptr = getelementptr i8, ptr addrspace(1) %acceleration_structure, i64 %v0y_offset
  %v0y = load float, ptr addrspace(1) %v0y_ptr, align 4
  %v0z_offset = add i64 %base_offset, 8
  %v0z_ptr = getelementptr i8, ptr addrspace(1) %acceleration_structure, i64 %v0z_offset
  %v0z = load float, ptr addrspace(1) %v0z_ptr, align 4
  %v1x_offset = add i64 %base_offset, 12
  %v1x_ptr = getelementptr i8, ptr addrspace(1) %acceleration_structure, i64 %v1x_offset
  %v1x = load float, ptr addrspace(1) %v1x_ptr, align 4
  %v1y_offset = add i64 %base_offset, 16
  %v1y_ptr = getelementptr i8, ptr addrspace(1) %acceleration_structure, i64 %v1y_offset
  %v1y = load float, ptr addrspace(1) %v1y_ptr, align 4
  %v1z_offset = add i64 %base_offset, 20
  %v1z_ptr = getelementptr i8, ptr addrspace(1) %acceleration_structure, i64 %v1z_offset
  %v1z = load float, ptr addrspace(1) %v1z_ptr, align 4
  %v2x_offset = add i64 %base_offset, 24
  %v2x_ptr = getelementptr i8, ptr addrspace(1) %acceleration_structure, i64 %v2x_offset
  %v2x = load float, ptr addrspace(1) %v2x_ptr, align 4
  %v2y_offset = add i64 %base_offset, 28
  %v2y_ptr = getelementptr i8, ptr addrspace(1) %acceleration_structure, i64 %v2y_offset
  %v2y = load float, ptr addrspace(1) %v2y_ptr, align 4
  %v2z_offset = add i64 %base_offset, 32
  %v2z_ptr = getelementptr i8, ptr addrspace(1) %acceleration_structure, i64 %v2z_offset
  %v2z = load float, ptr addrspace(1) %v2z_ptr, align 4

  %e1x = fsub float %v1x, %v0x
  %e1y = fsub float %v1y, %v0y
  %e1z = fsub float %v1z, %v0z
  %e2x = fsub float %v2x, %v0x
  %e2y = fsub float %v2y, %v0y
  %e2z = fsub float %v2z, %v0z
  %dy_e2z = fmul float %dy, %e2z
  %dz_e2y = fmul float %dz, %e2y
  %px = fsub float %dy_e2z, %dz_e2y
  %dz_e2x = fmul float %dz, %e2x
  %dx_e2z = fmul float %dx, %e2z
  %py = fsub float %dz_e2x, %dx_e2z
  %dx_e2y = fmul float %dx, %e2y
  %dy_e2x = fmul float %dy, %e2x
  %pz = fsub float %dx_e2y, %dy_e2x
  %e1x_px = fmul float %e1x, %px
  %e1y_py = fmul float %e1y, %py
  %det_xy = fadd float %e1x_px, %e1y_py
  %e1z_pz = fmul float %e1z, %pz
  %det = fadd float %det_xy, %e1z_pz
  %det_nonzero = fcmp one float %det, 0.000000e+00
  %inverse_det = fdiv float 1.000000e+00, %det

  %tx = fsub float %ox, %v0x
  %ty = fsub float %oy, %v0y
  %tz = fsub float %oz, %v0z
  %tx_px = fmul float %tx, %px
  %ty_py = fmul float %ty, %py
  %u_xy = fadd float %tx_px, %ty_py
  %tz_pz = fmul float %tz, %pz
  %u_sum = fadd float %u_xy, %tz_pz
  %u = fmul float %u_sum, %inverse_det

  %ty_e1z = fmul float %ty, %e1z
  %tz_e1y = fmul float %tz, %e1y
  %qx = fsub float %ty_e1z, %tz_e1y
  %tz_e1x = fmul float %tz, %e1x
  %tx_e1z = fmul float %tx, %e1z
  %qy = fsub float %tz_e1x, %tx_e1z
  %tx_e1y = fmul float %tx, %e1y
  %ty_e1x = fmul float %ty, %e1x
  %qz = fsub float %tx_e1y, %ty_e1x
  %dx_qx = fmul float %dx, %qx
  %dy_qy = fmul float %dy, %qy
  %v_xy = fadd float %dx_qx, %dy_qy
  %dz_qz = fmul float %dz, %qz
  %v_sum = fadd float %v_xy, %dz_qz
  %v = fmul float %v_sum, %inverse_det
  %e2x_qx = fmul float %e2x, %qx
  %e2y_qy = fmul float %e2y, %qy
  %distance_xy = fadd float %e2x_qx, %e2y_qy
  %e2z_qz = fmul float %e2z, %qz
  %distance_sum = fadd float %distance_xy, %e2z_qz
  %distance = fmul float %distance_sum, %inverse_det

  %u_nonnegative = fcmp oge float %u, 0.000000e+00
  %v_nonnegative = fcmp oge float %v, 0.000000e+00
  %uv = fadd float %u, %v
  %uv_in_triangle = fcmp ole float %uv, 1.000000e+00
  %after_min = fcmp oge float %distance, %min_distance
  %before_best = fcmp ole float %distance, %best_distance
  %valid0 = and i1 %det_nonzero, %u_nonnegative
  %valid1 = and i1 %valid0, %v_nonnegative
  %valid2 = and i1 %valid1, %uv_in_triangle
  %valid3 = and i1 %valid2, %after_min
  %valid4 = and i1 %valid3, %before_best
  %det_positive = fcmp ogt float %det, 0.000000e+00
  %det_negative = fcmp olt float %det, 0.000000e+00
  ; Metal's `winding` says which sign of the Moller-Trumbore determinant is the front face: 0 is
  ; clockwise, which is the default `intersection_params` holds, and 1 is counterclockwise, which
  ; swaps the two. It has to be read before anything that names a face -- the cull mode below and
  ; the front-facing result field are the same question asked twice.
  %counterclockwise = icmp eq i32 %front_facing_winding, 1
  %clockwise = icmp ne i32 %front_facing_winding, 1
  %front_cw = and i1 %clockwise, %det_positive
  %front_ccw = and i1 %counterclockwise, %det_negative
  %front = or i1 %front_cw, %front_ccw
  %back_cw = and i1 %clockwise, %det_negative
  %back_ccw = and i1 %counterclockwise, %det_positive
  %back = or i1 %back_cw, %back_ccw
  ; Metal's `triangle_cull_mode`: 0 none, 1 front, 2 back. Each mode only removes hits, so keeping
  ; one means the mode either is not that mode or names the face this hit is not on.
  %not_cull_front = icmp ne i32 %triangle_cull_mode, 1
  %not_cull_back = icmp ne i32 %triangle_cull_mode, 2
  %keep_front = or i1 %not_cull_front, %back
  %keep_back = or i1 %not_cull_back, %front
  %kept = and i1 %keep_front, %keep_back
  %valid = and i1 %valid4, %kept
  %selected_distance = select i1 %valid, float %distance, float %best_distance
  %selected_primitive = select i1 %valid, i32 %index, i32 %best_primitive
  %selected_u = select i1 %valid, float %u, float %best_u
  %selected_v = select i1 %valid, float %v, float %best_v
  %selected_front = select i1 %valid, i1 %front, i1 %best_front
  %next_index = add i32 %index, 1
  br label %loop

exit:
  %has_hit = icmp ne i32 %best_primitive, -1
  %intersection_type = select i1 %has_hit, i32 1, i32 0
  %bary0 = insertelement <2 x float> poison, float %best_u, i32 0
  %barycentrics = insertelement <2 x float> %bary0, float %best_v, i32 1
  %result0 = insertvalue { i32, float, i32, i32, i64, <2 x float>, i1 } poison, i32 %intersection_type, 0
  %result1 = insertvalue { i32, float, i32, i32, i64, <2 x float>, i1 } %result0, float %best_distance, 1
  %result2 = insertvalue { i32, float, i32, i32, i64, <2 x float>, i1 } %result1, i32 %best_primitive, 2
  %result3 = insertvalue { i32, float, i32, i32, i64, <2 x float>, i1 } %result2, i32 0, 3
  %result4 = insertvalue { i32, float, i32, i32, i64, <2 x float>, i1 } %result3, i64 0, 4
  %result5 = insertvalue { i32, float, i32, i32, i64, <2 x float>, i1 } %result4, <2 x float> %barycentrics, 5
  %result6 = insertvalue { i32, float, i32, i32, i64, <2 x float>, i1 } %result5, i1 %best_front, 6
  ret { i32, float, i32, i32, i64, <2 x float>, i1 } %result6
}

define internal { i32, float, i32, i32, i64 } @metal2vulkan.intersect.callback_free.primitive_motion(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, float %motion_time, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a9, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i1 %a18) {
entry:
  %full = call { i32, float, i32, i32, i64, <2 x float>, i1 } @metal2vulkan.intersect.callback_free.triangle_data(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a9, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i1 %a18)
  %type = extractvalue { i32, float, i32, i32, i64, <2 x float>, i1 } %full, 0
  %distance = extractvalue { i32, float, i32, i32, i64, <2 x float>, i1 } %full, 1
  %primitive = extractvalue { i32, float, i32, i32, i64, <2 x float>, i1 } %full, 2
  %geometry = extractvalue { i32, float, i32, i32, i64, <2 x float>, i1 } %full, 3
  %opaque = extractvalue { i32, float, i32, i32, i64, <2 x float>, i1 } %full, 4
  %result0 = insertvalue { i32, float, i32, i32, i64 } poison, i32 %type, 0
  %result1 = insertvalue { i32, float, i32, i32, i64 } %result0, float %distance, 1
  %result2 = insertvalue { i32, float, i32, i32, i64 } %result1, i32 %primitive, 2
  %result3 = insertvalue { i32, float, i32, i32, i64 } %result2, i32 %geometry, 3
  %result4 = insertvalue { i32, float, i32, i32, i64 } %result3, i64 %opaque, 4
  ret { i32, float, i32, i32, i64 } %result4
}

define internal { i32, float, i32, i32, i64, <2 x float>, i1 } @metal2vulkan.intersect.callback_free.triangle_data.primitive_motion(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, float %motion_time, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a9, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i1 %a18) {
entry:
  %full = call { i32, float, i32, i32, i64, <2 x float>, i1 } @metal2vulkan.intersect.callback_free.triangle_data(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a9, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i1 %a18)
  ret { i32, float, i32, i32, i64, <2 x float>, i1 } %full
}

define internal { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } @metal2vulkan.intersect.callback_free.instancing.triangle_data(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %front_facing_winding, i32 %triangle_cull_mode, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i1 %a18, i1 %a19) {
entry:
  %instance_count = load i32, ptr addrspace(1) %acceleration_structure, align 4
  %has_instance = icmp ne i32 %instance_count, 0
  %mask_matches = icmp ne i32 %instance_mask, 0
  %ox = extractelement <3 x float> %origin, i32 0
  %oy = extractelement <3 x float> %origin, i32 1
  %oz = extractelement <3 x float> %origin, i32 2
  %dx = extractelement <3 x float> %direction, i32 0
  %dy = extractelement <3 x float> %direction, i32 1
  %dz = extractelement <3 x float> %direction, i32 2
  %neg_two_dz = fmul float %dz, -2.000000e+00
  %two_dx = fmul float %dx, 2.000000e+00
  %pz = fsub float %two_dx, %dy
  %det = fmul float %neg_two_dz, 2.000000e+00
  %det_nonzero = fcmp one float %det, 0.000000e+00
  %inverse_det = fdiv float 1.000000e+00, %det
  %tx = fadd float %ox, 1.000000e+00
  %ty = fadd float %oy, 1.000000e+00
  %tx_px = fmul float %tx, %neg_two_dz
  %ty_dz = fmul float %ty, %dz
  %u_partial = fadd float %tx_px, %ty_dz
  %oz_pz = fmul float %oz, %pz
  %u_sum = fadd float %u_partial, %oz_pz
  %u = fmul float %u_sum, %inverse_det
  %two_dy = fmul float %dy, 2.000000e+00
  %dy_oz = fmul float %two_dy, %oz
  %two_dz_ty = fmul float %dz, %ty
  %neg_two_dz_ty = fmul float %two_dz_ty, -2.000000e+00
  %v_sum = fadd float %dy_oz, %neg_two_dz_ty
  %v = fmul float %v_sum, %inverse_det
  %four_oz = fmul float %oz, 4.000000e+00
  %distance = fmul float %four_oz, %inverse_det
  %u_nonnegative = fcmp oge float %u, 0.000000e+00
  %v_nonnegative = fcmp oge float %v, 0.000000e+00
  %uv = fadd float %u, %v
  %uv_in_triangle = fcmp ole float %uv, 1.000000e+00
  %after_min = fcmp oge float %distance, %min_distance
  %before_max = fcmp ole float %distance, %max_distance
  %valid0 = and i1 %has_instance, %mask_matches
  %valid1 = and i1 %valid0, %det_nonzero
  %valid2 = and i1 %valid1, %u_nonnegative
  %valid3 = and i1 %valid2, %v_nonnegative
  %valid4 = and i1 %valid3, %uv_in_triangle
  %valid5 = and i1 %valid4, %after_min
  %valid6 = and i1 %valid5, %before_max
  %det_positive = fcmp ogt float %det, 0.000000e+00
  %det_negative = fcmp olt float %det, 0.000000e+00
  ; Metal's `winding` says which sign of the Moller-Trumbore determinant is the front face: 0 is
  ; clockwise, which is the default `intersection_params` holds, and 1 is counterclockwise, which
  ; swaps the two. It has to be read before anything that names a face -- the cull mode below and
  ; the front-facing result field are the same question asked twice.
  %counterclockwise = icmp eq i32 %front_facing_winding, 1
  %clockwise = icmp ne i32 %front_facing_winding, 1
  %front_cw = and i1 %clockwise, %det_positive
  %front_ccw = and i1 %counterclockwise, %det_negative
  %front = or i1 %front_cw, %front_ccw
  %back_cw = and i1 %clockwise, %det_negative
  %back_ccw = and i1 %counterclockwise, %det_positive
  %back = or i1 %back_cw, %back_ccw
  ; Metal's `triangle_cull_mode`: 0 none, 1 front, 2 back. Each mode only removes hits, so keeping
  ; one means the mode either is not that mode or names the face this hit is not on.
  %not_cull_front = icmp ne i32 %triangle_cull_mode, 1
  %not_cull_back = icmp ne i32 %triangle_cull_mode, 2
  %keep_front = or i1 %not_cull_front, %back
  %keep_back = or i1 %not_cull_back, %front
  %kept = and i1 %keep_front, %keep_back
  %valid = and i1 %valid6, %kept
  %intersection_type = select i1 %valid, i32 1, i32 0
  %selected_distance = select i1 %valid, float %distance, float %max_distance
  %selected_front = and i1 %valid, %front
  %selected_u = select i1 %valid, float %u, float 0.000000e+00
  %selected_v = select i1 %valid, float %v, float 0.000000e+00
  %bary0 = insertelement <2 x float> poison, float %selected_u, i32 0
  %barycentrics = insertelement <2 x float> %bary0, float %selected_v, i32 1
  %result0 = insertvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } poison, i32 %intersection_type, 0
  %result1 = insertvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %result0, float %selected_distance, 1
  %result2 = insertvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %result1, i32 0, 2
  %result3 = insertvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %result2, i32 0, 3
  %result4 = insertvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %result3, i64 0, 4
  %result5 = insertvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %result4, i32 0, 5
  %result6 = insertvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %result5, i32 0, 6
  %result7 = insertvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %result6, <2 x float> %barycentrics, 7
  %result8 = insertvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %result7, i1 %selected_front, 8
  ret { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %result8
}

define internal { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } @metal2vulkan.intersect.callback_free.instancing.triangle_data.motion(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, float %motion_time, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i1 %a19, i1 %a20) {
entry:
  %full = call { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } @metal2vulkan.intersect.callback_free.instancing.triangle_data(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i1 %a19, i1 %a20)
  ret { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full
}

define internal { i32, float, i32, i32, i64, i32, i32 } @metal2vulkan.intersect.callback_free.instancing(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a9, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i1 %a18, i1 %a19) {
entry:
  %full = call { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } @metal2vulkan.intersect.callback_free.instancing.triangle_data(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a9, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i1 %a18, i1 %a19)
  %type = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 0
  %distance = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 1
  %primitive = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 2
  %geometry = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 3
  %opaque = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 4
  %instance_id = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 5
  %user_instance_id = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 6
  %result0 = insertvalue { i32, float, i32, i32, i64, i32, i32 } poison, i32 %type, 0
  %result1 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result0, float %distance, 1
  %result2 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result1, i32 %primitive, 2
  %result3 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result2, i32 %geometry, 3
  %result4 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result3, i64 %opaque, 4
  %result5 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result4, i32 %instance_id, 5
  %result6 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result5, i32 %user_instance_id, 6
  ret { i32, float, i32, i32, i64, i32, i32 } %result6
}

define internal { i32, float, i32, i32, i64, i32, i32 } @metal2vulkan.intersect.callback_free.instancing.motion(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, float %motion_time, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i1 %a19, i1 %a20) {
entry:
  %full = call { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } @metal2vulkan.intersect.callback_free.instancing.triangle_data.motion(<3 x float> %origin, <3 x float> %direction, float %min_distance, float %max_distance, ptr addrspace(1) %acceleration_structure, i32 %instance_mask, float %motion_time, ptr addrspace(1) %table, ptr %payload, i64 %payload_stride, i32 %a10, i32 %a11, i32 %a12, i32 %a13, i32 %a14, i32 %a15, i32 %a16, i32 %a17, i32 %a18, i1 %a19, i1 %a20)
  %type = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 0
  %distance = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 1
  %primitive = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 2
  %geometry = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 3
  %opaque = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 4
  %instance_id = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 5
  %user_instance_id = extractvalue { i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 } %full, 6
  %result0 = insertvalue { i32, float, i32, i32, i64, i32, i32 } poison, i32 %type, 0
  %result1 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result0, float %distance, 1
  %result2 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result1, i32 %primitive, 2
  %result3 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result2, i32 %geometry, 3
  %result4 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result3, i64 %opaque, 4
  %result5 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result4, i32 %instance_id, 5
  %result6 = insertvalue { i32, float, i32, i32, i64, i32, i32 } %result5, i32 %user_instance_id, 6
  ret { i32, float, i32, i32, i64, i32, i32 } %result6
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_free_table_parameter_is_rewritten_but_callback_family_is_not() {
        let ll = "%a = call { i32, float, i32, i32, ptr addrspace(1), <2 x float>, i1 } @air.intersect.triangle_data(i32 0, i32 0, i32 0, i32 0, ptr %as, ptr addrspace(1) %table)\n%b = call { i32, float, i32, i32, ptr addrspace(1), <2 x float>, i1 } @air.intersect.intersection_function_buffer.triangle_data(i32 0, i32 0, i32 0, i32 0, ptr %as, ptr addrspace(1) %table, ptr %payload, i64 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 0, i1 false)\n";
        let lowered = lower_callback_free_triangle_queries(ll).unwrap();
        assert_eq!(lowered.matches(TRIANGLE_LOWERED_CALLEE).count(), 4);
        assert!(lowered.contains("@air.intersect.intersection_function_buffer.triangle_data("));
        assert!(!all_air_intersection_calls_are_lowerable(ll));
    }

    #[test]
    fn null_table_primitive_motion_uses_the_shared_triangle_core() {
        let ll = "%null = call ptr addrspace(1) @air.get_null_intersection_function_table()\n%hit = call { i32, float, i32, i32, ptr addrspace(1) } @air.intersect.primitive_motion(i32 0, i32 0, i32 0, i32 0, ptr %as, float 0.0, ptr addrspace(1) %null)\n%type = extractvalue { i32, float, i32, i32, ptr addrspace(1) } %hit, 0\n";
        let lowered = lower_callback_free_triangle_queries(ll).unwrap();
        assert!(lowered.contains(PRIMITIVE_MOTION_LOWERED_CALLEE));
        assert!(all_air_intersection_calls_are_lowerable(ll));
    }

    #[test]
    fn triangle_data_and_primitive_motion_compose_without_a_symbol_table_entry() {
        let ll = "%null = call ptr addrspace(1) @air.get_null_intersection_function_table()\n%hit = call { i32, float, i32, i32, ptr addrspace(1), <2 x float>, i1 } @air.intersect.triangle_data.primitive_motion(i32 0, i32 0, i32 0, i32 0, ptr %as, float 0.0, ptr addrspace(1) %null)\n%bary = extractvalue { i32, float, i32, i32, ptr addrspace(1), <2 x float>, i1 } %hit, 5\n";
        let lowered = lower_callback_free_triangle_queries(ll).unwrap();
        assert!(lowered.contains(TRIANGLE_PRIMITIVE_MOTION_LOWERED_CALLEE));
        assert!(!lowered.contains("@air.intersect.triangle_data.primitive_motion("));
        assert!(all_air_intersection_calls_are_lowerable(ll));
    }

    #[test]
    fn triangle_facing_reads_the_winding_and_the_cull_mode_air_passes_beside_it() {
        let plain = "%hit = call { i32, float, i32, i32, ptr addrspace(1), <2 x float>, i1 } \
@air.intersect.triangle_data(<3 x float> zeroinitializer, <3 x float> zeroinitializer, float 0.0, \
float 1.0, ptr addrspace(1) %as, ptr addrspace(1) %table, ptr null, i64 0, i32 1, i32 2, i32 0, \
i32 0, i32 0, i32 3, i32 -1, i32 -1, i32 0, i1 false)\n";
        let motion = "%hit = call { i32, float, i32, i32, ptr addrspace(1), <2 x float>, i1 } \
@air.intersect.triangle_data.primitive_motion(<3 x float> zeroinitializer, <3 x float> \
zeroinitializer, float 0.0, float 1.0, ptr addrspace(1) %as, float 0.0, ptr addrspace(1) %table, \
ptr null, i64 0, i32 1, i32 2, i32 0, i32 0, i32 0, i32 3, i32 -1, i32 -1, i32 0, i1 false)\n";
        let instanced = "%hit = call { i32, float, i32, i32, ptr addrspace(1), i32, i32, \
<2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float> zeroinitializer, \
<3 x float> zeroinitializer, float 0.0, float 1.0, ptr addrspace(1) %as, i32 255, \
ptr addrspace(1) %table, ptr null, i64 0, i32 1, i32 2, i32 0, i32 0, i32 0, i32 3, i32 -1, \
i32 -1, i32 0, i1 false, i1 false)\n";
        let tail = |line: &str| {
            line.split_once("i64 %payload_stride, ").map(|(_, rest)| {
                rest.trim_end_matches([')', ' ', '{'])
                    .split(", ")
                    .filter_map(|operand| operand.split_whitespace().last().map(str::to_string))
                    .collect::<Vec<_>>()
            })
        };
        for source in [plain, motion, instanced] {
            let lowered = lower_callback_free_triangle_queries(source).expect("lowered");
            let mut cores = 0usize;
            let mut wrappers = 0usize;
            let mut definition: Option<Vec<String>> = None;
            for line in lowered.lines() {
                let trimmed = line.trim_start();
                if line.starts_with("define internal") {
                    definition = tail(line);
                    if line.contains("callback_free.triangle_data(")
                        || line.contains("callback_free.instancing.triangle_data(")
                    {
                        cores += 1;
                        let names = definition.clone().expect("a core takes a payload stride");
                        assert_eq!(
                            &names[..2],
                            ["%front_facing_winding", "%triangle_cull_mode"],
                            "the two operands AIR puts after the payload stride are the winding \
                             and the cull mode: {line}"
                        );
                    }
                    continue;
                }
                if trimmed.starts_with("%full = call")
                    && trimmed.contains("@metal2vulkan.intersect.callback_free")
                {
                    wrappers += 1;
                    assert_eq!(
                        tail(line),
                        definition,
                        "a wrapper must forward the trailing operand block unshifted: {line}"
                    );
                }
            }
            assert!(cores >= 1, "no triangle core was emitted: {lowered}");
            assert!(wrappers >= 1, "no wrapper was emitted: {lowered}");
            assert!(
                !lowered.contains("%front = fcmp ogt float %det"),
                "facing must follow the winding: {lowered}"
            );
            assert!(
                lowered.contains("%front = or i1 %front_cw, %front_ccw"),
                "facing must follow the winding: {lowered}"
            );
            assert!(
                lowered.contains("%kept = and i1 %keep_front, %keep_back"),
                "a culled hit must not reach the result: {lowered}"
            );
        }
    }

    #[test]
    fn single_level_instancing_uses_the_authored_identity_instance_shadow() {
        let ll = "%hit = call { i32, float, i32, i32, ptr addrspace(1), i32, i32, <2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float> zeroinitializer, <3 x float> zeroinitializer, float 0.0, float 1.0, ptr addrspace(1) %as, i32 255, ptr addrspace(1) %table, ptr null, i64 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 -1, i32 -1, i32 0, i1 false, i1 false)\n%instance = extractvalue { i32, float, i32, i32, ptr addrspace(1), i32, i32, <2 x float>, i1 } %hit, 6\n";
        let lowered = lower_callback_free_triangle_queries(ll).unwrap();
        assert!(lowered.contains(INSTANCE_TRIANGLE_LOWERED_CALLEE));
        assert!(!lowered.contains("@air.intersect.instancing.triangle_data("));
        assert!(all_air_intersection_calls_are_lowerable(ll));
    }

    #[test]
    fn callback_free_result_type_flows_through_phi_joins() {
        let result_type = "{ i32, float, i32, i32, ptr addrspace(1), i32, i32, <2 x float>, i1 }";
        let call = |result: &str| {
            format!(
                "  {result} = call {result_type} @air.intersect.instancing.triangle_data(<3 x float> zeroinitializer, <3 x float> zeroinitializer, float 0.0, float 1.0, ptr addrspace(1) %as, i32 255, ptr addrspace(1) %table, ptr null, i64 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 1, i32 -1, i32 -1, i32 0, i1 false, i1 false)\n"
            )
        };
        let ll = format!(
            "define void @query(i1 %condition, ptr addrspace(1) %as, ptr addrspace(1) %table) {{\nentry:\n{}  br i1 %condition, label %left, label %right\nleft:\n{}  br label %join\nright:\n  br label %join\njoin:\n  %joined = phi {result_type} [ %left_hit, %left ], [ %entry_hit, %right ]\n  %bary = extractvalue {result_type} %joined, 7\n  ret void\n}}\n",
            call("%entry_hit"),
            call("%left_hit")
        );
        let lowered = lower_callback_free_triangle_queries(&ll).unwrap();
        let lowered_type = "{ i32, float, i32, i32, i64, i32, i32, <2 x float>, i1 }";
        assert!(lowered.contains(&format!("%joined = phi {lowered_type}")));
        assert!(lowered.contains(&format!("%bary = extractvalue {lowered_type} %joined, 7")));
        assert!(!lowered.contains(&format!("%joined = phi {result_type}")));
    }

    #[test]
    fn callback_free_opaque_pointer_is_structurally_null() {
        let ll = "%hit = call { i32, float, i32, i32, ptr addrspace(1), i8 } @air.intersect.multi_level_instancing(<3 x float> zeroinitializer, <3 x float> zeroinitializer, float 0.0, float 1.0, ptr addrspace(1) %as, i32 255, ptr addrspace(1) %table, ptr null, i64 0, i8 2, ptr %ids, ptr %user_ids, i32 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 -1, i32 -1, i32 0, i1 false, i1 false)\n%opaque = extractvalue { i32, float, i32, i32, ptr addrspace(1), i8 } %hit, 4\n%copy = insertvalue { ptr addrspace(1) } poison, ptr addrspace(1) %opaque, 0\n%read = extractvalue { ptr addrspace(1) } %copy, 0\n%is_null = icmp eq ptr addrspace(1) %read, null\n";
        let lowered = lower_callback_free_triangle_queries(ll).unwrap();
        assert!(!lowered.contains(
            "%opaque = extractvalue { i32, float, i32, i32, ptr addrspace(1), i8 } %hit, 4"
        ));
        assert!(lowered
            .contains("%copy = insertvalue { ptr addrspace(1) } poison, ptr addrspace(1) null, 0"));
        assert!(all_air_intersection_calls_are_lowerable(ll));
    }

    #[test]
    fn local_numeric_result_rewrite_cannot_corrupt_another_function() {
        let ll = r#"define internal void @unrelated(i1 %condition) {
entry:
  br i1 %condition, label %43, label %exit

43:
  %44 = add i32 1, 2
  br label %exit

exit:
  ret void
}

define internal void @query(ptr addrspace(1) %as, ptr addrspace(1) %table, ptr %ids, ptr %user_ids) {
entry:
  %38 = call { i32, float, i32, i32, ptr addrspace(1), i8 } @air.intersect.multi_level_instancing(<3 x float> zeroinitializer, <3 x float> zeroinitializer, float 0.0, float 1.0, ptr addrspace(1) %as, i32 255, ptr addrspace(1) %table, ptr null, i64 0, i8 2, ptr %ids, ptr %user_ids, i32 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 -1, i32 -1, i32 0, i1 false, i1 false)
  %43 = extractvalue { i32, float, i32, i32, ptr addrspace(1), i8 } %38, 4
  %44 = icmp eq ptr addrspace(1) %43, null
  ret void
}
"#;
        let lowered = lower_callback_free_triangle_queries(ll).unwrap();
        assert!(lowered.contains("br i1 %condition, label %43, label %exit"));
        assert!(lowered.contains("\n43:\n  %44 = add i32 1, 2"));
        assert!(lowered.contains("%44 = icmp eq ptr addrspace(1) null, null"));
        assert!(!lowered
            .contains("%43 = extractvalue { i32, float, i32, i32, ptr addrspace(1), i8 } %38, 4"));
    }
}
