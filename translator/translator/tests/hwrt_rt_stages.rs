use metal2vulkan::passes::{Stage, TransformOptions};
use metal2vulkan::reflect::{ResourceKind, ShaderReflection};
use metal2vulkan::{
    disassemble, reflect_sanitized, tools, translate_native_no_retry_constructed_with_options,
};
use std::path::PathBuf;

const V_RT: &str = include_str!("fixtures/hwrt/rt3-v_rt.ll");
const F_RT: &str = include_str!("fixtures/hwrt/rt3-f_rt.ll");
const RQ_ARG: &str = include_str!("fixtures/hwrt/rt3-rq_arg.ll");
const RQ: &str = include_str!("fixtures/hwrt/rt3-rq.ll");
const V_PLAIN: &str = include_str!("fixtures/hwrt/rt3-v_plain.ll");
const RQ_DEV: &str = include_str!("fixtures/hwrt/rt3b-rq_dev.ll");
const RQ_OFF: &str = include_str!("fixtures/hwrt/rt3b-rq_off.ll");
const RQ_AOS: &str = include_str!("fixtures/hwrt/rt3b-rq_aos.ll");
const RQ_SEL: &str = include_str!("fixtures/hwrt/rt3b-rq_sel.ll");
const RQ_UNQ: &str = include_str!("fixtures/hwrt/rt3b-rq_unq.ll");

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("m2v_hwrt_{name}_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn translate(ll: &str, stage: Stage) -> Result<Vec<u8>, String> {
    translate_native_no_retry_constructed_with_options(ll, stage, TransformOptions::default())
}

fn reflect(ll: &str, stage: Stage) -> ShaderReflection {
    reflect_sanitized(ll, stage, TransformOptions::default()).expect("reflect")
}

fn asm_of(spv: &[u8]) -> String {
    disassemble(spv)
        .expect("disassemble")
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn structure_variable(asm: &str) -> (String, Option<u32>, Option<u32>) {
    let ty = asm
        .lines()
        .find(|l| l.contains("= OpTypeAccelerationStructureKHR"))
        .and_then(|l| l.split_whitespace().next())
        .unwrap_or_else(|| panic!("no OpTypeAccelerationStructureKHR in\n{asm}"))
        .to_string();
    let ptr = asm
        .lines()
        .find(|l| l.ends_with(&format!("= OpTypePointer UniformConstant {ty}")))
        .and_then(|l| l.split_whitespace().next())
        .unwrap_or_else(|| panic!("no UniformConstant pointer to {ty} in\n{asm}"))
        .to_string();
    let var = asm
        .lines()
        .find(|l| l.ends_with(&format!("= OpVariable {ptr} UniformConstant")))
        .and_then(|l| l.split_whitespace().next())
        .unwrap_or_else(|| panic!("no UniformConstant variable of {ptr} in\n{asm}"))
        .to_string();
    let decoration = |name: &str| -> Option<u32> {
        asm.lines()
            .find(|l| l.starts_with(&format!("OpDecorate {var} {name} ")))
            .and_then(|l| l.split_whitespace().last())
            .and_then(|n| n.parse().ok())
    };
    let set = decoration("DescriptorSet");
    let binding = decoration("Binding");
    (var, set, binding)
}

fn uint_constant(asm: &str, value: u32) -> Option<String> {
    let uint_ty = asm
        .lines()
        .find(|l| l.ends_with("= OpTypeInt 32 0"))
        .and_then(|l| l.split_whitespace().next())
        .unwrap_or_else(|| panic!("no uint type in\n{asm}"))
        .to_string();
    asm.lines()
        .find(|l| l.ends_with(&format!("= OpConstant {uint_ty} {value}")))
        .and_then(|l| l.split_whitespace().next())
        .map(str::to_string)
}

fn has_word_chain(asm: &str, word: u32) -> bool {
    let (Some(c0), Some(cw)) = (uint_constant(asm, 0), uint_constant(asm, word)) else {
        return false;
    };
    asm.lines()
        .any(|l| l.contains("= OpAccessChain ") && l.ends_with(&format!(" {c0} {cw}")))
}

fn shadow_binding(r: &ShaderReflection, metal_index: u32) -> (Option<u32>, Option<(u32, u32)>) {
    let b = r
        .bindings
        .iter()
        .find(|b| {
            b.kind == ResourceKind::AccelerationStructureShadow && b.metal_index == metal_index
        })
        .unwrap_or_else(|| {
            panic!(
                "no AccelerationStructureShadow at buffer {metal_index} in {:?}",
                r.bindings
            )
        });
    (
        b.param_index,
        b.descriptor.as_ref().map(|d| (d.set, d.binding)),
    )
}

#[test]
fn v_rt_vertex_acceleration_structure_at_buffer_0_translates_and_binds_like_a_buffer() {
    let t = tmp("v_rt");
    let result = translate(V_RT, Stage::Vertex);
    assert!(result.is_ok(), "v_rt must translate: {:?}", result.err());
    let spv = result.unwrap();
    let asm = asm_of(&spv);
    let (var, set, binding) = structure_variable(&asm);
    assert_eq!(set, Some(0), "{var} DescriptorSet\n{asm}");
    assert_eq!(
        binding,
        Some(0),
        "{var} Binding: the structure is `as [[buffer(0)]]`\n{asm}"
    );
    assert!(asm.contains("OpRayQueryInitializeKHR"), "{asm}");
    let r = reflect(V_RT, Stage::Vertex);
    let (param, descriptor) = shadow_binding(&r, 0);
    assert_eq!(param, Some(1));
    assert_eq!(descriptor, Some((0, 0)));
    let rk = reflect(RQ, Stage::Kernel);
    assert_eq!(shadow_binding(&rk, 0).1, descriptor);
    tools::spirv_val_bytes(&spv, &t).expect("spirv-val v_rt");
}

#[test]
fn f_rt_fragment_acceleration_structure_at_buffer_0_translates_and_binds_like_a_buffer() {
    let t = tmp("f_rt");
    let result = translate(F_RT, Stage::Fragment);
    assert!(result.is_ok(), "f_rt must translate: {:?}", result.err());
    let spv = result.unwrap();
    let asm = asm_of(&spv);
    let (var, set, binding) = structure_variable(&asm);
    assert_eq!(set, Some(0), "{var} DescriptorSet\n{asm}");
    assert_eq!(
        binding,
        Some(0),
        "{var} Binding: the structure is `as [[buffer(0)]]`\n{asm}"
    );
    assert!(asm.contains("OpRayQueryInitializeKHR"), "{asm}");
    let r = reflect(F_RT, Stage::Fragment);
    let (param, descriptor) = shadow_binding(&r, 0);
    assert_eq!(param, Some(4));
    assert_eq!(descriptor, Some((0, 0)));
    tools::spirv_val_bytes(&spv, &t).expect("spirv-val f_rt");
}

#[test]
fn v_plain_vertex_without_a_structure_mints_no_structure() {
    let t = tmp("v_plain");
    let spv = translate(V_PLAIN, Stage::Vertex).expect("v_plain translates");
    let asm = asm_of(&spv);
    assert!(!asm.contains("OpTypeAccelerationStructureKHR"), "{asm}");
    let r = reflect(V_PLAIN, Stage::Vertex);
    assert!(
        !r.bindings
            .iter()
            .any(|b| b.kind == ResourceKind::AccelerationStructureShadow),
        "{:?}",
        r.bindings
    );
    tools::spirv_val_bytes(&spv, &t).expect("spirv-val v_plain");
}

fn assert_converts_the_address(name: &str, ll: &str) -> String {
    let t = tmp(name);
    let result = translate(ll, Stage::Kernel);
    assert!(result.is_ok(), "{name} must translate: {:?}", result.err());
    let spv = result.unwrap();
    let asm = asm_of(&spv);
    assert!(
        asm.contains("OpConvertUToAccelerationStructureKHR"),
        "{name}: {asm}"
    );
    assert!(asm.contains("OpRayQueryInitializeKHR"), "{name}: {asm}");
    tools::spirv_val_bytes(&spv, &t).unwrap_or_else(|e| panic!("spirv-val {name}: {e}"));
    asm
}

#[test]
fn rq_arg_structure_in_a_constant_argument_buffer_converts_the_loaded_address() {
    let asm = assert_converts_the_address("rq_arg", RQ_ARG);
    assert!(has_word_chain(&asm, 0), "{asm}");
    assert!(has_word_chain(&asm, 1), "{asm}");
}

#[test]
fn rq_dev_structure_in_a_device_argument_buffer_converts_the_loaded_address() {
    assert_converts_the_address("rq_dev", RQ_DEV);
}

#[test]
fn rq_off_structure_field_at_byte_16_reads_words_4_and_5() {
    let asm = assert_converts_the_address("rq_off", RQ_OFF);
    assert!(has_word_chain(&asm, 4), "{asm}");
    assert!(has_word_chain(&asm, 5), "{asm}");
    assert!(
        !has_word_chain(&asm, 2) && !has_word_chain(&asm, 3),
        "field `a` (words 2, 3) must not be read\n{asm}"
    );
}

#[test]
fn rq_sel_select_of_two_structures_is_refused_by_name() {
    let err = translate(RQ_SEL, Stage::Kernel).expect_err("rq_sel must be refused");
    assert!(err.contains("ray query"), "{err}");
    assert!(
        err.contains("cannot address") && err.contains("heap load: true"),
        "{err}"
    );
}

#[test]
fn rq_aos_dynamically_indexed_structure_field_is_refused_by_name() {
    let err = translate(RQ_AOS, Stage::Kernel).expect_err("rq_aos must be refused");
    assert!(err.contains("ray query"), "{err}");
    assert!(
        err.contains("cannot address") && err.contains("dynamically indexed field: true"),
        "{err}"
    );
}

#[test]
fn rq_unq_unqueried_structure_parameter_mints_no_structure() {
    let t = tmp("rq_unq");
    let spv = translate(RQ_UNQ, Stage::Kernel).expect("rq_unq translates");
    let asm = asm_of(&spv);
    assert!(!asm.contains("OpTypeAccelerationStructureKHR"), "{asm}");
    assert!(
        !asm.contains("OpConvertUToAccelerationStructureKHR"),
        "{asm}"
    );
    tools::spirv_val_bytes(&spv, &t).expect("spirv-val rq_unq");
}

#[test]
fn rq_direct_parameter_keeps_the_load_path() {
    let t = tmp("rq");
    let spv = translate(RQ, Stage::Kernel).expect("rq translates");
    let asm = asm_of(&spv);
    assert!(asm.contains("OpTypeAccelerationStructureKHR"), "{asm}");
    assert!(
        !asm.contains("OpConvertUToAccelerationStructureKHR"),
        "{asm}"
    );
    tools::spirv_val_bytes(&spv, &t).expect("spirv-val rq");
}
