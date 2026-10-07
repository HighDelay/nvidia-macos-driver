; ModuleID = 'sp.metal'
source_filename = "sp.metal"
target datalayout = "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v16:16:16-v24:32:32-v32:32:32-v48:64:64-v64:64:64-v96:128:128-v128:128:128-v192:256:256-v256:256:256-v512:512:512-v1024:1024:1024-n8:16:32"
target triple = "air64_v27-apple-macosx15.7.0"

%struct.Pt = type { i32, i32, float, float, float }

; Function Attrs: argmemonly mustprogress nofree nosync nounwind
define void @sp1(%struct.Pt addrspace(1)* nocapture noundef readonly "air-buffer-no-alias" %0, %struct.Pt addrspace(1)* nocapture noundef writeonly "air-buffer-no-alias" %1, float addrspace(1)* nocapture noundef readonly "air-buffer-no-alias" %2, i32 addrspace(2)* nocapture noundef readonly align 4 dereferenceable(4) "air-buffer-no-alias" %3, i32 noundef %4) local_unnamed_addr #0 {
  %6 = alloca { i32, float, float, float }, align 8
  %7 = load i32, i32 addrspace(2)* %3, align 4, !tbaa !25, !alias.scope !29, !noalias !32
  %8 = icmp sgt i32 %7, %4
  br i1 %8, label %9, label %34

9:                                                ; preds = %5
  %10 = bitcast { i32, float, float, float }* %6 to i8*
  call void @llvm.lifetime.start.p0i8(i64 16, i8* nonnull %10)
  %11 = zext i32 %4 to i64
  %12 = getelementptr inbounds %struct.Pt, %struct.Pt addrspace(1)* %0, i64 %11, i32 0
  %13 = load i32, i32 addrspace(1)* %12, align 4, !tbaa.struct !36, !alias.scope !39, !noalias !40
  %14 = getelementptr inbounds %struct.Pt, %struct.Pt addrspace(1)* %0, i64 %11, i32 1
  %15 = bitcast i32 addrspace(1)* %14 to i8 addrspace(1)*
  call void @llvm.memcpy.p0i8.p1i8.i64(i8* noundef nonnull align 8 dereferenceable(16) %10, i8 addrspace(1)* noundef align 4 dereferenceable(16) %15, i64 16, i1 false), !tbaa.struct !41
  %16 = icmp sgt i32 %13, 0
  br i1 %16, label %25, label %17

17:                                               ; preds = %25, %9
  %18 = phi float [ 0.000000e+00, %9 ], [ %31, %25 ]
  %19 = getelementptr inbounds %struct.Pt, %struct.Pt addrspace(1)* %1, i64 %11, i32 0
  store i32 %13, i32 addrspace(1)* %19, align 4, !tbaa.struct !36, !alias.scope !42, !noalias !43
  %20 = getelementptr inbounds %struct.Pt, %struct.Pt addrspace(1)* %1, i64 %11, i32 1
  %21 = bitcast i32 addrspace(1)* %20 to i8 addrspace(1)*
  call void @llvm.memcpy.p1i8.p0i8.i64(i8 addrspace(1)* noundef align 4 dereferenceable(16) %21, i8* noundef nonnull align 8 dereferenceable(16) %10, i64 16, i1 false), !tbaa.struct !41
  %22 = getelementptr inbounds %struct.Pt, %struct.Pt addrspace(1)* %1, i64 %11, i32 3
  store float %18, float addrspace(1)* %22, align 4, !tbaa !44, !alias.scope !42, !noalias !43
  %23 = fmul fast float %18, 5.000000e-01
  %24 = getelementptr inbounds %struct.Pt, %struct.Pt addrspace(1)* %1, i64 %11, i32 4
  store float %23, float addrspace(1)* %24, align 4, !tbaa !46, !alias.scope !42, !noalias !43
  call void @llvm.lifetime.end.p0i8(i64 16, i8* nonnull %10)
  br label %34

25:                                               ; preds = %9, %25
  %26 = phi i32 [ %32, %25 ], [ 0, %9 ]
  %27 = phi float [ %31, %25 ], [ 0.000000e+00, %9 ]
  %28 = zext i32 %26 to i64
  %29 = getelementptr inbounds float, float addrspace(1)* %2, i64 %28
  %30 = load float, float addrspace(1)* %29, align 4, !tbaa !37, !alias.scope !47, !noalias !48
  %31 = fadd fast float %30, %27
  %32 = add nuw nsw i32 %26, 1
  %33 = icmp eq i32 %32, %13
  br i1 %33, label %17, label %25, !llvm.loop !49

34:                                               ; preds = %5, %17
  ret void
}

; Function Attrs: argmemonly mustprogress nocallback nofree nosync nounwind willreturn
declare void @llvm.lifetime.start.p0i8(i64 immarg, i8* nocapture) #1

; Function Attrs: argmemonly mustprogress nofree nounwind willreturn
declare void @llvm.memcpy.p0i8.p1i8.i64(i8* noalias nocapture writeonly, i8 addrspace(1)* noalias nocapture readonly, i64, i1 immarg) #2

; Function Attrs: argmemonly mustprogress nocallback nofree nosync nounwind willreturn
declare void @llvm.lifetime.end.p0i8(i64 immarg, i8* nocapture) #1

; Function Attrs: argmemonly mustprogress nofree nounwind willreturn
declare void @llvm.memcpy.p1i8.p0i8.i64(i8 addrspace(1)* noalias nocapture writeonly, i8* noalias nocapture readonly, i64, i1 immarg) #2

attributes #0 = { argmemonly mustprogress nofree nosync nounwind "approx-func-fp-math"="true" "frame-pointer"="all" "min-legal-vector-width"="0" "no-builtins" "no-infs-fp-math"="true" "no-nans-fp-math"="true" "no-signed-zeros-fp-math"="true" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "unsafe-fp-math"="true" }
attributes #1 = { argmemonly mustprogress nocallback nofree nosync nounwind willreturn }
attributes #2 = { argmemonly mustprogress nofree nounwind willreturn }

!llvm.module.flags = !{!0, !1, !2, !3, !4, !5, !6, !7, !8}
!air.kernel = !{!9}
!air.compile_options = !{!18, !19, !20}
!llvm.ident = !{!21}
!air.version = !{!22}
!air.language_version = !{!23}
!air.source_file_name = !{!24}

!0 = !{i32 2, !"SDK Version", [2 x i32] [i32 27, i32 0]}
!1 = !{i32 1, !"wchar_size", i32 4}
!2 = !{i32 7, !"frame-pointer", i32 2}
!3 = !{i32 7, !"air.max_device_buffers", i32 31}
!4 = !{i32 7, !"air.max_constant_buffers", i32 31}
!5 = !{i32 7, !"air.max_threadgroup_buffers", i32 31}
!6 = !{i32 7, !"air.max_textures", i32 128}
!7 = !{i32 7, !"air.max_read_write_textures", i32 8}
!8 = !{i32 7, !"air.max_samplers", i32 16}
!9 = !{void (%struct.Pt addrspace(1)*, %struct.Pt addrspace(1)*, float addrspace(1)*, i32 addrspace(2)*, i32)* @sp1, !10, !11}
!10 = !{}
!11 = !{!12, !14, !15, !16, !17}
!12 = !{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 1, !"air.struct_type_info", !13, !"air.arg_type_size", i32 20, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"Pt", !"air.arg_name", !"src"}
!13 = !{i32 0, i32 4, i32 0, !"int", !"x", i32 4, i32 4, i32 0, !"int", !"y", i32 8, i32 4, i32 0, !"float", !"s", i32 12, i32 4, i32 0, !"float", !"a", i32 16, i32 4, i32 0, !"float", !"r"}
!14 = !{i32 1, !"air.buffer", !"air.location_index", i32 1, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.struct_type_info", !13, !"air.arg_type_size", i32 20, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"Pt", !"air.arg_name", !"dst"}
!15 = !{i32 2, !"air.buffer", !"air.location_index", i32 2, i32 1, !"air.read", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"float", !"air.arg_name", !"img"}
!16 = !{i32 3, !"air.buffer", !"air.buffer_size", i32 4, !"air.location_index", i32 3, i32 1, !"air.read", !"air.address_space", i32 2, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"int", !"air.arg_name", !"n"}
!17 = !{i32 4, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint", !"air.arg_name", !"gid"}
!18 = !{!"air.compile.denorms_disable"}
!19 = !{!"air.compile.fast_math_enable"}
!20 = !{!"air.compile.framebuffer_fetch_enable"}
!21 = !{!"Apple metal version 32023.917 (metalfe-32023.917.2)"}
!22 = !{i32 2, i32 7, i32 0}
!23 = !{!"Metal", i32 3, i32 2, i32 0}
!24 = !{!"/private/tmp/build/-Users-nullmoth-Work-apple/44d387b3-1da3-49cf-b6aa-1d066e9bf129/scratchpad/b70/sp.metal"}
!25 = !{!26, !26, i64 0}
!26 = !{!"int", !27, i64 0}
!27 = !{!"omnipotent char", !28, i64 0}
!28 = !{!"Simple C++ TBAA"}
!29 = !{!30}
!30 = distinct !{!30, !31, !"air-alias-scope-arg(3)"}
!31 = distinct !{!31, !"air-alias-scopes(sp1)"}
!32 = !{!33, !34, !35}
!33 = distinct !{!33, !31, !"air-alias-scope-arg(0)"}
!34 = distinct !{!34, !31, !"air-alias-scope-arg(1)"}
!35 = distinct !{!35, !31, !"air-alias-scope-arg(2)"}
!36 = !{i64 0, i64 4, !25, i64 4, i64 4, !25, i64 8, i64 4, !37, i64 12, i64 4, !37, i64 16, i64 4, !37}
!37 = !{!38, !38, i64 0}
!38 = !{!"float", !27, i64 0}
!39 = !{!33}
!40 = !{!34, !35, !30}
!41 = !{i64 0, i64 4, !25, i64 4, i64 4, !37, i64 8, i64 4, !37, i64 12, i64 4, !37}
!42 = !{!34}
!43 = !{!33, !35, !30}
!44 = !{!45, !38, i64 12}
!45 = !{!"_ZTS2Pt", !26, i64 0, !26, i64 4, !38, i64 8, !38, i64 12, !38, i64 16}
!46 = !{!45, !38, i64 16}
!47 = !{!35}
!48 = !{!33, !34, !30}
!49 = distinct !{!49, !50}
!50 = !{!"llvm.loop.mustprogress"}
