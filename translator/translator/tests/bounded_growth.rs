use metal2vulkan::passes::Stage;
use metal2vulkan::translate_sanitized_native;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static HANDED_OUT: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
            HANDED_OUT.fetch_add(layout.size(), Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let grown = unsafe { System.realloc(pointer, layout, new_size) };
        if !grown.is_null() {
            let live = LIVE
                .fetch_add(new_size, Ordering::Relaxed)
                .saturating_add(new_size)
                .saturating_sub(layout.size());
            LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
            PEAK.fetch_max(live, Ordering::Relaxed);
            HANDED_OUT.fetch_add(new_size.saturating_sub(layout.size()), Ordering::Relaxed);
        }
        grown
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

struct Cost {
    peak: usize,
    handed_out: usize,
}

fn cost_of<T>(work: impl FnOnce() -> T) -> (T, Cost) {
    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);
    let handed_out_before = HANDED_OUT.load(Ordering::Relaxed);
    let value = work();
    let cost = Cost {
        peak: PEAK.load(Ordering::Relaxed).saturating_sub(before),
        handed_out: HANDED_OUT
            .load(Ordering::Relaxed)
            .saturating_sub(handed_out_before),
    };
    (value, cost)
}

fn irreducible_chain(groups: usize) -> String {
    let mut out = String::from(
        r#"target triple = "air64_v28-apple-macosx26.5.0"

%Words = type { [1024 x i32] }

define void @k(ptr addrspace(1) %out) {
entry:
  br label %g0
"#,
    );
    for group in 0..groups {
        let word = group % 1024;
        out.push_str(&format!(
            "g{group}:
  %p{group} = getelementptr inbounds %Words, ptr addrspace(1) %out, i64 0, i32 0, i64 {word}
  %v{group} = load i32, ptr addrspace(1) %p{group}
  %c{group} = icmp sgt i32 %v{group}, 0
  br i1 %c{group}, label %a{group}, label %b{group}
a{group}:
  %av{group} = add i32 %v{group}, 1
  %ac{group} = icmp sgt i32 %av{group}, 3
  br i1 %ac{group}, label %b{group}, label %g{next}
b{group}:
  %bv{group} = add i32 %v{group}, 2
  %bc{group} = icmp sgt i32 %bv{group}, 5
  br i1 %bc{group}, label %a{group}, label %g{next}
",
            next = group + 1
        ));
    }
    out.push_str(&format!(
        r#"g{groups}:
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

const SMALL: usize = 100;

const MAX_PEAK_GROWTH: f64 = 2.6;

const MAX_WORK_GROWTH: f64 = 2.6;

#[test]
fn translation_cost_grows_linearly_with_the_block_count() {
    let scratch = std::env::temp_dir().join(format!("m2v_bounded_growth_{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("scratch directory");

    let small = irreducible_chain(SMALL);
    let large = irreducible_chain(SMALL * 2);
    for source in [&small, &large] {
        translate_sanitized_native(source, Stage::Kernel, &scratch)
            .expect("the generated kernel translates");
    }

    let (_, small_cost) = cost_of(|| {
        translate_sanitized_native(&small, Stage::Kernel, &scratch)
            .expect("the smaller kernel translates")
    });
    let (_, large_cost) = cost_of(|| {
        translate_sanitized_native(&large, Stage::Kernel, &scratch)
            .expect("the doubled kernel translates")
    });
    let _ = std::fs::remove_dir_all(&scratch);

    assert!(
        small_cost.peak > 0 && large_cost.peak > 0,
        "the allocator measured nothing: {} and {} bytes",
        small_cost.peak,
        large_cost.peak
    );
    let peak_growth = large_cost.peak as f64 / small_cost.peak as f64;
    assert!(
        peak_growth <= MAX_PEAK_GROWTH,
        "doubling the function multiplied peak translation memory by {peak_growth:.2} \
         ({} bytes at {SMALL} groups, {} bytes at {}); some analysis is \
         quadratic in the block count, which is how the 500 MiB per-translation budget gets broken",
        small_cost.peak,
        large_cost.peak,
        SMALL * 2
    );

    let work_growth = large_cost.handed_out as f64 / small_cost.handed_out as f64;
    assert!(
        work_growth <= MAX_WORK_GROWTH,
        "doubling the function multiplied total translation allocation by {work_growth:.2} \
         ({} bytes at {SMALL} groups, {} bytes at {}); an analysis of the whole function is being \
         re-run once per construct, which is how the 20-second per-translation ceiling gets broken",
        small_cost.handed_out,
        large_cost.handed_out,
        SMALL * 2
    );
}
