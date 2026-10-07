use metal2vulkan::passes::{Stage, TransformOptions};

const POOL_SHAPE: &str = r#"target triple = "air64_v27-apple-macosx15.7.0"

@_Z6kDest1.MTL_FC_INIT_118_j = internal local_unnamed_addr addrspace(2) externally_initialized constant i32 undef, section "air.fc_initializer", align 4
@_Z6kDest0.MTL_FC_INIT_119_j = internal local_unnamed_addr addrspace(2) externally_initialized constant i32 undef, section "air.fc_initializer", align 4
@_ZL5multi = internal unnamed_addr addrspace(2) global i8 0, align 1

declare i1 @air.is_function_constant_defined(ptr addrspace(2))
declare void @postfixPrimary_i.MTL_UNRESOLVED_VISIBLE_FN(ptr addrspace(1)) local_unnamed_addr section "air.externally_defined"

define internal void @_GLOBAL__sub_I_pool() section "air.static_init" {
  %1 = tail call i1 @air.is_function_constant_defined(ptr addrspace(2) @_Z6kDest0.MTL_FC_INIT_119_j)
  %2 = tail call i1 @air.is_function_constant_defined(ptr addrspace(2) @_Z6kDest1.MTL_FC_INIT_118_j)
  %3 = select i1 %1, i1 %2, i1 false
  %4 = zext i1 %3 to i8
  store i8 %4, ptr addrspace(2) @_ZL5multi, align 1
  ret void
}

define void @k(ptr addrspace(1) %out) {
entry:
  %m = load i8, ptr addrspace(2) @_ZL5multi, align 1
  %single = icmp eq i8 %m, 0
  br i1 %single, label %post, label %multi

post:
  call void @postfixPrimary_i.MTL_UNRESOLVED_VISIBLE_FN(ptr addrspace(1) %out)
  br label %done

multi:
  store i32 1, ptr addrspace(1) %out, align 4
  br label %done

done:
  ret void
}

!air.kernel = !{!0}
!air.function_constants = !{!5, !6}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3}
!3 = !{i32 0, !"air.buffer", !"air.location_index", i32 0, i32 1, !"air.read_write", !"air.address_space", i32 1, !"air.arg_type_size", i32 4, !"air.arg_type_align_size", i32 4, !"air.arg_type_name", !"int", !"air.arg_name", !"out"}
!5 = !{ptr addrspace(2) @_Z6kDest1.MTL_FC_INIT_118_j, !"uint", !"kDest1", i32 118, i1 false}
!6 = !{ptr addrspace(2) @_Z6kDest0.MTL_FC_INIT_119_j, !"uint", !"kDest0", i32 119, i1 false}
"#;

fn translate(values: &[(u32, Vec<u8>)]) -> Result<usize, String> {
    let sp = metal2vulkan::specialize_air_function_constants(POOL_SHAPE, values)?;
    metal2vulkan::translate_native_no_retry_constructed_with_options(
        sp.as_ref(),
        Stage::Kernel,
        TransformOptions::default(),
    )
    .map(|spv| spv.len())
}

const LIVE: &str = "is called on a live path";

#[test]
fn both_supplied_optional_constants_fold_the_guarded_visible_call_away() {
    let r = translate(&[(118, vec![0x0b, 0, 0, 0]), (119, vec![0x0b, 0, 0, 1])]);
    assert!(
        matches!(r, Ok(n) if n > 0),
        "both constants supplied => multi = 1 => the postfix call is dead and must fold: {r:?}"
    );
}

#[test]
fn unsupplied_optional_constants_keep_the_visible_call_live_and_refused() {
    let r = translate(&[]);
    assert!(
        matches!(&r, Err(e) if e.contains(LIVE)),
        "no constant supplied must keep the unresolved call live and refused, got {r:?}"
    );
}

#[test]
fn one_supplied_constant_keeps_the_conjunction_false_and_the_call_refused() {
    let r = translate(&[(119, vec![0x0b, 0, 0, 1])]);
    assert!(
        matches!(&r, Err(e) if e.contains(LIVE)),
        "half the conjunction supplied must not fold the call away, got {r:?}"
    );
}
