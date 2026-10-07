use metal2vulkan::passes::{Stage, TransformOptions};
use metal2vulkan::reflect::ResourceKind;
use metal2vulkan::{disassemble, translate_sanitized_native, translate_sanitized_native_reflected};
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("m2v_gated_buffer_bindings_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

const GATED_BUFFER: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

@pred = internal addrspace(2) global i8 undef, align 1
@enabled.MTL_FC_INIT_0_b = internal addrspace(2) externally_initialized constant i8 undef, section "air.fc_initializer", align 1

CTOR
define void @k(ptr addrspace(1) %gated, ptr addrspace(1) %always, i32 %gid) {
entry:
  %slot = zext i32 %gid to i64
  %p = getelementptr inbounds float, ptr addrspace(1) %gated, i64 %slot
  store float 1.000000e+00, ptr addrspace(1) %p, align 4
  %q = getelementptr inbounds float, ptr addrspace(1) %always, i64 %slot
  store float 2.000000e+00, ptr addrspace(1) %q, align 4
  ret void
}

!air.kernel = !{!0}
!air.function_constants = !{!7}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !5, !6}
!3 = !{i32 0, !"air.function_constant", !4, !"air.buffer", !"air.location_index", i32 3, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"float", !"air.arg_name", !"gated"}
!4 = !{ptr addrspace(2) @pred, !"bool", !"kEnabled"}
!5 = !{i32 1, !"air.buffer", !"air.location_index", i32 1, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"float", !"air.arg_name", !"always"}
!6 = !{i32 2, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"gid"}
!7 = !{ptr addrspace(2) @enabled.MTL_FC_INIT_0_b, !"bool", !"enabled", i32 0, i1 false}
"#;

fn constructor(value: &str) -> String {
    format!(
        r#"define internal void @ctor() section "air.static_init" {{
  store i8 {value}, ptr addrspace(2) @pred, align 1
  ret void
}}
"#
    )
}

fn source(ctor: &str) -> String {
    let ll = GATED_BUFFER.replace("CTOR", ctor);
    assert_ne!(
        ll, GATED_BUFFER,
        "the constructor placeholder must be substituted"
    );
    ll
}

fn storage_buffer_stores(asm: &str) -> usize {
    let mut storage_buffer_types = Vec::new();
    for line in asm.lines() {
        let line = line.trim();
        let Some((result, rest)) = line.split_once(" = ") else {
            continue;
        };
        if rest.starts_with("OpTypePointer StorageBuffer ") {
            storage_buffer_types.push(result.to_string());
        }
    }
    let mut storage_buffer_pointers = Vec::new();
    for line in asm.lines() {
        let line = line.trim();
        let Some((result, rest)) = line.split_once(" = ") else {
            continue;
        };
        let mut words = rest.split_whitespace();
        let Some(_opcode) = words.next() else {
            continue;
        };
        let Some(result_type) = words.next() else {
            continue;
        };
        if storage_buffer_types.iter().any(|t| t == result_type) {
            storage_buffer_pointers.push(result.to_string());
        }
    }
    asm.lines()
        .filter(|line| {
            line.trim()
                .strip_prefix("OpStore ")
                .and_then(|rest| rest.split_whitespace().next())
                .is_some_and(|pointer| storage_buffer_pointers.iter().any(|p| p == pointer))
        })
        .count()
}

fn buffer_bindings(ll: &str) -> Vec<u32> {
    let (_, reflection) = translate_sanitized_native_reflected(
        ll,
        Stage::Kernel,
        &tmp(),
        TransformOptions::default(),
    )
    .expect("translate");
    reflection
        .bindings
        .iter()
        .filter(|binding| binding.kind == ResourceKind::Buffer)
        .filter_map(|binding| binding.descriptor.map(|d| d.binding))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn asm_for(ll: &str) -> String {
    let spv = translate_sanitized_native(ll, Stage::Kernel, &tmp()).expect("translate");
    disassemble(&spv).expect("disassemble")
}

#[test]
fn a_gate_the_module_drives_on_binds_the_buffer_and_keeps_its_store() {
    let ll = source(&constructor("1"));
    let asm = asm_for(&ll);
    assert_eq!(
        storage_buffer_stores(&asm),
        2,
        "a buffer this module's own initializers switch on must carry its store beside the \
         ungated one:\n{asm}"
    );
    assert_eq!(buffer_bindings(&ll), vec![1, 3]);
}

#[test]
fn a_gate_the_module_drives_off_leaves_the_buffer_absent() {
    let ll = source(&constructor("0"));
    let asm = asm_for(&ll);
    assert_eq!(
        storage_buffer_stores(&asm),
        1,
        "only the ungated store may survive a gate driven off:\n{asm}"
    );
    assert_eq!(buffer_bindings(&ll), vec![1]);
}

#[test]
fn a_gate_the_module_cannot_resolve_keeps_the_possibly_absent_placeholder() {
    let ll = source("");
    let asm = asm_for(&ll);
    assert_eq!(
        storage_buffer_stores(&asm),
        1,
        "an unresolved gate must keep the conservative placeholder:\n{asm}"
    );
    assert_eq!(buffer_bindings(&ll), vec![1]);
}
