target triple = "spirv-unknown-vulkan1.2"
%struct.rs = type { [8 x i8] }

define void @k(ptr addrspace(1) %out, i32 %x) {
entry:
  %a = alloca <2 x float>, align 8
  %c = bitcast ptr %a to ptr
  %t = trunc i32 %x to i8
  %b0 = getelementptr inbounds %struct.rs, ptr %c, i64 0, i32 0, i64 0
  store i8 %t, ptr %b0, align 8
  %b5 = getelementptr inbounds %struct.rs, ptr %c, i64 0, i32 0, i64 5
  store i8 %t, ptr %b5, align 1
  %l0p = getelementptr inbounds <2 x float>, ptr %a, i64 0, i64 0
  %l1p = getelementptr inbounds <2 x float>, ptr %a, i64 0, i64 1
  %l0 = load float, ptr %l0p, align 8
  %l1 = load float, ptr %l1p, align 4
  %s = fadd float %l0, %l1
  store float %s, ptr %l1p, align 4
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
