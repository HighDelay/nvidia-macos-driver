use super::super::ir::{LlModule, LlType};
use super::helpers::bitcast_width;
use super::Emitter;

fn emitter() -> Emitter {
    let ir = LlModule::parse("define void @k() {\nentry:\n  ret void\n}\n")
        .expect("minimal module parses");
    Emitter::new(ir)
}

fn vec(elem: LlType, lanes: u32) -> LlType {
    LlType::Vector(Box::new(elem), lanes)
}

#[test]
fn raw_rule_pads_vec3_and_keeps_element_array_align() {
    let e = emitter();
    let raw = |ty: &LlType| e.raw_type_size_align(ty).expect("covered type");
    assert_eq!(raw(&LlType::Bool), (1, 1));
    assert_eq!(raw(&LlType::Int(8)), (1, 1));
    assert_eq!(raw(&LlType::Int(16)), (2, 2));
    assert_eq!(raw(&LlType::Half), (2, 2));
    assert_eq!(raw(&LlType::BFloat), (2, 2));
    assert_eq!(raw(&LlType::Int(32)), (4, 4));
    assert_eq!(raw(&LlType::Float), (4, 4));
    assert_eq!(raw(&LlType::Int(64)), (8, 8));
    assert_eq!(raw(&LlType::Ptr(1)), (8, 8));
    assert_eq!(raw(&vec(LlType::Float, 2)), (8, 8));
    assert_eq!(raw(&vec(LlType::Float, 3)), (16, 16));
    assert_eq!(raw(&vec(LlType::Float, 4)), (16, 16));
    assert_eq!(raw(&LlType::Array(Box::new(LlType::Float), 3)), (12, 4));
    assert_eq!(raw(&LlType::Array(Box::new(LlType::Int(8)), 11)), (11, 1));
    assert_eq!(
        raw(&LlType::Array(Box::new(vec(LlType::Float, 3)), 2)),
        (32, 16)
    );
    assert_eq!(
        raw(&LlType::Struct(vec![LlType::Int(8), LlType::Float])),
        (8, 4)
    );
    assert_eq!(
        raw(&LlType::Struct(vec![
            LlType::Int(8),
            LlType::Array(Box::new(LlType::Int(8)), 3),
            LlType::Int(32),
        ])),
        (8, 4)
    );
}

#[test]
fn raw_rule_errors_on_uncovered_types() {
    let e = emitter();
    assert!(e.raw_type_size_align(&LlType::Int(24)).is_err());
    assert!(e.raw_type_size_align(&LlType::Void).is_err());
}

#[test]
fn workgroup_struct_padding_ranges_exclude_members_and_allocation_escape() {
    let e = emitter();
    let padded = LlType::Struct(vec![LlType::Int(32), LlType::Int(8)]);

    assert!(e
        .struct_range_is_padding(&padded, 5, 3)
        .expect("tail-padding range"));
    assert!(!e
        .struct_range_is_padding(&padded, 4, 1)
        .expect("member range"));
    assert!(!e
        .struct_range_is_padding(&padded, 5, 4)
        .expect("range beyond allocation"));
}

#[test]
fn raw_rule_uses_source_vector_abi_alignment() {
    let ir = LlModule::parse(concat!(
        "target datalayout = \"e-v24:64:64\"\n",
        "define void @k() {\nentry:\n  ret void\n}\n",
    ))
    .expect("module with custom datalayout parses");
    let e = Emitter::new(ir);

    assert_eq!(
        e.raw_type_size_align(&vec(LlType::Int(8), 3))
            .expect("covered vector"),
        (8, 8)
    );
    assert_eq!(
        e.raw_type_size_align(&LlType::Struct(vec![
            vec(LlType::Int(8), 3),
            LlType::Int(8),
        ]))
        .expect("covered struct"),
        (16, 8)
    );
}

#[test]
fn bitcast_width_is_total_scalar_bits() {
    assert_eq!(bitcast_width(&LlType::Float), Some(32));
    assert_eq!(bitcast_width(&LlType::Int(32)), Some(32));
    assert_eq!(bitcast_width(&LlType::Half), Some(16));
    assert_eq!(bitcast_width(&LlType::BFloat), Some(16));
    assert_eq!(bitcast_width(&LlType::Int(16)), Some(16));
    assert_eq!(bitcast_width(&LlType::Int(8)), Some(8));
    assert_eq!(bitcast_width(&LlType::Int(1)), Some(1));
    assert_eq!(bitcast_width(&LlType::Int(64)), Some(64));
    assert_eq!(bitcast_width(&vec(LlType::Float, 4)), Some(128));
    assert_eq!(bitcast_width(&vec(LlType::Half, 2)), Some(32));
    assert_eq!(bitcast_width(&LlType::Bool), None);
    assert_eq!(bitcast_width(&LlType::Ptr(1)), None);
    assert_eq!(
        bitcast_width(&LlType::Array(Box::new(LlType::Float), 4)),
        None
    );
    assert_eq!(
        bitcast_width(&LlType::Struct(vec![LlType::Float, LlType::Float])),
        None
    );
}

#[test]
fn vector_total_bits_is_vectors_only() {
    let e = emitter();
    assert_eq!(e.vector_total_bits(&vec(LlType::Float, 4)), Some(128));
    assert_eq!(e.vector_total_bits(&vec(LlType::Half, 3)), Some(48));
    assert_eq!(e.vector_total_bits(&LlType::Float), None);
    assert_eq!(e.vector_total_bits(&vec(LlType::Float, 1)), None);
}
