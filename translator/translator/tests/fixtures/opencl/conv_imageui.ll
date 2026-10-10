; ModuleID = 'nvmtl-air'
source_filename = "__entry_conv"
target datalayout = "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v16:16:16-v24:32:32-v32:32:32-v48:64:64-v64:64:64-v96:128:128-v128:128:128-v192:256:256-v256:256:256-v512:512:512-v1024:1024:1024-n8:16:32"
target triple = "air64-apple-macosx15.8.0"

@__air_sampler_state = internal addrspace(2) constant [2 x i64] [i64 34901797601050697, i64 0], align 8

; Function Attrs: norecurse nounwind
define void @__entry_conv(ptr addrspace(1) %0, ptr addrspace(1) %1, ptr addrspace(2) readonly captures(none) %2, ptr addrspace(2) readonly captures(none) %3) local_unnamed_addr #0 {
entry:
  %4 = load i32, ptr addrspace(2) %3, align 4
  tail call fastcc void @conv(ptr addrspace(1) %0, ptr addrspace(1) %1, ptr addrspace(2) %2, i32 %4)
  ret void
}

; Function Attrs: convergent norecurse nounwind
define internal fastcc void @conv(ptr addrspace(1) %in, ptr addrspace(1) %out, ptr addrspace(2) noundef readonly align 4 captures(none) %w, i32 noundef %taps) unnamed_addr #1 {
entry:
  %call = tail call fastcc i64 @_Z13get_global_idj(i32 noundef 0) #10
  %conv = trunc i64 %call to i32
  %vecinit = insertelement <2 x i32> undef, i32 %conv, i64 0
  %call1 = tail call fastcc i64 @_Z13get_global_idj(i32 noundef 1) #10
  %conv2 = trunc i64 %call1 to i32
  %vecinit3 = insertelement <2 x i32> %vecinit, i32 %conv2, i64 1
  %cmp20 = icmp sgt i32 %taps, 0
  br i1 %cmp20, label %for.body.lr.ph, label %for.cond.cleanup

for.body.lr.ph:                                   ; preds = %entry
  %div.neg2324 = lshr i32 %taps, 1
  br label %for.body

for.cond.cleanup:                                 ; preds = %for.body, %entry
  %sum.0.lcssa = phi <4 x float> [ zeroinitializer, %entry ], [ %1, %for.body ]
  %call10 = tail call fastcc <4 x i32> @_Z17convert_uint4_satDv4_f(<4 x float> noundef %sum.0.lcssa) #10
  tail call fastcc void @_Z13write_imageui14ocl_image2d_woDv2_iDv4_j(ptr addrspace(1) %out, <2 x i32> noundef %vecinit3, <4 x i32> noundef %call10) #11
  ret void

for.body:                                         ; preds = %for.body, %for.body.lr.ph
  %sum.022 = phi <4 x float> [ zeroinitializer, %for.body.lr.ph ], [ %1, %for.body ]
  %i.021 = phi i32 [ 0, %for.body.lr.ph ], [ %inc, %for.body ]
  %sub = sub nsw i32 %i.021, %div.neg2324
  %vecinit7 = insertelement <2 x i32> <i32 poison, i32 0>, i32 %sub, i64 0
  %add = add <2 x i32> %vecinit7, %vecinit3
  %call8 = tail call fastcc <4 x i32> @_Z12read_imageui14ocl_image2d_ro11ocl_samplerDv2_i(ptr addrspace(1) %in, ptr addrspace(2) @__air_sampler_state, <2 x i32> noundef %add) #12
  %call9 = tail call fastcc <4 x float> @_Z14convert_float4Dv4_j(<4 x i32> noundef %call8) #10
  %idxprom = zext i32 %i.021 to i64
  %arrayidx = getelementptr inbounds float, ptr addrspace(2) %w, i64 %idxprom
  %0 = load float, ptr addrspace(2) %arrayidx, align 4, !tbaa !22
  %splat.splatinsert = insertelement <4 x float> poison, float %0, i64 0
  %splat.splat = shufflevector <4 x float> %splat.splatinsert, <4 x float> poison, <4 x i32> zeroinitializer
  %1 = tail call <4 x float> @llvm.fmuladd.v4f32(<4 x float> %call9, <4 x float> %splat.splat, <4 x float> %sum.022)
  %inc = add nuw nsw i32 %i.021, 1
  %exitcond.not = icmp eq i32 %inc, %taps
  br i1 %exitcond.not, label %for.cond.cleanup, label %for.body
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare <4 x float> @llvm.fmuladd.v4f32(<4 x float>, <4 x float>, <4 x float>) #2

; Function Attrs: mustprogress nofree nosync nounwind optsize willreturn memory(none)
define internal fastcc <4 x float> @_Z14convert_float4Dv4_j(<4 x i32> noundef %0) unnamed_addr #3 {
  %2 = tail call <4 x float> @air.convert.f.v4f32.u.v4i32.rte(<4 x i32> %0) #13
  ret <4 x float> %2
}

; Function Attrs: mustprogress nofree nosync nounwind willreturn memory(none)
declare <4 x float> @air.convert.f.v4f32.u.v4i32.rte(<4 x i32>) local_unnamed_addr #4

; Function Attrs: convergent mustprogress nofree nounwind optsize willreturn memory(argmem: read)
define internal fastcc <4 x i32> @_Z12read_imageui14ocl_image2d_ro11ocl_samplerDv2_i(ptr addrspace(1) readonly captures(none) %0, ptr addrspace(2) readonly captures(none) %1, <2 x i32> noundef %2) unnamed_addr #5 {
  %4 = tail call <2 x float> @air.convert.f.v2f32.s.v2i32(<2 x i32> %2) #13
  %5 = tail call { <4 x i32>, i8 } @air.sample_texture_2d.u.v4i32(ptr addrspace(1) readonly captures(none) %0, ptr addrspace(2) readonly captures(none) %1, <2 x float> %4, i1 true, <2 x i32> zeroinitializer, i1 false, float 0.000000e+00, float 0.000000e+00, i32 0) #14
  %6 = extractvalue { <4 x i32>, i8 } %5, 0
  ret <4 x i32> %6
}

; Function Attrs: convergent mustprogress nofree nounwind willreturn memory(argmem: read)
declare { <4 x i32>, i8 } @air.sample_texture_2d.u.v4i32(ptr addrspace(1) readonly captures(none), ptr addrspace(2) readonly captures(none), <2 x float>, i1, <2 x i32>, i1, float, float, i32) local_unnamed_addr #6

; Function Attrs: mustprogress nofree nosync nounwind willreturn memory(none)
declare <2 x float> @air.convert.f.v2f32.s.v2i32(<2 x i32>) local_unnamed_addr #4

; Function Attrs: mustprogress nounwind optsize willreturn memory(argmem: readwrite)
define internal fastcc void @_Z13write_imageui14ocl_image2d_woDv2_iDv4_j(ptr addrspace(1) captures(none) %0, <2 x i32> noundef %1, <4 x i32> noundef %2) unnamed_addr #7 {
  tail call void @air.write_texture_2d.u.v4i32(ptr addrspace(1) captures(none) %0, <2 x i32> %1, <4 x i32> %2, i32 0, i32 2) #15
  ret void
}

; Function Attrs: mustprogress nounwind willreturn memory(argmem: readwrite)
declare void @air.write_texture_2d.u.v4i32(ptr addrspace(1) captures(none), <2 x i32>, <4 x i32>, i32, i32) local_unnamed_addr #8

; Function Attrs: mustprogress nofree nosync nounwind optsize willreturn memory(none)
define internal fastcc <4 x i32> @_Z17convert_uint4_satDv4_f(<4 x float> noundef %0) unnamed_addr #3 {
  %2 = tail call <4 x i32> @air.convert.u.v4i32.f.v4f32.rtz.sat(<4 x float> %0) #13
  ret <4 x i32> %2
}

; Function Attrs: mustprogress nofree nosync nounwind willreturn memory(none)
declare <4 x i32> @air.convert.u.v4i32.f.v4f32.rtz.sat(<4 x float>) local_unnamed_addr #4

; Function Attrs: mustprogress nofree nosync nounwind optsize willreturn memory(none)
define internal fastcc i64 @_Z13get_global_idj(i32 noundef %0) unnamed_addr #9 {
  %2 = tail call i32 @air.get_global_id.i32(i32 %0) #13
  %3 = zext i32 %2 to i64
  ret i64 %3
}

; Function Attrs: mustprogress nofree nosync nounwind willreturn memory(none)
declare i32 @air.get_global_id.i32(i32) local_unnamed_addr #4

attributes #0 = { norecurse nounwind }
attributes #1 = { convergent norecurse nounwind "frame-pointer"="all" "min-legal-vector-width"="128" "no-builtins" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "uniform-work-group-size"="true" }
attributes #2 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #3 = { mustprogress nofree nosync nounwind optsize willreturn memory(none) "frame-pointer"="all" "min-legal-vector-width"="128" "no-builtins" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #4 = { mustprogress nofree nosync nounwind willreturn memory(none) }
attributes #5 = { convergent mustprogress nofree nounwind optsize willreturn memory(argmem: read) "frame-pointer"="all" "min-legal-vector-width"="128" "no-builtins" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #6 = { convergent mustprogress nofree nounwind willreturn memory(argmem: read) }
attributes #7 = { mustprogress nounwind optsize willreturn memory(argmem: readwrite) "frame-pointer"="all" "min-legal-vector-width"="128" "no-builtins" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #8 = { mustprogress nounwind willreturn memory(argmem: readwrite) }
attributes #9 = { mustprogress nofree nosync nounwind optsize willreturn memory(none) "frame-pointer"="all" "min-legal-vector-width"="0" "no-builtins" "no-trapping-math"="true" "stack-protector-buffer-size"="8" }
attributes #10 = { nobuiltin nounwind willreturn memory(none) "no-builtins" }
attributes #11 = { nobuiltin nounwind "no-builtins" }
attributes #12 = { convergent nobuiltin nounwind willreturn memory(read) "no-builtins" }
attributes #13 = { nounwind willreturn memory(none) }
attributes #14 = { convergent nounwind willreturn memory(argmem: read) }
attributes #15 = { nounwind willreturn memory(argmem: readwrite) }

!llvm.module.flags = !{!0, !1, !2, !3, !4, !5, !6, !7}
!llvm.ident = !{!8}
!air.version = !{!9}
!air.language_version = !{!10}
!air.compile_options = !{!11, !12, !13}
!air.kernel = !{!14}
!air.sampler_states = !{!21}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 7, !"frame-pointer", i32 2}
!2 = !{i32 7, !"air.max_device_buffers", i32 -1}
!3 = !{i32 7, !"air.max_constant_buffers", i32 -1}
!4 = !{i32 7, !"air.max_threadgroup_buffers", i32 -1}
!5 = !{i32 7, !"air.max_textures", i32 -1}
!6 = !{i32 7, !"air.max_read_write_textures", i32 -1}
!7 = !{i32 7, !"air.max_samplers", i32 -1}
!8 = !{!"Apple metal version 32023.622 (metalfe-32023.622)"}
!9 = !{i32 2, i32 7, i32 0}
!10 = !{!"Metal", i32 1, i32 1, i32 0}
!11 = !{!"air.compile.denorms_disable"}
!12 = !{!"air.compile.fast_math_disable"}
!13 = !{!"air.compile.framebuffer_fetch_disable"}
!14 = !{ptr @__entry_conv, !15, !16}
!15 = !{}
!16 = !{!17, !18, !19, !20}
!17 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.sample", !"air.arg_type_name", !"texture2d<unknown, read>", !"air.arg_name", !"in"}
!18 = !{i32 1, !"air.texture", !"air.location_index", i32 1, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<unknown, write>", !"air.arg_name", !"out"}
!19 = !{i32 2, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"float", !"air.arg_name", !"w"}
!20 = !{i32 3, !"air.constant", !"air.location_index", i32 1, i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"int", !"air.arg_name", !"taps"}
!21 = !{!"air.sampler_state", ptr addrspace(2) @__air_sampler_state}
!22 = !{!23, !23, i64 0}
!23 = !{!"float", !24, i64 0}
!24 = !{!"omnipotent char", !25, i64 0}
!25 = !{!"Simple C/C++ TBAA"}
