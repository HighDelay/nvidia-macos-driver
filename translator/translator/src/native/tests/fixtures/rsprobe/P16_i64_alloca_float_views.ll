target triple = "spirv-unknown-vulkan1.2"
%struct.rs = type { [8 x i8] }

define void @k(ptr addrspace(1) %out, i32 %x) {
entry:
  %a = alloca i64, align 8
  %z = zext i32 %x to i64
  %w = mul i64 %z, 4294967297
  store i64 %w, ptr %a, align 8
  %f0 = load float, ptr %a, align 8
  %lp = getelementptr inbounds [2 x float], ptr %a, i64 0, i64 1
  %f1 = load float, ptr %lp, align 4
  %s = fadd float %f0, %f1
  %i = bitcast float %s to i32
  store i32 %i, ptr addrspace(1) %out, align 4
  ret void
}

!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
!4 = !{i32 1, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"x"}
