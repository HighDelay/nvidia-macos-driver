use metal2vulkan::{disassemble, passes::Stage, translate_sanitized_native_reflected};

const AIR: &str = r#"
target triple = "air64_v29-apple-macosx27.0.0"
define <2 x float> @positions(i32 %sample) {
entry:
  %p = call <2 x float> @air.get_sample_position.v2f32(i32 %sample, i32 0)
  %q = call <2 x float> @air.get_sample_position.v2f32(i32 0, i32 0)
  %sum = fadd <2 x float> %p, %q
  ret <2 x float> %sum
}
declare <2 x float> @air.get_sample_position.v2f32(i32, i32)
!air.fragment = !{!0}
!0 = !{ptr @positions, !1, !3}
!1 = !{!2}
!2 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float2", !"air.arg_name", !"color"}
!3 = !{!4}
!4 = !{i32 0, !"air.sample_id", !"air.arg_type_name", !"uint", !"air.arg_name", !"sample"}
"#;

#[test]
fn fragment_sample_positions_use_one_runtime_indexed_payload() {
    let tmp = std::env::temp_dir().join(format!("sample_position_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let (bytes, reflection) =
        translate_sanitized_native_reflected(AIR, Stage::Fragment, &tmp, Default::default())
            .unwrap();
    let text = disassemble(&bytes).unwrap();
    assert_bounded_dynamic_index_dataflow(&bytes);
    let range = reflection
        .fragment_sample_positions
        .expect("runtime consumer contract");
    assert_eq!(
        (range.offset, range.size, range.positions, range.stride),
        (96, 64, 8, 8)
    );
    assert_eq!(
        text.lines()
            .filter(|l| l.contains("OpVariable") && l.contains("PushConstant"))
            .count(),
        1,
        "{text}"
    );
    assert!(text.contains("ArrayStride 8"), "{text}");
    assert!(text.contains("Offset 96"), "{text}");
    assert!(text.contains("BuiltIn SampleId"), "{text}");
    assert!(!text.contains("OpFunctionCall"), "{text}");
    assert!(
        text.lines().filter(|l| l.contains("OpAccessChain")).count() >= 2,
        "{text}"
    );
    let spv = tmp.join("positions.spv");
    std::fs::write(&spv, bytes).unwrap();
    let output = std::process::Command::new("spirv-val")
        .args(["--target-env", "vulkan1.2"])
        .arg(spv)
        .output()
        .expect("required SPIR-V validator");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn fragment_sample_positions_reject_unknown_flags() {
    let ll = AIR.replace("i32 %sample, i32 0)", "i32 %sample, i32 1)");
    let tmp = std::env::temp_dir();
    let error =
        translate_sanitized_native_reflected(&ll, Stage::Fragment, &tmp, Default::default())
            .unwrap_err();
    assert!(error.contains("flags=0"), "{error}");
}

#[test]
fn fragment_sample_positions_reject_out_of_payload_constant_index() {
    let ll = AIR.replace("i32 %sample, i32 0)", "i32 8, i32 0)");
    let error = translate_sanitized_native_reflected(
        &ll,
        Stage::Fragment,
        &std::env::temp_dir(),
        Default::default(),
    )
    .unwrap_err();
    assert!(error.contains("eight-position payload"), "{error}");
}

#[test]
fn fragment_sample_positions_reject_dynamic_flags() {
    let ll = AIR.replace("i32 %sample, i32 0)", "i32 %sample, i32 %sample)");
    let error = translate_sanitized_native_reflected(
        &ll,
        Stage::Fragment,
        &std::env::temp_dir(),
        Default::default(),
    )
    .unwrap_err();
    assert!(error.contains("flags=0"), "{error}");
}

#[test]
fn fragment_sample_positions_reject_wide_flags_even_when_zero() {
    let ll = AIR
        .replace("i32 %sample, i32 0)", "i32 %sample, i64 0)")
        .replace("i32 0, i32 0)", "i32 0, i64 0)")
        .replace("v2f32(i32, i32)", "v2f32(i32, i64)");
    let error = translate_sanitized_native_reflected(
        &ll,
        Stage::Fragment,
        &std::env::temp_dir(),
        Default::default(),
    )
    .unwrap_err();
    assert!(error.contains("flags must be a 32-bit integer"), "{error}");
}

#[test]
fn fragment_sample_positions_reject_non_fragment_stage() {
    let error = translate_sanitized_native_reflected(
        AIR,
        Stage::Vertex,
        &std::env::temp_dir(),
        Default::default(),
    )
    .unwrap_err();
    assert!(error.contains("fragment"), "{error}");
}

fn assert_bounded_dynamic_index_dataflow(bytes: &[u8]) {
    use spirv::Op;
    struct Inst {
        opcode: u16,
        result_id: u32,
        operands: Vec<u32>,
    }
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    let mut instructions = Vec::new();
    let mut offset = 5;
    while offset < words.len() {
        let len = (words[offset] >> 16) as usize;
        let opcode = (words[offset] & 0xffff) as u16;
        assert!(len > 0 && offset + len <= words.len());
        if [
            Op::Constant,
            Op::ConstantComposite,
            Op::ULessThan,
            Op::Select,
            Op::AccessChain,
            Op::Load,
            Op::CompositeConstruct,
        ]
        .iter()
        .any(|op| *op as u16 == opcode)
        {
            assert!(len >= 3);
            instructions.push(Inst {
                opcode,
                result_id: words[offset + 2],
                operands: words[offset + 3..offset + len].to_vec(),
            });
        }
        offset += len;
    }
    let id = |o: &u32| *o;
    let constant = |word: u32| -> Option<u32> {
        instructions
            .iter()
            .find(|i| i.result_id == word && i.opcode == Op::Constant as u16)
            .and_then(|i| i.operands.first().copied())
    };
    let guard = instructions
        .iter()
        .find(|i| {
            i.opcode == Op::ULessThan as u16
                && constant(id(&i.operands[0])).is_none()
                && constant(id(&i.operands[1])) == Some(8)
        })
        .unwrap();
    let argument = id(&guard.operands[0]);
    let guard_id = guard.result_id;
    let safe = instructions
        .iter()
        .find(|i| {
            i.opcode == Op::Select as u16
                && i.operands.len() == 3
                && id(&i.operands[0]) == guard_id
                && id(&i.operands[1]) == argument
                && constant(id(&i.operands[2])) == Some(0)
        })
        .unwrap();
    let pointer = instructions
        .iter()
        .find(|i| {
            i.opcode == Op::AccessChain as u16
                && i.operands.last().is_some_and(|o| id(o) == safe.result_id)
        })
        .unwrap();
    let loaded = instructions
        .iter()
        .find(|i| i.opcode == Op::Load as u16 && id(&i.operands[0]) == pointer.result_id)
        .unwrap();
    let condition = instructions
        .iter()
        .find(|i| {
            i.opcode == Op::CompositeConstruct as u16 && i.operands == vec![guard_id, guard_id]
        })
        .unwrap();
    let selected = instructions
        .iter()
        .find(|i| {
            i.opcode == Op::Select as u16
                && id(&i.operands[0]) == condition.result_id
                && id(&i.operands[1]) == loaded.result_id
        })
        .unwrap();
    let fallback = instructions
        .iter()
        .find(|i| {
            i.result_id == id(&selected.operands[2]) && i.opcode == Op::ConstantComposite as u16
        })
        .unwrap();
    assert_eq!(fallback.operands.len(), 2);
    assert!(fallback.operands.iter().all(|o| constant(id(o)) == Some(0)));
    let bound = constant(id(&guard.operands[1])).unwrap();
    let false_index = constant(id(&safe.operands[2])).unwrap();
    let values: [[f32; 2]; 8] = std::array::from_fn(|i| [i as f32 + 0.125, i as f32 + 0.375]);
    for index in [0u32, 7, 8, u32::MAX] {
        let valid = index < bound;
        let address = if valid { index } else { false_index };
        assert!(address < 8);
        let result = if valid {
            values[address as usize]
        } else {
            [0.0, 0.0]
        };
        assert_eq!(
            result,
            if index < 8 {
                values[index as usize]
            } else {
                [0.0, 0.0]
            }
        );
    }
}
