; ModuleID = 'rt3b.metal'
source_filename = "rt3b.metal"
target datalayout = "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v16:16:16-v24:32:32-v32:32:32-v48:64:64-v64:64:64-v96:128:128-v128:128:128-v192:256:256-v256:256:256-v512:512:512-v1024:1024:1024-n8:16:32"
target triple = "air64_v27-apple-macosx15.7.0"

%struct.Args = type { %"struct.metal::raytracing::_acceleration_structure" }
%"struct.metal::raytracing::_acceleration_structure" = type { %struct._instance_acceleration_structure_t addrspace(1)* }
%struct._instance_acceleration_structure_t = type opaque
%struct.Out = type { i32, float, i32, i32 }
%struct._intersection_function_table_t = type opaque
%struct.Args2 = type { i32, float, %"struct.metal::raytracing::_acceleration_structure", %"struct.metal::raytracing::_acceleration_structure" }

; Function Attrs: mustprogress nounwind willreturn
define void @rq_dev(%struct.Args addrspace(1)* nocapture noundef readonly "air-buffer-no-alias" %0, %struct.Out addrspace(1)* nocapture noundef writeonly "air-buffer-no-alias" %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.Args, %struct.Args addrspace(1)* %0, i64 0, i32 0, i32 0
  %4 = load %struct._instance_acceleration_structure_t addrspace(1)*, %struct._instance_acceleration_structure_t addrspace(1)* addrspace(1)* %3, align 8, !alias.scope !41, !noalias !44
  %5 = tail call %struct._intersection_function_table_t addrspace(1)* @air.get_null_intersection_function_table() #4
  %6 = tail call { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float> <float 0.000000e+00, float 0.000000e+00, float -1.000000e+00>, <3 x float> <float 0.000000e+00, float 0.000000e+00, float 1.000000e+00>, float 0.000000e+00, float 1.000000e+02, %struct._instance_acceleration_structure_t addrspace(1)* readonly %4, i32 255, %struct._intersection_function_table_t addrspace(1)* readonly %5, i8* null, i64 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 3, i32 -1, i32 -1, i32 0, i1 false, i1 false) #5
  %7 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 0
  %8 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 1
  %9 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 2
  %10 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 5
  %11 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 0
  store i32 %7, i32 addrspace(1)* %11, align 4, !tbaa.struct !46, !alias.scope !44, !noalias !41
  %12 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 1
  store float %8, float addrspace(1)* %12, align 4, !tbaa.struct !53, !alias.scope !44, !noalias !41
  %13 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 2
  store i32 %9, i32 addrspace(1)* %13, align 4, !tbaa.struct !54, !alias.scope !44, !noalias !41
  %14 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 3
  store i32 %10, i32 addrspace(1)* %14, align 4, !tbaa.struct !55, !alias.scope !44, !noalias !41
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define void @rq_off(%struct.Args2 addrspace(2)* nocapture noundef readonly align 8 dereferenceable(24) "air-buffer-no-alias" %0, %struct.Out addrspace(1)* nocapture noundef writeonly "air-buffer-no-alias" %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.Args2, %struct.Args2 addrspace(2)* %0, i64 0, i32 3, i32 0
  %4 = load %struct._instance_acceleration_structure_t addrspace(1)*, %struct._instance_acceleration_structure_t addrspace(1)* addrspace(2)* %3, align 8, !alias.scope !56, !noalias !59
  %5 = tail call %struct._intersection_function_table_t addrspace(1)* @air.get_null_intersection_function_table() #4
  %6 = tail call { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float> <float 0.000000e+00, float 0.000000e+00, float -1.000000e+00>, <3 x float> <float 0.000000e+00, float 0.000000e+00, float 1.000000e+00>, float 0.000000e+00, float 1.000000e+02, %struct._instance_acceleration_structure_t addrspace(1)* readonly %4, i32 255, %struct._intersection_function_table_t addrspace(1)* readonly %5, i8* null, i64 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 3, i32 -1, i32 -1, i32 0, i1 false, i1 false) #5
  %7 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 0
  %8 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 1
  %9 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 2
  %10 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 5
  %11 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 0
  store i32 %7, i32 addrspace(1)* %11, align 4, !tbaa.struct !46, !alias.scope !59, !noalias !56
  %12 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 1
  store float %8, float addrspace(1)* %12, align 4, !tbaa.struct !53, !alias.scope !59, !noalias !56
  %13 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 2
  store i32 %9, i32 addrspace(1)* %13, align 4, !tbaa.struct !54, !alias.scope !59, !noalias !56
  %14 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 3
  store i32 %10, i32 addrspace(1)* %14, align 4, !tbaa.struct !55, !alias.scope !59, !noalias !56
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define void @rq_aos(%struct.Args addrspace(2)* nocapture noundef readonly "air-buffer-no-alias" %0, %struct.Out addrspace(1)* nocapture noundef writeonly "air-buffer-no-alias" %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = zext i32 %2 to i64
  %5 = getelementptr inbounds %struct.Args, %struct.Args addrspace(2)* %0, i64 %4, i32 0, i32 0
  %6 = load %struct._instance_acceleration_structure_t addrspace(1)*, %struct._instance_acceleration_structure_t addrspace(1)* addrspace(2)* %5, align 8, !alias.scope !61, !noalias !64
  %7 = tail call %struct._intersection_function_table_t addrspace(1)* @air.get_null_intersection_function_table() #4
  %8 = tail call { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float> <float 0.000000e+00, float 0.000000e+00, float -1.000000e+00>, <3 x float> <float 0.000000e+00, float 0.000000e+00, float 1.000000e+00>, float 0.000000e+00, float 1.000000e+02, %struct._instance_acceleration_structure_t addrspace(1)* readonly %6, i32 255, %struct._intersection_function_table_t addrspace(1)* readonly %7, i8* null, i64 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 3, i32 -1, i32 -1, i32 0, i1 false, i1 false) #5
  %9 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %8, 0
  %10 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %8, 1
  %11 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %8, 2
  %12 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %8, 5
  %13 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 0
  store i32 %9, i32 addrspace(1)* %13, align 4, !tbaa.struct !46, !alias.scope !64, !noalias !61
  %14 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 1
  store float %10, float addrspace(1)* %14, align 4, !tbaa.struct !53, !alias.scope !64, !noalias !61
  %15 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 2
  store i32 %11, i32 addrspace(1)* %15, align 4, !tbaa.struct !54, !alias.scope !64, !noalias !61
  %16 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 3
  store i32 %12, i32 addrspace(1)* %16, align 4, !tbaa.struct !55, !alias.scope !64, !noalias !61
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define void @rq_sel(%struct.Args2 addrspace(2)* nocapture noundef readonly align 8 dereferenceable(24) "air-buffer-no-alias" %0, %struct.Out addrspace(1)* nocapture noundef writeonly "air-buffer-no-alias" %1, i32 noundef %2) local_unnamed_addr #0 {
  %4 = and i32 %2, 1
  %5 = icmp eq i32 %4, 0
  %6 = getelementptr inbounds %struct.Args2, %struct.Args2 addrspace(2)* %0, i64 0, i32 2
  %7 = getelementptr inbounds %struct.Args2, %struct.Args2 addrspace(2)* %0, i64 0, i32 3
  %8 = select i1 %5, %"struct.metal::raytracing::_acceleration_structure" addrspace(2)* %7, %"struct.metal::raytracing::_acceleration_structure" addrspace(2)* %6
  %9 = getelementptr inbounds %"struct.metal::raytracing::_acceleration_structure", %"struct.metal::raytracing::_acceleration_structure" addrspace(2)* %8, i64 0, i32 0
  %10 = load %struct._instance_acceleration_structure_t addrspace(1)*, %struct._instance_acceleration_structure_t addrspace(1)* addrspace(2)* %9, align 8, !alias.scope !66, !noalias !69
  %11 = tail call %struct._intersection_function_table_t addrspace(1)* @air.get_null_intersection_function_table() #4
  %12 = tail call { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float> <float 0.000000e+00, float 0.000000e+00, float -1.000000e+00>, <3 x float> <float 0.000000e+00, float 0.000000e+00, float 1.000000e+00>, float 0.000000e+00, float 1.000000e+02, %struct._instance_acceleration_structure_t addrspace(1)* readonly %10, i32 255, %struct._intersection_function_table_t addrspace(1)* readonly %11, i8* null, i64 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 3, i32 -1, i32 -1, i32 0, i1 false, i1 false) #5
  %13 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %12, 0
  %14 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %12, 1
  %15 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %12, 2
  %16 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %12, 5
  %17 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 0
  store i32 %13, i32 addrspace(1)* %17, align 4, !tbaa.struct !46, !alias.scope !69, !noalias !66
  %18 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 1
  store float %14, float addrspace(1)* %18, align 4, !tbaa.struct !53, !alias.scope !69, !noalias !66
  %19 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 2
  store i32 %15, i32 addrspace(1)* %19, align 4, !tbaa.struct !54, !alias.scope !69, !noalias !66
  %20 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 3
  store i32 %16, i32 addrspace(1)* %20, align 4, !tbaa.struct !55, !alias.scope !69, !noalias !66
  ret void
}

; Function Attrs: argmemonly mustprogress nofree norecurse nosync nounwind willreturn writeonly
define void @rq_unq(%struct._instance_acceleration_structure_t addrspace(1)* nocapture readnone %0, %struct.Out addrspace(1)* nocapture noundef writeonly "air-buffer-no-alias" %1) local_unnamed_addr #1 {
  %3 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 0
  store i32 5, i32 addrspace(1)* %3, align 4, !tbaa !71, !alias.scope !73
  ret void
}

; Function Attrs: inaccessiblememonly mustprogress nofree nounwind readonly willreturn
declare %struct._intersection_function_table_t addrspace(1)* @air.get_null_intersection_function_table() local_unnamed_addr #2

; Function Attrs: mustprogress nounwind willreturn
declare { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float>, <3 x float>, float, float, %struct._instance_acceleration_structure_t addrspace(1)* readonly, i32, %struct._intersection_function_table_t addrspace(1)* readonly, i8*, i64, i32, i32, i32, i32, i32, i32, i32, i32, i32, i1, i1) local_unnamed_addr #3

attributes #0 = { mustprogress nounwind willreturn "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="96" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #1 = { argmemonly mustprogress nofree norecurse nosync nounwind willreturn writeonly "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="0" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #2 = { inaccessiblememonly mustprogress nofree nounwind readonly willreturn }
attributes #3 = { mustprogress nounwind willreturn }
attributes #4 = { inaccessiblememonly nounwind readonly willreturn }
attributes #5 = { nounwind willreturn }

!llvm.module.flags = !{!0, !1, !2, !3, !4, !5, !6, !7, !8}
!air.kernel = !{!29}
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
!9 = !{void (%struct.Args addrspace(1)*, %struct.Out addrspace(1)*)* @rq_dev, !10, !11}
!10 = !{}
!11 = !{!12, !15}
!12 = !{i32 0, !"air.indirect_buffer", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.struct_type_info", !13, !"air.arg_type_size", i32 8, !"air.arg_type_align_size", i32 8, !"air.arg_type_name", !"Args", !"air.arg_name", !"a"}
!13 = !{i32 0, i32 8, i32 0, !"acceleration_structure<instancing>", !"as", !"air.indirect_argument", !14}
!14 = !{i32 0, !"air.instance_acceleration_structure", !"air.location_index", i32 0, i32 1, !"air.read", !"air.arg_type_name", !"acceleration_structure<instancing>", !"air.arg_name", !"as"}
!15 = !{i32 1, !"air.buffer", !"air.location_index", i32 1, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.struct_type_info", !16, !"air.arg_type_size", i32 16, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"Out", !"air.arg_name", !"out"}
!16 = !{i32 0, i32 4, i32 0, !"uint", !"hit", i32 4, i32 4, i32 0, !"float", !"t", i32 8, i32 4, i32 0, !"uint", !"prim", i32 12, i32 4, i32 0, !"uint", !"inst"}
!17 = !{void (%struct.Args2 addrspace(2)*, %struct.Out addrspace(1)*)* @rq_off, !10, !18}
!18 = !{!19, !15}
!19 = !{i32 0, !"air.indirect_buffer", !"air.buffer_size", i32 24, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.struct_type_info", !20, !"air.arg_type_size", i32 24, !"air.arg_type_align_size", i32 8, !"air.arg_type_name", !"Args2", !"air.arg_name", !"a"}
!20 = !{i32 0, i32 4, i32 0, !"uint", !"pad", !"air.indirect_argument", !21, i32 4, i32 4, i32 0, !"float", !"pad2", !"air.indirect_argument", !22, i32 8, i32 8, i32 0, !"acceleration_structure<instancing>", !"a", !"air.indirect_argument", !23, i32 16, i32 8, i32 0, !"acceleration_structure<instancing>", !"b", !"air.indirect_argument", !24}
!21 = !{i32 0, !"air.indirect_constant", !"air.location_index", i32 0, i32 1, !"air.arg_type_name", !"uint", !"air.arg_name", !"pad"}
!22 = !{i32 1, !"air.indirect_constant", !"air.location_index", i32 1, i32 1, !"air.arg_type_name", !"float", !"air.arg_name", !"pad2"}
!23 = !{i32 2, !"air.instance_acceleration_structure", !"air.location_index", i32 2, i32 1, !"air.read", !"air.arg_type_name", !"acceleration_structure<instancing>", !"air.arg_name", !"a"}
!24 = !{i32 3, !"air.instance_acceleration_structure", !"air.location_index", i32 3, i32 1, !"air.read", !"air.arg_type_name", !"acceleration_structure<instancing>", !"air.arg_name", !"b"}
!25 = !{void (%struct.Args addrspace(2)*, %struct.Out addrspace(1)*, i32)* @rq_aos, !10, !26}
!26 = !{!27, !15, !28}
!27 = !{i32 0, !"air.indirect_buffer", !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.struct_type_info", !13, !"air.arg_type_size", i32 8, !"air.arg_type_align_size", i32 8, !"air.arg_type_name", !"Args", !"air.arg_name", !"a"}
!28 = !{i32 2, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"i"}
!29 = !{void (%struct.Args2 addrspace(2)*, %struct.Out addrspace(1)*, i32)* @rq_sel, !10, !30}
!30 = !{!19, !15, !28}
!31 = !{void (%struct._instance_acceleration_structure_t addrspace(1)*, %struct.Out addrspace(1)*)* @rq_unq, !10, !32}
!32 = !{!33, !15}
!33 = !{i32 0, !"air.instance_acceleration_structure", !"air.location_index", i32 0, i32 1, !"air.read", !"air.arg_type_name", !"acceleration_structure<instancing>", !"air.arg_name", !"as", !"air.arg_unused"}
!34 = !{!"air.compile.denorms_disable"}
!35 = !{!"air.compile.fast_math_enable"}
!36 = !{!"air.compile.framebuffer_fetch_enable"}
!37 = !{!"Apple metal version 32023.917 (metalfe-32023.917.2)"}
!38 = !{i32 2, i32 7, i32 0}
!39 = !{!"Metal", i32 3, i32 2, i32 0}
!40 = !{!"/private/tmp/build/-Users-nullmoth-Work-apple/41038f0c-7516-4e7e-98e6-3ae944306ce0/scratchpad/hwrt/rt3air/rt3b.metal"}
!41 = !{!42}
!42 = distinct !{!42, !43, !"air-alias-scope-arg(0)"}
!43 = distinct !{!43, !"air-alias-scopes(rq_dev)"}
!44 = !{!45}
!45 = distinct !{!45, !43, !"air-alias-scope-arg(1)"}
!46 = !{i64 0, i64 4, !47, i64 4, i64 4, !51, i64 8, i64 4, !47, i64 12, i64 4, !47}
!47 = !{!48, !48, i64 0}
!48 = !{!"int", !49, i64 0}
!49 = !{!"omnipotent char", !50, i64 0}
!50 = !{!"Simple C++ TBAA"}
!51 = !{!52, !52, i64 0}
!52 = !{!"float", !49, i64 0}
!53 = !{i64 0, i64 4, !51, i64 4, i64 4, !47, i64 8, i64 4, !47}
!54 = !{i64 0, i64 4, !47, i64 4, i64 4, !47}
!55 = !{i64 0, i64 4, !47}
!56 = !{!57}
!57 = distinct !{!57, !58, !"air-alias-scope-arg(0)"}
!58 = distinct !{!58, !"air-alias-scopes(rq_off)"}
!59 = !{!60}
!60 = distinct !{!60, !58, !"air-alias-scope-arg(1)"}
!61 = !{!62}
!62 = distinct !{!62, !63, !"air-alias-scope-arg(0)"}
!63 = distinct !{!63, !"air-alias-scopes(rq_aos)"}
!64 = !{!65}
!65 = distinct !{!65, !63, !"air-alias-scope-arg(1)"}
!66 = !{!67}
!67 = distinct !{!67, !68, !"air-alias-scope-arg(0)"}
!68 = distinct !{!68, !"air-alias-scopes(rq_sel)"}
!69 = !{!70}
!70 = distinct !{!70, !68, !"air-alias-scope-arg(1)"}
!71 = !{!72, !48, i64 0}
!72 = !{!"_ZTS3Out", !48, i64 0, !52, i64 4, !48, i64 8, !48, i64 12}
!73 = !{!74}
!74 = distinct !{!74, !75, !"air-alias-scope-arg(1)"}
!75 = distinct !{!75, !"air-alias-scopes(rq_unq)"}
