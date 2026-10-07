#![allow(unused_imports)]
use super::*;
use crate::passes::Stage;
use crate::{disassemble, tools};

fn rsprobe_tmp(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!(
        "metal2vulkan_rsprobe_{name}_{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&tmp);
    tmp
}

fn rsprobe_translates(name: &str, ll: &str) {
    let tmp = rsprobe_tmp(name);
    let spv = crate::translate_sanitized_native(ll, Stage::Kernel, &tmp).unwrap_or_else(|e| {
        panic!("{name}: stage-55 translated this reduce_stitched shape, now refused: {e:?}")
    });
    disassemble(&spv).unwrap_or_else(|e| panic!("{name}: disassemble: {e:?}"));
    tools::spirv_val_bytes(&spv, &tmp).unwrap_or_else(|e| panic!("{name}: spirv-val: {e:?}"));
}

#[test]
fn rsprobe_p0_control_translates_and_validates() {
    rsprobe_translates("P0_control", include_str!("fixtures/rsprobe/P0_control.ll"));
}

#[test]
fn rsprobe_p12_store_half_hi_translates_and_validates() {
    rsprobe_translates(
        "P12_store_half_hi",
        include_str!("fixtures/rsprobe/P12_store_half_hi.ll"),
    );
}

#[test]
fn rsprobe_p13_mixed_view_alloca_translates_and_validates() {
    rsprobe_translates(
        "P13_mixed_view_alloca",
        include_str!("fixtures/rsprobe/P13_mixed_view_alloca.ll"),
    );
}

#[test]
fn rsprobe_p14_byte_image_alloca_translates_and_validates() {
    rsprobe_translates(
        "P14_byte_image_alloca",
        include_str!("fixtures/rsprobe/P14_byte_image_alloca.ll"),
    );
}

#[test]
fn rsprobe_p15_i64_copy_elem1_translates_and_validates() {
    rsprobe_translates(
        "P15_i64_copy_elem1",
        include_str!("fixtures/rsprobe/P15_i64_copy_elem1.ll"),
    );
}

#[test]
fn rsprobe_p16_i64_alloca_float_views_translates_and_validates() {
    rsprobe_translates(
        "P16_i64_alloca_float_views",
        include_str!("fixtures/rsprobe/P16_i64_alloca_float_views.ll"),
    );
}

#[test]
fn rsprobe_p17a_iv_param_i64_translates_and_validates() {
    rsprobe_translates(
        "P17a_iv_param_i64",
        include_str!("fixtures/rsprobe/P17a_iv_param_i64.ll"),
    );
}

#[test]
fn rsprobe_p17b_iv_param_i64_translates_and_validates() {
    rsprobe_translates(
        "P17b_iv_param_i64",
        include_str!("fixtures/rsprobe/P17b_iv_param_i64.ll"),
    );
}

#[test]
fn rsprobe_p17c_iv_param_i64_translates_and_validates() {
    rsprobe_translates(
        "P17c_iv_param_i64",
        include_str!("fixtures/rsprobe/P17c_iv_param_i64.ll"),
    );
}

#[test]
fn rsprobe_p18a_direct_elem_i64_translates_and_validates() {
    rsprobe_translates(
        "P18a_direct_elem_i64",
        include_str!("fixtures/rsprobe/P18a_direct_elem_i64.ll"),
    );
}

#[test]
fn rsprobe_p18b_direct_elem_i64_translates_and_validates() {
    rsprobe_translates(
        "P18b_direct_elem_i64",
        include_str!("fixtures/rsprobe/P18b_direct_elem_i64.ll"),
    );
}

#[test]
fn rsprobe_p19a_iv_bitcast_param_translates_and_validates() {
    rsprobe_translates(
        "P19a_iv_bitcast_param",
        include_str!("fixtures/rsprobe/P19a_iv_bitcast_param.ll"),
    );
}

#[test]
fn rsprobe_p19b_iv_bitcast_param_translates_and_validates() {
    rsprobe_translates(
        "P19b_iv_bitcast_param",
        include_str!("fixtures/rsprobe/P19b_iv_bitcast_param.ll"),
    );
}

#[test]
fn rsprobe_p1_store_half_translates_and_validates() {
    rsprobe_translates(
        "P1_store_half",
        include_str!("fixtures/rsprobe/P1_store_half.ll"),
    );
}

#[test]
fn rsprobe_p2_store_float_translates_and_validates() {
    rsprobe_translates(
        "P2_store_float",
        include_str!("fixtures/rsprobe/P2_store_float.ll"),
    );
}

#[test]
fn rsprobe_p3_store_v2f_translates_and_validates() {
    rsprobe_translates(
        "P3_store_v2f",
        include_str!("fixtures/rsprobe/P3_store_v2f.ll"),
    );
}

#[test]
fn rsprobe_p4_load_float_translates_and_validates() {
    rsprobe_translates(
        "P4_load_float",
        include_str!("fixtures/rsprobe/P4_load_float.ll"),
    );
}

#[test]
fn rsprobe_p5_load_v2f_translates_and_validates() {
    rsprobe_translates(
        "P5_load_v2f",
        include_str!("fixtures/rsprobe/P5_load_v2f.ll"),
    );
}

#[test]
fn rsprobe_p6_bytes_into_v2f_translates_and_validates() {
    rsprobe_translates(
        "P6_bytes_into_v2f",
        include_str!("fixtures/rsprobe/P6_bytes_into_v2f.ll"),
    );
}

#[test]
fn rsprobe_p7_i64_copy_translates_and_validates() {
    rsprobe_translates(
        "P7_i64_copy",
        include_str!("fixtures/rsprobe/P7_i64_copy.ll"),
    );
}

#[test]
fn rsprobe_p8_load_float_hi_translates_and_validates() {
    rsprobe_translates(
        "P8_load_float_hi",
        include_str!("fixtures/rsprobe/P8_load_float_hi.ll"),
    );
}

#[test]
fn rsprobe_p9_store_float_hi_translates_and_validates() {
    rsprobe_translates(
        "P9_store_float_hi",
        include_str!("fixtures/rsprobe/P9_store_float_hi.ll"),
    );
}

#[test]
fn rsprobe_straddling_byte_gep_views_refuse_loud() {
    for (name, ll) in [
        (
            "P10_load_float_bytegep",
            include_str!("fixtures/rsprobe/P10_load_float_bytegep.ll"),
        ),
        (
            "P11_store_float_bytegep",
            include_str!("fixtures/rsprobe/P11_store_float_bytegep.ll"),
        ),
    ] {
        let tmp = rsprobe_tmp(name);
        let r =
            std::panic::catch_unwind(|| crate::translate_sanitized_native(ll, Stage::Kernel, &tmp));
        match r {
            Ok(Err(_)) => {}
            Ok(Ok(_)) => panic!("{name}: now translates - validate it with spirv-val and move it to the translating arms"),
            Err(_) => panic!("{name}: the translator PANICKED instead of refusing"),
        }
    }
}
