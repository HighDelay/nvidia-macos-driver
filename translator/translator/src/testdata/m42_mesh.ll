; ModuleID = 'm42.metal'
source_filename = "m42.metal"
target datalayout = "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v16:16:16-v24:32:32-v32:32:32-v48:64:64-v64:64:64-v96:128:128-v128:128:128-v192:256:256-v256:256:256-v512:512:512-v1024:1024:1024-n8:16:32"
target triple = "air64_v27-apple-macosx15.7.0"

%struct._mesh_t = type opaque

; Function Attrs: convergent mustprogress nounwind willreturn
define void @m_cols(%struct._mesh_t addrspace(7)* %0, <4 x float> addrspace(2)* nocapture noundef readonly "air-buffer-no-alias" %1, i32 addrspace(2)* nocapture noundef readonly align 4 dereferenceable(4) "air-buffer-no-alias" %2, i32 noundef %3, i32 noundef %4) local_unnamed_addr #0 {
  %6 = zext i32 %4 to i64
  %7 = getelementptr inbounds <4 x float>, <4 x float> addrspace(2)* %1, i64 %6
  %8 = load <4 x float>, <4 x float> addrspace(2)* %7, align 16, !tbaa !41, !alias.scope !44, !noalias !47
  %9 = load i32, i32 addrspace(2)* %2, align 4, !tbaa !49, !alias.scope !47, !noalias !44
  tail call fastcc void @_ZL9emit_quadN5metal4meshI1V1PLj8ELj4ELNS_8topologyE2EEEjDv4_fjj(%struct._mesh_t addrspace(7)* %0, i32 noundef %4, <4 x float> noundef %8, i32 noundef %3, i32 noundef %9) #5
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define internal fastcc void @_ZL9emit_quadN5metal4meshI1V1PLj8ELj4ELNS_8topologyE2EEEjDv4_fjj(%struct._mesh_t addrspace(7)* %0, i32 noundef %1, <4 x float> noundef %2, i32 noundef %3, i32 noundef %4) unnamed_addr #1 {
  %6 = icmp ult i32 %3, 4
  br i1 %6, label %7, label %19

7:                                                ; preds = %5
  %8 = tail call fast float @air.convert.f.f32.u.i32(i32 %1) #6
  %9 = fmul fast float %8, 5.000000e-01
  %10 = and i32 %3, 1
  %11 = icmp eq i32 %10, 0
  %12 = select i1 %11, float -1.000000e+00, float -5.000000e-01
  %13 = fadd fast float %9, %12
  %14 = and i32 %3, 2
  %15 = icmp eq i32 %14, 0
  %16 = select fast i1 %15, float -1.000000e+00, float 1.000000e+00
  %17 = insertelement <4 x float> <float poison, float poison, float 0.000000e+00, float 1.000000e+00>, float %13, i64 0
  %18 = insertelement <4 x float> %17, float %16, i64 1
  tail call void @air.set_position_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %3, <4 x float> %18) #7
  tail call void @air.set_vertex_data_mesh.v4f32(%struct._mesh_t addrspace(7)* nocapture %0, i32 0, i32 %3, <4 x float> %2) #7
  br label %19

19:                                               ; preds = %7, %5
  %20 = icmp ult i32 %3, 2
  br i1 %20, label %21, label %33

21:                                               ; preds = %19
  %22 = mul nuw nsw i32 %3, 3
  %23 = icmp eq i32 %3, 0
  %24 = xor i1 %23, true
  %25 = zext i1 %24 to i8
  tail call void @air.set_index_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %22, i8 %25) #7
  %26 = add nuw nsw i32 %22, 1
  %27 = select i1 %23, i8 1, i8 3
  tail call void @air.set_index_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %26, i8 %27) #7
  %28 = add nuw nsw i32 %22, 2
  tail call void @air.set_index_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %28, i8 2) #7
  %29 = icmp eq i32 %4, 2
  %30 = icmp eq i32 %3, 1
  %31 = and i1 %30, %29
  %32 = select fast i1 %31, <4 x float> <float 0.000000e+00, float 0.000000e+00, float 0.000000e+00, float 1.000000e+00>, <4 x float> <float 1.000000e+00, float 1.000000e+00, float 1.000000e+00, float 1.000000e+00>
  tail call void @air.set_primitive_data_mesh.v4f32(%struct._mesh_t addrspace(7)* nocapture %0, i32 0, i32 %3, <4 x float> %32) #7
  br label %33

33:                                               ; preds = %21, %19
  %34 = icmp eq i32 %3, 0
  br i1 %34, label %35, label %41

35:                                               ; preds = %33
  %36 = icmp eq i32 %4, 1
  %37 = and i32 %1, 1
  %38 = icmp ne i32 %37, 0
  %39 = and i1 %38, %36
  %40 = select i1 %39, i32 1, i32 2
  tail call void @air.set_primitive_count_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %40) #7
  br label %41

41:                                               ; preds = %35, %33
  ret void
}

; Function Attrs: convergent mustprogress nounwind willreturn
define void @m_2d(%struct._mesh_t addrspace(7)* %0, <4 x float> addrspace(2)* nocapture noundef readonly "air-buffer-no-alias" %1, i32 noundef %2, <2 x i32> noundef %3) local_unnamed_addr #0 {
  %5 = extractelement <2 x i32> %3, i64 0
  %6 = extractelement <2 x i32> %3, i64 1
  %7 = shl i32 %6, 1
  %8 = add i32 %7, %5
  %9 = zext i32 %8 to i64
  %10 = getelementptr inbounds <4 x float>, <4 x float> addrspace(2)* %1, i64 %9
  %11 = load <4 x float>, <4 x float> addrspace(2)* %10, align 16, !tbaa !41, !alias.scope !51
  tail call fastcc void @_ZL9emit_quadN5metal4meshI1V1PLj8ELj4ELNS_8topologyE2EEEjDv4_fjj(%struct._mesh_t addrspace(7)* %0, i32 noundef %8, <4 x float> noundef %11, i32 noundef %2, i32 noundef 0) #5
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind readnone willreturn
define <4 x float> @f_mesh(<4 x float> %0, <4 x float> %1, <4 x float> %2) local_unnamed_addr #2 {
  %4 = fmul fast <4 x float> %2, %1
  ret <4 x float> %4
}

; Function Attrs: mustprogress nofree nosync nounwind readnone willreturn
declare float @air.convert.f.f32.u.i32(i32) local_unnamed_addr #3

; Function Attrs: argmemonly mustprogress nounwind willreturn
declare void @air.set_position_mesh(%struct._mesh_t addrspace(7)* nocapture, i32, <4 x float>) local_unnamed_addr #4

; Function Attrs: argmemonly mustprogress nounwind willreturn
declare void @air.set_vertex_data_mesh.v4f32(%struct._mesh_t addrspace(7)* nocapture, i32, i32, <4 x float>) local_unnamed_addr #4

; Function Attrs: argmemonly mustprogress nounwind willreturn
declare void @air.set_index_mesh(%struct._mesh_t addrspace(7)* nocapture, i32, i8) local_unnamed_addr #4

; Function Attrs: argmemonly mustprogress nounwind willreturn
declare void @air.set_primitive_data_mesh.v4f32(%struct._mesh_t addrspace(7)* nocapture, i32, i32, <4 x float>) local_unnamed_addr #4

; Function Attrs: argmemonly mustprogress nounwind willreturn
declare void @air.set_primitive_count_mesh(%struct._mesh_t addrspace(7)* nocapture, i32) local_unnamed_addr #4

attributes #0 = { convergent mustprogress nounwind willreturn "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="128" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #1 = { mustprogress nounwind willreturn "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="128" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #2 = { mustprogress nofree norecurse nosync nounwind readnone willreturn "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="128" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #3 = { mustprogress nofree nosync nounwind readnone willreturn }
attributes #4 = { argmemonly mustprogress nounwind willreturn }
attributes #5 = { nobuiltin "no-builtins" }
attributes #6 = { nounwind readnone willreturn }
attributes #7 = { argmemonly nounwind willreturn }

!llvm.module.flags = !{!0, !1, !2, !3, !4, !5, !6, !7, !8}
!air.mesh = !{!9, !23}
!air.fragment = !{!27}
!air.compile_options = !{!34, !35, !36}
!llvm.ident = !{!37}
!air.version = !{!38}
!air.language_version = !{!39}
!air.source_file_name = !{!40}

!0 = !{i32 2, !"SDK Version", [2 x i32] [i32 27, i32 0]}
!1 = !{i32 1, !"wchar_size", i32 4}
!2 = !{i32 7, !"frame-pointer", i32 2}
!3 = !{i32 7, !"air.max_device_buffers", i32 31}
!4 = !{i32 7, !"air.max_constant_buffers", i32 31}
!5 = !{i32 7, !"air.max_threadgroup_buffers", i32 31}
!6 = !{i32 7, !"air.max_textures", i32 128}
!7 = !{i32 7, !"air.max_read_write_textures", i32 8}
!8 = !{i32 7, !"air.max_samplers", i32 16}
!9 = !{void (%struct._mesh_t addrspace(7)*, <4 x float> addrspace(2)*, i32 addrspace(2)*, i32, i32)* @m_cols, !10, !11}
!10 = !{}
!11 = !{!12, !19, !20, !21, !22}
!12 = !{i32 0, !"air.mesh", !13, !"air.arg_type_name", !"mesh<V, P, 8, 4, triangle>", !"air.arg_name", !"out"}
!13 = !{!"air.mesh_type_info", !14, !17, i32 8, i32 4, !"air.triangle"}
!14 = !{!15, !16}
!15 = !{!"air.position", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
!16 = !{!"air.mesh_vertex_data", i32 0, !"generated(3colDv4_f)", !"air.arg_type_name", !"float4", !"air.arg_name", !"col"}
!17 = !{!18}
!18 = !{!"air.mesh_primitive_data", i32 0, !"generated(4tintDv4_f)", !"air.arg_type_name", !"float4", !"air.arg_name", !"tint"}
!19 = !{i32 1, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.arg_type_size", i32 16, !"air.arg_type_align_size", i32 16, !"air.arg_type_name", !"float4", !"air.arg_name", !"cols"}
!20 = !{i32 2, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 1, i32 1, !"air.read", !"air.address_space", i32 2, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"uint", !"air.arg_name", !"mode"}
!21 = !{i32 3, !"air.thread_index_in_threadgroup", !"air.arg_type_name", !"uint", !"air.arg_name", !"tid"}
!22 = !{i32 4, !"air.threadgroup_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"gid"}
!23 = !{void (%struct._mesh_t addrspace(7)*, <4 x float> addrspace(2)*, i32, <2 x i32>)* @m_2d, !10, !24}
!24 = !{!12, !19, !25, !26}
!25 = !{i32 2, !"air.thread_index_in_threadgroup", !"air.arg_type_name", !"uint", !"air.arg_name", !"tid"}
!26 = !{i32 3, !"air.threadgroup_position_in_grid", !"air.arg_type_name", !"uint2", !"air.arg_name", !"g"}
!27 = !{<4 x float> (<4 x float>, <4 x float>, <4 x float>)* @f_mesh, !28, !30}
!28 = !{!29}
!29 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!30 = !{!31, !32, !33}
!31 = !{i32 0, !"air.position", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos", !"air.arg_unused"}
!32 = !{i32 1, !"air.fragment_input", !"generated(3colDv4_f)", !"air.center", !"air.perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"col"}
!33 = !{i32 2, !"air.fragment_input", !"generated(4tintDv4_f)", !"air.flat", !"air.arg_type_name", !"float4", !"air.arg_name", !"tint"}
!34 = !{!"air.compile.denorms_disable"}
!35 = !{!"air.compile.fast_math_enable"}
!36 = !{!"air.compile.framebuffer_fetch_enable"}
!37 = !{!"Apple metal version 32023.917 (metalfe-32023.917.2)"}
!38 = !{i32 2, i32 7, i32 0}
!39 = !{!"Metal", i32 3, i32 2, i32 0}
!40 = !{!"/private/tmp/build/-Users-nullmoth-Work-apple/41038f0c-7516-4e7e-98e6-3ae944306ce0/scratchpad/b42/m42.metal"}
!41 = !{!42, !42, i64 0}
!42 = !{!"omnipotent char", !43, i64 0}
!43 = !{!"Simple C++ TBAA"}
!44 = !{!45}
!45 = distinct !{!45, !46, !"air-alias-scope-arg(1)"}
!46 = distinct !{!46, !"air-alias-scopes(m_cols)"}
!47 = !{!48}
!48 = distinct !{!48, !46, !"air-alias-scope-arg(2)"}
!49 = !{!50, !50, i64 0}
!50 = !{!"int", !42, i64 0}
!51 = !{!52}
!52 = distinct !{!52, !53, !"air-alias-scope-arg(1)"}
!53 = distinct !{!53, !"air-alias-scopes(m_2d)"}
