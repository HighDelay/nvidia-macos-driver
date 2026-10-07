target triple = "spirv-unknown-vulkan1.2"
%struct.rs = type { [8 x i8] }

define void @k(ptr addrspace(1) %out, i32 %x) {
entry:
  %a = alloca [4 x %struct.rs], align 8
  %xi = and i32 %x, 3
  %xz = zext i32 %xi to i64
  %e = getelementptr inbounds [4 x %struct.rs], ptr %a, i64 0, i64 %xz
  call void @iv(ptr %e)
  %bp = getelementptr inbounds [4 x %struct.rs], ptr %a, i64 0, i64 1, i32 0, i64 3
  %b = load i8, ptr %bp, align 1
  %z = zext i8 %b to i32
  store i32 %z, ptr addrspace(1) %out, align 4
  ret void
}

define internal void @iv(ptr %p) {
entry:
  %t = alloca i64, align 8
  %q = bitcast ptr %p to ptr
  %w = load i64, ptr %q, align 1
  store i64 %w, ptr %t, align 8
  store float 1.0, ptr %t, align 8
  %v = load i64, ptr %t, align 8
  store i64 %v, ptr %q, align 1
  ret void
}

!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
!4 = !{i32 1, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"x"}
