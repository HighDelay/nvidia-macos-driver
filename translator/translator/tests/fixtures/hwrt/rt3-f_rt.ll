; ModuleID = 'rt3.metal'
source_filename = "rt3.metal"
target datalayout = "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v16:16:16-v24:32:32-v32:32:32-v48:64:64-v64:64:64-v96:128:128-v128:128:128-v192:256:256-v256:256:256-v512:512:512-v1024:1024:1024-n8:16:32"
target triple = "air64_v29-apple-macosx27.0.0"

%struct._instance_acceleration_structure_t = type opaque
%struct.Out = type { i32, float, i32, i32 }
%struct._intersection_function_table_t = type opaque
%struct.Args = type { %"struct.metal::raytracing::_acceleration_structure" }
%"struct.metal::raytracing::_acceleration_structure" = type { %struct._instance_acceleration_structure_t addrspace(1)* }

; Function Attrs: mustprogress nounwind willreturn
define void @rq(%struct._instance_acceleration_structure_t addrspace(1)* %0, %struct.Out addrspace(1)* nocapture noundef writeonly "air-buffer-no-alias" %1) local_unnamed_addr #0 {
  %3 = tail call %struct._intersection_function_table_t addrspace(1)* @air.get_null_intersection_function_table() #7
  %4 = tail call { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float> <float 0.000000e+00, float 0.000000e+00, float -1.000000e+00>, <3 x float> <float 0.000000e+00, float 0.000000e+00, float 1.000000e+00>, float 0.000000e+00, float 1.000000e+02, %struct._instance_acceleration_structure_t addrspace(1)* readonly %0, i32 255, %struct._intersection_function_table_t addrspace(1)* readonly %3, i8* null, i64 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 3, i32 -1, i32 -1, i32 0, i1 false, i1 false) #8
  %5 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %4, 0
  %6 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %4, 1
  %7 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %4, 2
  %8 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %4, 5
  %9 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 0
  store i32 %5, i32 addrspace(1)* %9, align 4, !tbaa.struct !51, !alias.scope !58, !noalias !61
  %10 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 1
  store float %6, float addrspace(1)* %10, align 4, !tbaa.struct !63, !alias.scope !58, !noalias !61
  %11 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 2
  store i32 %7, i32 addrspace(1)* %11, align 4, !tbaa.struct !64, !alias.scope !58, !noalias !61
  %12 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 3
  store i32 %8, i32 addrspace(1)* %12, align 4, !tbaa.struct !65, !alias.scope !58, !noalias !61
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define void @rq_arg(%struct.Args addrspace(2)* nocapture noundef readonly align 8 dereferenceable(8) "air-buffer-no-alias" %0, %struct.Out addrspace(1)* nocapture noundef writeonly "air-buffer-no-alias" %1) local_unnamed_addr #0 {
  %3 = getelementptr inbounds %struct.Args, %struct.Args addrspace(2)* %0, i64 0, i32 0, i32 0
  %4 = load %struct._instance_acceleration_structure_t addrspace(1)*, %struct._instance_acceleration_structure_t addrspace(1)* addrspace(2)* %3, align 8, !alias.scope !66, !noalias !69
  %5 = tail call %struct._intersection_function_table_t addrspace(1)* @air.get_null_intersection_function_table() #7
  %6 = tail call { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float> <float 0.000000e+00, float 0.000000e+00, float -1.000000e+00>, <3 x float> <float 0.000000e+00, float 0.000000e+00, float 1.000000e+00>, float 0.000000e+00, float 1.000000e+02, %struct._instance_acceleration_structure_t addrspace(1)* readonly %4, i32 255, %struct._intersection_function_table_t addrspace(1)* readonly %5, i8* null, i64 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 3, i32 -1, i32 -1, i32 0, i1 false, i1 false) #8
  %7 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 0
  %8 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 1
  %9 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 2
  %10 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %6, 5
  %11 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 0
  store i32 %7, i32 addrspace(1)* %11, align 4, !tbaa.struct !51, !alias.scope !69, !noalias !66
  %12 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 1
  store float %8, float addrspace(1)* %12, align 4, !tbaa.struct !63, !alias.scope !69, !noalias !66
  %13 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 2
  store i32 %9, i32 addrspace(1)* %13, align 4, !tbaa.struct !64, !alias.scope !69, !noalias !66
  %14 = getelementptr inbounds %struct.Out, %struct.Out addrspace(1)* %1, i64 0, i32 3
  store i32 %10, i32 addrspace(1)* %14, align 4, !tbaa.struct !65, !alias.scope !69, !noalias !66
  ret void
}

; Function Attrs: mustprogress nounwind willreturn
define <{ <4 x float>, float, i32, i32 }> @v_rt(i32 noundef %0, %struct._instance_acceleration_structure_t addrspace(1)* %1) local_unnamed_addr #0 {
  %3 = shl i32 %0, 1
  %4 = and i32 %3, 2
  %5 = tail call fast float @air.convert.f.f32.u.i32(i32 %4) #9
  %6 = insertelement <2 x float> undef, float %5, i64 0
  %7 = and i32 %0, 2
  %8 = tail call fast float @air.convert.f.f32.u.i32(i32 %7) #9
  %9 = insertelement <2 x float> %6, float %8, i64 1
  %10 = fmul fast <2 x float> %9, <float 2.000000e+00, float 2.000000e+00>
  %11 = fadd fast <2 x float> %10, <float -1.000000e+00, float -1.000000e+00>
  %12 = shufflevector <2 x float> %11, <2 x float> poison, <4 x i32> <i32 0, i32 1, i32 undef, i32 undef>
  %13 = shufflevector <4 x float> %12, <4 x float> <float poison, float poison, float 0.000000e+00, float 1.000000e+00>, <4 x i32> <i32 0, i32 1, i32 6, i32 7>
  %14 = tail call %struct._intersection_function_table_t addrspace(1)* @air.get_null_intersection_function_table() #7
  %15 = tail call { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float> <float 0.000000e+00, float 0.000000e+00, float -1.000000e+00>, <3 x float> <float 0.000000e+00, float 0.000000e+00, float 1.000000e+00>, float 0.000000e+00, float 1.000000e+02, %struct._instance_acceleration_structure_t addrspace(1)* readonly %1, i32 255, %struct._intersection_function_table_t addrspace(1)* readonly %14, i8* null, i64 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 3, i32 -1, i32 -1, i32 0, i1 false, i1 false) #8
  %16 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %15, 0
  %17 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %15, 1
  %18 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %15, 2
  %19 = insertvalue <{ <4 x float>, float, i32, i32 }> undef, <4 x float> %13, 0
  %20 = insertvalue <{ <4 x float>, float, i32, i32 }> %19, float %17, 1
  %21 = insertvalue <{ <4 x float>, float, i32, i32 }> %20, i32 %16, 2
  %22 = insertvalue <{ <4 x float>, float, i32, i32 }> %21, i32 %18, 3
  ret <{ <4 x float>, float, i32, i32 }> %22
}

; Function Attrs: mustprogress nofree nosync nounwind readnone willreturn
declare float @air.convert.f.f32.u.i32(i32) local_unnamed_addr #1

; Function Attrs: mustprogress nofree nosync nounwind readnone willreturn
define <{ <4 x float>, float, i32, i32 }> @v_plain(i32 noundef %0) local_unnamed_addr #2 {
  %2 = shl i32 %0, 1
  %3 = and i32 %2, 2
  %4 = tail call fast float @air.convert.f.f32.u.i32(i32 %3) #9
  %5 = insertelement <2 x float> undef, float %4, i64 0
  %6 = and i32 %0, 2
  %7 = tail call fast float @air.convert.f.f32.u.i32(i32 %6) #9
  %8 = insertelement <2 x float> %5, float %7, i64 1
  %9 = fmul fast <2 x float> %8, <float 2.000000e+00, float 2.000000e+00>
  %10 = fadd fast <2 x float> %9, <float -1.000000e+00, float -1.000000e+00>
  %11 = shufflevector <2 x float> %10, <2 x float> poison, <4 x i32> <i32 0, i32 1, i32 undef, i32 undef>
  %12 = shufflevector <4 x float> %11, <4 x float> <float poison, float poison, float 0.000000e+00, float 1.000000e+00>, <4 x i32> <i32 0, i32 1, i32 6, i32 7>
  %13 = insertvalue <{ <4 x float>, float, i32, i32 }> undef, <4 x float> %12, 0
  %14 = insertvalue <{ <4 x float>, float, i32, i32 }> %13, float -7.000000e+00, 1
  %15 = insertvalue <{ <4 x float>, float, i32, i32 }> %14, i32 7, 2
  %16 = insertvalue <{ <4 x float>, float, i32, i32 }> %15, i32 7, 3
  ret <{ <4 x float>, float, i32, i32 }> %16
}

; Function Attrs: mustprogress nofree nosync nounwind readnone willreturn
define <4 x float> @f_pass(<4 x float> %0, float %1, i32 %2, i32 %3) local_unnamed_addr #3 {
  %5 = tail call fast float @air.convert.f.f32.u.i32(i32 %2) #9
  %6 = tail call fast float @air.convert.f.f32.u.i32(i32 %3) #9
  %7 = insertelement <4 x float> <float poison, float poison, float poison, float 1.000000e+00>, float %5, i64 0
  %8 = insertelement <4 x float> %7, float %1, i64 1
  %9 = insertelement <4 x float> %8, float %6, i64 2
  ret <4 x float> %9
}

; Function Attrs: mustprogress nounwind willreturn
define <4 x float> @f_rt(<4 x float> %0, float %1, i32 %2, i32 %3, %struct._instance_acceleration_structure_t addrspace(1)* %4) local_unnamed_addr #4 {
  %6 = tail call %struct._intersection_function_table_t addrspace(1)* @air.get_null_intersection_function_table() #7
  %7 = tail call { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float> <float 0.000000e+00, float 0.000000e+00, float -1.000000e+00>, <3 x float> <float 0.000000e+00, float 0.000000e+00, float 1.000000e+00>, float 0.000000e+00, float 1.000000e+02, %struct._instance_acceleration_structure_t addrspace(1)* readonly %4, i32 255, %struct._intersection_function_table_t addrspace(1)* readonly %6, i8* null, i64 0, i32 0, i32 0, i32 0, i32 0, i32 0, i32 3, i32 -1, i32 -1, i32 0, i1 false, i1 false) #8
  %8 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %7, 0
  %9 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %7, 1
  %10 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %7, 2
  %11 = extractvalue { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } %7, 5
  %12 = tail call fast float @air.convert.f.f32.u.i32(i32 %8) #9
  %13 = insertelement <4 x float> undef, float %12, i64 0
  %14 = insertelement <4 x float> %13, float %9, i64 1
  %15 = tail call fast float @air.convert.f.f32.u.i32(i32 %10) #9
  %16 = insertelement <4 x float> %14, float %15, i64 2
  %17 = tail call fast float @air.convert.f.f32.u.i32(i32 %11) #9
  %18 = insertelement <4 x float> %16, float %17, i64 3
  ret <4 x float> %18
}

; Function Attrs: inaccessiblememonly mustprogress nofree nounwind readonly willreturn
declare %struct._intersection_function_table_t addrspace(1)* @air.get_null_intersection_function_table() local_unnamed_addr #5

; Function Attrs: mustprogress nounwind willreturn
declare { i32, float, i32, i32, i8 addrspace(1)*, i32, i32, <2 x float>, i1 } @air.intersect.instancing.triangle_data(<3 x float>, <3 x float>, float, float, %struct._instance_acceleration_structure_t addrspace(1)* readonly, i32, %struct._intersection_function_table_t addrspace(1)* readonly, i8*, i64, i32, i32, i32, i32, i32, i32, i32, i32, i32, i1, i1) local_unnamed_addr #6

attributes #0 = { mustprogress nounwind willreturn "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="96" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #1 = { mustprogress nofree nosync nounwind readnone willreturn }
attributes #2 = { mustprogress nofree nosync nounwind readnone willreturn "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="0" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #3 = { mustprogress nofree nosync nounwind readnone willreturn "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="128" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #4 = { mustprogress nounwind willreturn "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="128" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #5 = { inaccessiblememonly mustprogress nofree nounwind readonly willreturn }
attributes #6 = { mustprogress nounwind willreturn }
attributes #7 = { inaccessiblememonly nounwind readonly willreturn }
attributes #8 = { nounwind willreturn }
attributes #9 = { nounwind readnone willreturn }

!llvm.module.flags = !{!0, !1, !2, !3, !4, !5, !6, !7, !8}
!air.kernel = !{!9, !15}
!air.vertex = !{!19, !28}
!air.fragment = !{!38}
!air.compile_options = !{!44, !45, !46}
!llvm.ident = !{!47}
!air.version = !{!48}
!air.language_version = !{!49}
!air.source_file_name = !{!50}

!0 = !{i32 2, !"SDK Version", [2 x i32] [i32 27, i32 0]}
!1 = !{i32 1, !"wchar_size", i32 4}
!2 = !{i32 7, !"frame-pointer", i32 2}
!3 = !{i32 7, !"air.max_device_buffers", i32 31}
!4 = !{i32 7, !"air.max_constant_buffers", i32 31}
!5 = !{i32 7, !"air.max_threadgroup_buffers", i32 31}
!6 = !{i32 7, !"air.max_textures", i32 128}
!7 = !{i32 7, !"air.max_read_write_textures", i32 8}
!8 = !{i32 7, !"air.max_samplers", i32 16}
!9 = !{void (%struct._instance_acceleration_structure_t addrspace(1)*, %struct.Out addrspace(1)*)* @rq, !10, !11}
!10 = !{}
!11 = !{!12, !13}
!12 = !{i32 0, !"air.instance_acceleration_structure", !"air.location_index", i32 0, i32 1, !"air.read", !"air.arg_type_name", !"acceleration_structure<instancing>", !"air.arg_name", !"as"}
!13 = !{i32 1, !"air.buffer", !"air.location_index", i32 1, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.struct_type_info", !14, !"air.arg_type_size", i32 16, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"Out", !"air.arg_name", !"out"}
!14 = !{i32 0, i32 4, i32 0, !"uint", !"hit", i32 4, i32 4, i32 0, !"float", !"t", i32 8, i32 4, i32 0, !"uint", !"prim", i32 12, i32 4, i32 0, !"uint", !"inst"}
!15 = !{void (%struct.Args addrspace(2)*, %struct.Out addrspace(1)*)* @rq_arg, !10, !16}
!16 = !{!17, !13}
!17 = !{i32 0, !"air.indirect_buffer", !"air.buffer_size", i32 8, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.struct_type_info", !18, !"air.arg_type_size", i32 8, !"air.arg_type_align_size", i32 8, !"air.arg_type_name", !"Args", !"air.arg_name", !"a"}
!18 = !{i32 0, i32 8, i32 0, !"acceleration_structure<instancing>", !"as", !"air.indirect_argument", !12}
!19 = !{<{ <4 x float>, float, i32, i32 }> (i32, %struct._instance_acceleration_structure_t addrspace(1)*)* @v_rt, !20, !25}
!20 = !{!21, !22, !23, !24}
!21 = !{!"air.position", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos"}
!22 = !{!"air.vertex_output", !"generated(1tf)", !"air.arg_type_name", !"float", !"air.arg_name", !"t"}
!23 = !{!"air.vertex_output", !"generated(3hitj)", !"air.arg_type_name", !"uint", !"air.arg_name", !"hit"}
!24 = !{!"air.vertex_output", !"generated(4primj)", !"air.arg_type_name", !"uint", !"air.arg_name", !"prim"}
!25 = !{!26, !27}
!26 = !{i32 0, !"air.vertex_id", !"air.arg_type_name", !"uint", !"air.arg_name", !"vid"}
!27 = !{i32 1, !"air.instance_acceleration_structure", !"air.location_index", i32 0, i32 1, !"air.read", !"air.arg_type_name", !"acceleration_structure<instancing>", !"air.arg_name", !"as"}
!28 = !{<{ <4 x float>, float, i32, i32 }> (i32)* @v_plain, !20, !29}
!29 = !{!26}
!30 = !{<4 x float> (<4 x float>, float, i32, i32)* @f_pass, !31, !33}
!31 = !{!32}
!32 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!33 = !{!34, !35, !36, !37}
!34 = !{i32 0, !"air.position", !"air.center", !"air.no_perspective", !"air.arg_type_name", !"float4", !"air.arg_name", !"pos", !"air.arg_unused"}
!35 = !{i32 1, !"air.fragment_input", !"generated(1tf)", !"air.center", !"air.perspective", !"air.arg_type_name", !"float", !"air.arg_name", !"t"}
!36 = !{i32 2, !"air.fragment_input", !"generated(3hitj)", !"air.flat", !"air.arg_type_name", !"uint", !"air.arg_name", !"hit"}
!37 = !{i32 3, !"air.fragment_input", !"generated(4primj)", !"air.flat", !"air.arg_type_name", !"uint", !"air.arg_name", !"prim"}
!38 = !{<4 x float> (<4 x float>, float, i32, i32, %struct._instance_acceleration_structure_t addrspace(1)*)* @f_rt, !31, !39}
!39 = !{!34, !40, !41, !42, !43}
!40 = !{i32 1, !"air.fragment_input", !"generated(1tf)", !"air.center", !"air.perspective", !"air.arg_type_name", !"float", !"air.arg_name", !"t", !"air.arg_unused"}
!41 = !{i32 2, !"air.fragment_input", !"generated(3hitj)", !"air.flat", !"air.arg_type_name", !"uint", !"air.arg_name", !"hit", !"air.arg_unused"}
!42 = !{i32 3, !"air.fragment_input", !"generated(4primj)", !"air.flat", !"air.arg_type_name", !"uint", !"air.arg_name", !"prim", !"air.arg_unused"}
!43 = !{i32 4, !"air.instance_acceleration_structure", !"air.location_index", i32 0, i32 1, !"air.read", !"air.arg_type_name", !"acceleration_structure<instancing>", !"air.arg_name", !"as"}
!44 = !{!"air.compile.denorms_disable"}
!45 = !{!"air.compile.fast_math_enable"}
!46 = !{!"air.compile.framebuffer_fetch_enable"}
!47 = !{!"Apple metal version 32023.917 (metalfe-32023.917.2)"}
!48 = !{i32 2, i32 9, i32 0}
!49 = !{!"Metal", i32 4, i32 1, i32 0}
!50 = !{!"/private/tmp/build/-Users-nullmoth-Work-apple/41038f0c-7516-4e7e-98e6-3ae944306ce0/scratchpad/hwrt/rt3air/rt3.metal"}
!51 = !{i64 0, i64 4, !52, i64 4, i64 4, !56, i64 8, i64 4, !52, i64 12, i64 4, !52}
!52 = !{!53, !53, i64 0}
!53 = !{!"int", !54, i64 0}
!54 = !{!"omnipotent char", !55, i64 0}
!55 = !{!"Simple C++ TBAA"}
!56 = !{!57, !57, i64 0}
!57 = !{!"float", !54, i64 0}
!58 = !{!59}
!59 = distinct !{!59, !60, !"air-alias-scope-arg(1)"}
!60 = distinct !{!60, !"air-alias-scopes(rq)"}
!61 = !{!62}
!62 = distinct !{!62, !60, !"air-alias-scope-arg(0)"}
!63 = !{i64 0, i64 4, !56, i64 4, i64 4, !52, i64 8, i64 4, !52}
!64 = !{i64 0, i64 4, !52, i64 4, i64 4, !52}
!65 = !{i64 0, i64 4, !52}
!66 = !{!67}
!67 = distinct !{!67, !68, !"air-alias-scope-arg(0)"}
!68 = distinct !{!68, !"air-alias-scopes(rq_arg)"}
!69 = !{!70}
!70 = distinct !{!70, !68, !"air-alias-scope-arg(1)"}
