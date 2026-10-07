target triple = "spirv-unknown-vulkan1.2"
%struct.rs = type { [8 x i8] }

define void @k(ptr addrspace(1) %out, i32 %x) {
entry:
  %a = alloca [4 x %struct.rs], align 8
  %e = getelementptr inbounds [4 x %struct.rs], ptr %a, i64 0, i64 0
  %xf = uitofp i32 %x to float
  %xh = fptrunc float %xf to half
  %v2a = insertelement <2 x float> undef, float %xf, i64 0
  %v2 = insertelement <2 x float> %v2a, float 2.0, i64 1
  %r = call float @h(ptr %e, half %xh, float %xf, <2 x float> %v2)
  %b0p = getelementptr inbounds [4 x %struct.rs], ptr %a, i64 0, i64 0, i32 0, i64 0
  %b0 = load i8, ptr %b0p, align 1
  %b1p = getelementptr inbounds [4 x %struct.rs], ptr %a, i64 0, i64 0, i32 0, i64 1
  %b1 = load i8, ptr %b1p, align 1
  %z0 = zext i8 %b0 to i32
  %z1 = zext i8 %b1 to i32
  %s = shl i32 %z1, 8
  %w = or i32 %z0, %s
  %ri = bitcast float %r to i32
  %t = xor i32 %w, %ri
  store i32 %t, ptr addrspace(1) %out, align 4
  ret void
}

define internal float @h(ptr noundef nonnull align 1 dereferenceable(8) %p, half %vh, float %vf, <2 x float> %v2) {
entry:
  %g = getelementptr inbounds %struct.rs, ptr %p, i64 0, i32 0, i64 0
  store i8 0, ptr %g, align 1
  %c = bitcast ptr %p to ptr
  store <2 x float> %v2, ptr %c, align 1
  ret float %vf
}

!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !4}
!3 = !{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"out"}
!4 = !{i32 1, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"x"}
