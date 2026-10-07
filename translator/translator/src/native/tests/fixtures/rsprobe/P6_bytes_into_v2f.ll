target triple = "spirv-unknown-vulkan1.2"
%struct.rs = type { [8 x i8] }

define void @k(ptr addrspace(1) %out, i32 %x) {
entry:
  %a = alloca <2 x float>, align 8
  %c = bitcast ptr %a to ptr
  %t = trunc i32 %x to i8
  store i8 %t, ptr %c, align 8
  %b1 = getelementptr inbounds i8, ptr %c, i64 1
  store i8 %t, ptr %b1, align 1
  %b5 = getelementptr inbounds i8, ptr %c, i64 5
  store i8 %t, ptr %b5, align 1
  %v = load <2 x float>, ptr %a, align 8
  %f = extractelement <2 x float> %v, i64 1
  %i = bitcast float %f to i32
  store i32 %i, ptr addrspace(1) %out, align 4
  ret void
}

!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
!4 = !{i32 1, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"x"}
