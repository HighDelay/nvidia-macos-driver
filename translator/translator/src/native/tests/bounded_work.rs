use crate::native::cfg::graph::cfg_builds_during;
use crate::passes::Stage;

fn loop_exit_selection_chain(groups: usize) -> String {
    let mut out = String::from(
        r#"target triple = "air64_v28-apple-macosx26.5.0"

%Words = type { [1024 x i32] }

define void @k(ptr addrspace(1) %out) {
entry:
  br label %loop

loop:
  %i = phi i32 [ 0, %entry ], [ %inext, %latch ]
  br label %h0
"#,
    );
    for group in 0..groups {
        let word = group % 1024;
        out.push_str(&format!(
            "h{group}:
  %p{group} = getelementptr inbounds %Words, ptr addrspace(1) %out, i64 0, i32 0, i64 {word}
  %v{group} = load i32, ptr addrspace(1) %p{group}
  %c{group} = icmp sgt i32 %v{group}, 0
  br i1 %c{group}, label %t{group}, label %h{next}
t{group}:
  %tv{group} = add i32 %v{group}, 1
  %tc{group} = icmp sgt i32 %tv{group}, 3
  br i1 %tc{group}, label %exit, label %h{next}
",
            next = group + 1
        ));
    }
    out.push_str(&format!(
        r#"h{groups}:
  br label %latch

latch:
  %inext = add i32 %i, 1
  %done = icmp slt i32 %inext, 16
  br i1 %done, label %loop, label %exit

exit:
  ret void
}}

!air.kernel = !{{!0}}
!0 = !{{ptr @k, !1, !2}}
!1 = !{{}}
!2 = !{{!3}}
!3 = !{{i32 0, !"air.buffer", !"air.buffer_size", i32 4096, !"air.struct_type_info", !4, !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.arg_type_name", !"Words", !"air.arg_name", !"out"}}
!4 = !{{i32 0, i32 4096, i32 0, !"uint", !"v0"}}
"#
    ));
    out
}

const GROUPS: usize = 6;

const MAX_CFG_BUILDS: usize = 1400;

#[test]
fn a_shared_loop_exit_is_not_re_analyzed_once_per_split() {
    let scratch = std::env::temp_dir().join(format!(
        "m2v_bounded_work_{}_{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&scratch).expect("scratch directory");
    let source = loop_exit_selection_chain(GROUPS);
    let (translated, builds) =
        cfg_builds_during(|| crate::translate_sanitized_native(&source, Stage::Kernel, &scratch));
    let _ = std::fs::remove_dir_all(&scratch);

    translated.expect("the generated kernel translates");
    assert!(
        builds <= MAX_CFG_BUILDS,
        "translating a {GROUPS}-selection kernel built {builds} source CFGs (bound {MAX_CFG_BUILDS}); \
         some pass is deriving the whole function again after a graph edit instead of maintaining \
         what it already had, which is how the 20-second per-attempt ceiling gets broken"
    );
}
