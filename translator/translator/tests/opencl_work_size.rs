//! OpenCL kernels reach the driver as AIR through Apple's OpenCL. Geekbench 6's OpenCL run stopped at its first workload
//! because get_local_size had no lowering: the kernel never translated and clSetKernelArg failed with -49.
use metal2vulkan::passes::Stage;
use metal2vulkan::{disassemble, translate_sanitized_native};
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("m2v_opencl_work_size_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

const KERNEL: &str = r#"target triple = "air64_v27-apple-macosx15.7.0"

declare i32 @air.get_local_size.i32(i32)
declare i32 @air.get_global_size.i32(i32)
declare i32 @air.get_global_id.i32(i32)

define void @sizes(ptr addrspace(1) noalias %out) {
entry:
  %gid = call i32 @air.get_global_id.i32(i32 0)
  %ls = call i32 @air.get_local_size.i32(i32 0)
  %gs = call i32 @air.get_global_size.i32(i32 1)
  %sum = add i32 %ls, %gs
  %idx = zext i32 %gid to i64
  %p = getelementptr inbounds i32, ptr addrspace(1) %out, i64 %idx
  store i32 %sum, ptr addrspace(1) %p, align 4
  ret void
}

!air.kernel = !{!0}
!0 = !{ptr @sizes, !1, !2}
!1 = !{}
!2 = !{!3}
!3 = !{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
"#;

#[test]
fn opencl_local_and_global_size_translate() {
    let spv = translate_sanitized_native(KERNEL, Stage::Kernel, &tmp())
        .expect("get_local_size / get_global_size must translate");
    let asm = disassemble(&spv).expect("disassemble");
    assert!(asm.contains("BuiltIn WorkgroupSize"), "local size reads the pipeline's workgroup size:\n{asm}");
    assert!(asm.contains("BuiltIn NumWorkgroups"), "global size multiplies the group count:\n{asm}");
}
