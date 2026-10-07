target triple = "spirv-unknown-vulkan1.2"
%struct.rs = type { [8 x i8] }

define void @k(ptr addrspace(1) %out, i32 %x) {
entry:
  %a = alloca [4 x %struct.rs], align 8
  %b = alloca [4 x %struct.rs], align 8
  %t = trunc i32 %x to i8
  %g = getelementptr inbounds [4 x %struct.rs], ptr %a, i64 0, i64 1, i32 0, i64 3
  store i8 %t, ptr %g, align 1
  %ga = getelementptr inbounds [4 x %struct.rs], ptr %a, i64 0, i64 1, i32 0, i64 0
  %gb = getelementptr inbounds [4 x %struct.rs], ptr %b, i64 0, i64 1, i32 0, i64 0
  %pa = bitcast ptr %ga to ptr
  %pb = bitcast ptr %gb to ptr
  %w = load i64, ptr %pa, align 8
  store i64 %w, ptr %pb, align 8
  %h = getelementptr inbounds [4 x %struct.rs], ptr %b, i64 0, i64 1, i32 0, i64 3
  %r = load i8, ptr %h, align 1
  %z = zext i8 %r to i32
  store i32 %z, ptr addrspace(1) %out, align 4
  ret void
}

!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
!4 = !{i32 1, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"x"}
