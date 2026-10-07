; ModuleID = 'mesh.metal'
source_filename = "mesh.metal"
target datalayout = "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v16:16:16-v24:32:32-v32:32:32-v48:64:64-v64:64:64-v96:128:128-v128:128:128-v192:256:256-v256:256:256-v512:512:512-v1024:1024:1024-n8:16:32"
target triple = "air64_v27-apple-macosx15.7.0"

%struct.Payload = type { [4 x i32] }
%struct._mesh_grid_properties_t = type opaque
%struct._mesh_t = type opaque

; Function Attrs: mustprogress nounwind willreturn
define void @o_quads(%struct.Payload addrspace(6)* nocapture noundef writeonly align 4 dereferenceable(16) %0, %struct._mesh_grid_properties_t addrspace(3)* %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #0 {
  %5 = icmp ult i32 %2, 2
  br i1 %5, label %6, label %11

6:                                                ; preds = %4
  %7 = zext i32 %2 to i64
  %8 = getelementptr inbounds %struct.Payload, %struct.Payload addrspace(6)* %0, i64 0, i32 0, i64 %7
  %9 = shl i32 %3, 1
  %10 = add nuw i32 %9, %2
  store i32 %10, i32 addrspace(6)* %8, align 4, !tbaa !47
  br label %11

11:                                               ; preds = %6, %4
  %12 = icmp eq i32 %2, 0
  br i1 %12, label %13, label %14

13:                                               ; preds = %11
  tail call void @air.set_threadgroups_per_grid_mesh_properties(%struct._mesh_grid_properties_t addrspace(3)* nocapture %1, <3 x i32> <i32 2, i32 1, i32 1>) #5
  br label %14

14:                                               ; preds = %13, %11
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define void @m_quad(%struct._mesh_t addrspace(7)* %0, %struct.Payload addrspace(6)* nocapture noundef readonly align 4 dereferenceable(16) %1, <4 x float> addrspace(2)* nocapture noundef readonly "air-buffer-no-alias" %2, i32 noundef %3, i32 noundef %4) local_unnamed_addr #1 {
  %6 = icmp ult i32 %3, 4
  br i1 %6, label %7, label %25

7:                                                ; preds = %5
  %8 = zext i32 %4 to i64
  %9 = getelementptr inbounds %struct.Payload, %struct.Payload addrspace(6)* %1, i64 0, i32 0, i64 %8
  %10 = load i32, i32 addrspace(6)* %9, align 4, !tbaa !47
  %11 = tail call fast float @air.convert.f.f32.u.i32(i32 %10) #6
  %12 = fmul fast float %11, 5.000000e-01
  %13 = and i32 %3, 1
  %14 = icmp eq i32 %13, 0
  %15 = select i1 %14, float -1.000000e+00, float -5.000000e-01
  %16 = fadd fast float %12, %15
  %17 = and i32 %3, 2
  %18 = icmp eq i32 %17, 0
  %19 = select fast i1 %18, float -1.000000e+00, float 1.000000e+00
  %20 = insertelement <4 x float> <float poison, float poison, float 0.000000e+00, float 1.000000e+00>, float %16, i64 0
  %21 = insertelement <4 x float> %20, float %19, i64 1
  %22 = zext i32 %10 to i64
  %23 = getelementptr inbounds <4 x float>, <4 x float> addrspace(2)* %2, i64 %22
  %24 = load <4 x float>, <4 x float> addrspace(2)* %23, align 16, !tbaa !51, !alias.scope !52
  tail call void @air.set_position_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %3, <4 x float> %21) #5
  tail call void @air.set_vertex_data_mesh.v4f32(%struct._mesh_t addrspace(7)* nocapture %0, i32 0, i32 %3, <4 x float> %24) #5
  br label %25

25:                                               ; preds = %7, %5
  %26 = icmp ult i32 %3, 2
  br i1 %26, label %27, label %35

27:                                               ; preds = %25
  %28 = mul nuw nsw i32 %3, 3
  %29 = add nuw nsw i32 %28, 2
  %30 = icmp eq i32 %3, 0
  %31 = select i1 %30, i8 1, i8 3
  %32 = add nuw nsw i32 %28, 1
  %33 = xor i1 %30, true
  %34 = zext i1 %33 to i8
  tail call void @air.set_index_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %28, i8 %34) #5
  tail call void @air.set_index_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %32, i8 %31) #5
  tail call void @air.set_index_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %29, i8 2) #5
  tail call void @air.set_primitive_data_mesh.v4f32(%struct._mesh_t addrspace(7)* nocapture %0, i32 0, i32 %3, <4 x float> <float 1.000000e+00, float 1.000000e+00, float 1.000000e+00, float 1.000000e+00>) #5
  br label %35

35:                                               ; preds = %27, %25
  %36 = icmp eq i32 %3, 0
  br i1 %36, label %37, label %38

37:                                               ; preds = %35
  tail call void @air.set_primitive_count_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 2) #5
  br label %38

38:                                               ; preds = %37, %35
  ret void
}

; Function Attrs: mustprogress nofree nosync nounwind readnone willreturn
declare float @air.convert.f.f32.u.i32(i32) local_unnamed_addr #2

; Function Attrs: mustprogress nounwind willreturn
define void @m_only(%struct._mesh_t addrspace(7)* %0, <4 x float> addrspace(2)* nocapture noundef readonly "air-buffer-no-alias" %1, i32 noundef %2, i32 noundef %3) local_unnamed_addr #1 {
  %5 = icmp ult i32 %2, 4
  br i1 %5, label %6, label %21

6:                                                ; preds = %4
  %7 = tail call fast float @air.convert.f.f32.u.i32(i32 %3) #6
  %8 = fmul fast float %7, 5.000000e-01
  %9 = and i32 %2, 1
  %10 = icmp eq i32 %9, 0
  %11 = select i1 %10, float -1.000000e+00, float -5.000000e-01
  %12 = fadd fast float %8, %11
  %13 = and i32 %2, 2
  %14 = icmp eq i32 %13, 0
  %15 = select fast i1 %14, float -1.000000e+00, float 1.000000e+00
  %16 = insertelement <4 x float> <float poison, float poison, float 0.000000e+00, float 1.000000e+00>, float %12, i64 0
  %17 = insertelement <4 x float> %16, float %15, i64 1
  %18 = zext i32 %3 to i64
  %19 = getelementptr inbounds <4 x float>, <4 x float> addrspace(2)* %1, i64 %18
  %20 = load <4 x float>, <4 x float> addrspace(2)* %19, align 16, !tbaa !51, !alias.scope !55
  tail call void @air.set_position_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %2, <4 x float> %17) #5
  tail call void @air.set_vertex_data_mesh.v4f32(%struct._mesh_t addrspace(7)* nocapture %0, i32 0, i32 %2, <4 x float> %20) #5
  br label %21

21:                                               ; preds = %6, %4
  %22 = icmp ult i32 %2, 2
  br i1 %22, label %23, label %31

23:                                               ; preds = %21
  %24 = mul nuw nsw i32 %2, 3
  %25 = add nuw nsw i32 %24, 2
  %26 = icmp eq i32 %2, 0
  %27 = select i1 %26, i8 1, i8 3
  %28 = add nuw nsw i32 %24, 1
  %29 = xor i1 %26, true
  %30 = zext i1 %29 to i8
  tail call void @air.set_index_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %24, i8 %30) #5
  tail call void @air.set_index_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %28, i8 %27) #5
  tail call void @air.set_index_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 %25, i8 2) #5
  tail call void @air.set_primitive_data_mesh.v4f32(%struct._mesh_t addrspace(7)* nocapture %0, i32 0, i32 %2, <4 x float> <float 1.000000e+00, float 1.000000e+00, float 1.000000e+00, float 1.000000e+00>) #5
  br label %31

31:                                               ; preds = %23, %21
  %32 = icmp eq i32 %2, 0
  br i1 %32, label %33, label %34

33:                                               ; preds = %31
  tail call void @air.set_primitive_count_mesh(%struct._mesh_t addrspace(7)* nocapture %0, i32 2) #5
  br label %34

34:                                               ; preds = %33, %31
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind readnone willreturn
define <4 x float> @f_mesh(<4 x float> %0, <4 x float> %1, <4 x float> %2) local_unnamed_addr #3 {
  %4 = fmul fast <4 x float> %2, %1
  ret <4 x float> %4
}

; Function Attrs: argmemonly mustprogress nounwind willreturn
declare void @air.set_threadgroups_per_grid_mesh_properties(%struct._mesh_grid_properties_t addrspace(3)* nocapture, <3 x i32>) local_unnamed_addr #4

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

attributes #0 = { mustprogress nounwind willreturn "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="96" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #1 = { mustprogress nounwind willreturn "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="128" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #2 = { mustprogress nofree nosync nounwind readnone willreturn }
attributes #3 = { mustprogress nofree norecurse nosync nounwind readnone willreturn "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="128" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #4 = { argmemonly mustprogress nounwind willreturn }
attributes #5 = { argmemonly nounwind willreturn }
attributes #6 = { nounwind readnone willreturn }

!llvm.module.flags = !{!0, !1, !2, !3, !4, !5, !6, !7, !8}
!air.object = !{!9}
!air.mesh = !{!17, !30}
!air.fragment = !{!33}
!air.compile_options = !{!40, !41, !42}
!llvm.ident = !{!43}
!air.version = !{!44}
!air.language_version = !{!45}
!air.source_file_name = !{!46}

!0 = !{i32 2, !"SDK Version", [2 x i32] [i32 27, i32 0]}
!1 = !{i32 1, !"wchar_size", i32 4}
!2 = !{i32 7, !"frame-pointer", i32 2}
!3 = !{i32 7, !"air.max_device_buffers", i32 31}
!4 = !{i32 7, !"air.max_constant_buffers", i32 31}
!5 = !{i32 7, !"air.max_threadgroup_buffers", i32 31}
!6 = !{i32 7, !"air.max_textures", i32 128}
!7 = !{i32 7, !"air.max_read_write_textures", i32 8}
!8 = !{i32 7, !"air.max_samplers", i32 16}
!9 = !{void (%struct.Payload addrspace(6)*, %struct._mesh_grid_properties_t addrspace(3)*, i32, i32)* @o_quads, !10, !11}
!10 = !{}
!11 = !{!12, !14, !15, !16}
!12 = !{i32 0, !"air.payload", !"air.struct_type_info", !13, !"air.arg_type_size", i32 16, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"Payload", !"air.arg_name", !"pl"}
!13 = !{i32 0, i32 4, i32 4, !"uint", !"quads"}
!14 = !{i32 1, !"air.mesh_grid_properties", !"air.arg_type_name", !"mesh_grid_properties", !"air.arg_name", !"mgp"}
!15 = !{i32 2, !"air.thread_index_in_threadgroup", !"air.arg_type_name", !"uint", !"air.arg_name", !"tid"}
!16 = !{i32 3, !"air.threadgroup_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"gid"}
!17 = !{void (%struct._mesh_t addrspace(7)*, %struct.Payload addrspace(6)*, <4 x float> addrspace(2)*, i32, i32)* @m_quad, !10, !18}
!18 = !{!19, !26, !27, !28, !29}
!19 = !{i32 0, !"air.mesh", !20, !"air.arg_type_name", !"mesh<V, P, 8, 4, triangle>", !"air.arg_name", !"out"}
!20 = !{!"air.mesh_type_info", !21, !24, i32 8, i32 4, !"air.triangle"}
!21 = !{!22, !23}
!22 = !{!"air.position", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
!23 = !{!"air.mesh_vertex_data", i32 0, !"generated(3colDv4_f)", !"air.arg_type_name", !"float4", !"air.arg_name", !"col"}
!24 = !{!25}
!25 = !{!"air.mesh_primitive_data", i32 0, !"generated(4tintDv4_f)", !"air.arg_type_name", !"float4", !"air.arg_name", !"tint"}
!26 = !{i32 1, !"air.payload", !"air.struct_type_info", !13, !"air.arg_type_size", i32 16, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"Payload", !"air.arg_name", !"pl"}
!27 = !{i32 2, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.arg_type_size", i32 16, !"air.arg_type_align_size", i32 16, !"air.arg_type_name", !"float4", !"air.arg_name", !"cols"}
!28 = !{i32 3, !"air.thread_index_in_threadgroup", !"air.arg_type_name", !"uint", !"air.arg_name", !"tid"}
!29 = !{i32 4, !"air.threadgroup_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"gid"}
!30 = !{void (%struct._mesh_t addrspace(7)*, <4 x float> addrspace(2)*, i32, i32)* @m_only, !10, !31}
!31 = !{!19, !32, !15, !16}
!32 = !{i32 1, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.arg_type_size", i32 16, !"air.arg_type_align_size", i32 16, !"air.arg_type_name", !"float4", !"air.arg_name", !"cols"}
!33 = !{<4 x float> (<4 x float>, <4 x float>, <4 x float>)* @f_mesh, !34, !36}
!34 = !{!35}
!35 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!36 = !{!37, !38, !39}
!37 = !{i32 0, !"air.position", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos", !"air.arg_unused"}
!38 = !{i32 1, !"air.fragment_input", !"generated(3colDv4_f)", !"air.center", !"air.perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"col"}
!39 = !{i32 2, !"air.fragment_input", !"generated(4tintDv4_f)", !"air.flat", !"air.arg_type_name", !"float4", !"air.arg_name", !"tint"}
!40 = !{!"air.compile.denorms_disable"}
!41 = !{!"air.compile.fast_math_enable"}
!42 = !{!"air.compile.framebuffer_fetch_enable"}
!43 = !{!"Apple metal version 32023.917 (metalfe-32023.917.2)"}
!44 = !{i32 2, i32 7, i32 0}
!45 = !{!"Metal", i32 3, i32 2, i32 0}
!46 = !{!"/private/tmp/build/-Users-nullmoth-Work-apple/41038f0c-7516-4e7e-98e6-3ae944306ce0/scratchpad/b42/mesh.metal"}
!47 = !{!48, !48, i64 0}
!48 = !{!"int", !49, i64 0}
!49 = !{!"omnipotent char", !50, i64 0}
!50 = !{!"Simple C++ TBAA"}
!51 = !{!49, !49, i64 0}
!52 = !{!53}
!53 = distinct !{!53, !54, !"air-alias-scope-arg(2)"}
!54 = distinct !{!54, !"air-alias-scopes(m_quad)"}
!55 = !{!56}
!56 = distinct !{!56, !57, !"air-alias-scope-arg(1)"}
!57 = distinct !{!57, !"air-alias-scopes(m_only)"}
