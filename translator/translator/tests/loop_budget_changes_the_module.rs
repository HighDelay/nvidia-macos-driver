use metal2vulkan::passes::Stage;
use metal2vulkan::{instrument_spirv_loop_budget, translate_sanitized_native, DEFAULT_LOOP_BUDGET};
use std::path::PathBuf;

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "metal2vulkan_loop_budget_{label}_{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&path);
    path
}

const LOOPY: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define void @k(ptr addrspace(1) %out, i32 %n) {
entry:
  br label %loop

loop:
  %i = phi i32 [ 0, %entry ], [ %next, %loop ]
  %acc = phi float [ 0.000000e+00, %entry ], [ %sum, %loop ]
  %slot = getelementptr inbounds float, ptr addrspace(1) %out, i32 %i
  %v = load float, ptr addrspace(1) %slot, align 4
  %sum = fadd float %acc, %v
  %next = add i32 %i, 1
  %done = icmp eq i32 %next, %n
  br i1 %done, label %exit, label %loop

exit:
  store float %sum, ptr addrspace(1) %out, align 4
  ret void
}

!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3}
!3 = !{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_name", !"float*", !"air.arg_name", !"out"}
"#;

const LOOP_FREE: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

define void @k(ptr addrspace(1) %out) {
entry:
  %v = load float, ptr addrspace(1) %out, align 4
  %sum = fadd float %v, %v
  store float %sum, ptr addrspace(1) %out, align 4
  ret void
}

!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3}
!3 = !{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_name", !"float*", !"air.arg_name", !"out"}
"#;

#[test]
fn a_loopy_module_is_rewritten_by_the_bound_it_is_dispatched_under() {
    let spv =
        translate_sanitized_native(LOOPY, Stage::Kernel, &scratch("loopy")).expect("translate");
    let (bounded, report) =
        instrument_spirv_loop_budget(&spv, DEFAULT_LOOP_BUDGET).expect("instrument");
    assert!(
        report.had_loops(),
        "the fixture exists to carry a loop; the bound reported none"
    );
    assert_ne!(
        spv, bounded,
        "the bound reported a loop but left the module byte-identical, so a candidate would be \
         dispatched exactly as translated and the caveat in this file's header no longer applies"
    );
}

#[test]
fn a_loop_free_module_is_dispatched_exactly_as_translated() {
    let spv =
        translate_sanitized_native(LOOP_FREE, Stage::Kernel, &scratch("flat")).expect("translate");
    let (bounded, report) =
        instrument_spirv_loop_budget(&spv, DEFAULT_LOOP_BUDGET).expect("instrument");
    assert!(!report.had_loops(), "the fixture has no loop to bound");
    assert_eq!(
        spv, bounded,
        "a module with no loop must come back byte-identical, or every loop-free case would be \
         compared against bytes the translator did not emit"
    );
}

#[test]
fn two_budgets_three_orders_apart_instrument_the_same_instructions() {
    let spv =
        translate_sanitized_native(LOOPY, Stage::Kernel, &scratch("budgets")).expect("translate");
    let (small, small_report) = instrument_spirv_loop_budget(&spv, 1024).expect("instrument small");
    let (large, large_report) =
        instrument_spirv_loop_budget(&spv, 262_144).expect("instrument large");
    assert_eq!(
        small_report.loops_bounded_in_place + small_report.loops_bounded_via_early_return,
        large_report.loops_bounded_in_place + large_report.loops_bounded_via_early_return
    );
    assert_eq!(
        small.len(),
        large.len(),
        "the two budgets must differ only in a constant, not in how much they add"
    );
    assert_ne!(
        small, large,
        "the budget constant itself must be in the module"
    );
}
