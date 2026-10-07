use metal2vulkan::meta::parse_air_kernel_meta;
use metal2vulkan::passes::Stage;
use metal2vulkan::{disassemble, translate_sanitized_native};
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("m2v_wrapped_arg_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

const FLAT: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

%Args = type <{ %"struct.metal::texture2d", i16, [6 x i8] }>
%"struct.metal::texture2d" = type { ptr addrspace(1) }

define void @k(ptr addrspace(2) %args, <2 x i32> %coord) local_unnamed_addr #0 {
entry:
  %field = getelementptr inbounds %Args, ptr addrspace(2) %args, i64 0, i32 0, i32 0
  %tex = load ptr addrspace(1), ptr addrspace(2) %field, align 8
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) %tex, <2 x i32> %coord, <4 x float> zeroinitializer, i32 0, i32 2) #3
  ret void
}

declare void @air.write_texture_2d.v4f32(ptr addrspace(1), <2 x i32>, <4 x float>, i32, i32) local_unnamed_addr #3

attributes #0 = { convergent nounwind }
attributes #3 = { convergent nounwind memory(argmem: write) }

!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !7}
!3 = !{i32 0, !"air.indirect_buffer", !"air.buffer_size", i32 16, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.struct_type_info", !4, !"air.arg_type_name", !"Args", !"air.arg_name", !"args"}
!4 = !{i32 0, i32 8, i32 0, !"texture2d<float, write>", !"output", !"air.indirect_argument", !5, i32 8, i32 2, i32 0, !"short", !"radius", !"air.indirect_argument", !6}
!5 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"output"}
!6 = !{i32 1, !"air.indirect_constant", !"air.location_index", i32 1, i32 1, !"air.arg_type_name", !"short", !"air.arg_name", !"radius"}
!7 = !{i32 1, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint2", !"air.arg_name", !"coord"}
"#;

const WRAPPED: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

%Args = type <{ %Wrapper, i16, [6 x i8] }>
%Wrapper = type { %"struct.metal::texture2d" }
%"struct.metal::texture2d" = type { ptr addrspace(1) }

define void @k(ptr addrspace(2) %args, <2 x i32> %coord) local_unnamed_addr #0 {
entry:
  %field = getelementptr inbounds %Args, ptr addrspace(2) %args, i64 0, i32 0, i32 0, i32 0
  %tex = load ptr addrspace(1), ptr addrspace(2) %field, align 8
  tail call void @air.write_texture_2d.v4f32(ptr addrspace(1) %tex, <2 x i32> %coord, <4 x float> zeroinitializer, i32 0, i32 2) #3
  ret void
}

declare void @air.write_texture_2d.v4f32(ptr addrspace(1), <2 x i32>, <4 x float>, i32, i32) local_unnamed_addr #3

attributes #0 = { convergent nounwind }
attributes #3 = { convergent nounwind memory(argmem: write) }

!air.kernel = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{}
!2 = !{!3, !7}
!3 = !{i32 0, !"air.indirect_buffer", !"air.buffer_size", i32 16, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.struct_type_info", !4, !"air.arg_type_name", !"Args", !"air.arg_name", !"args"}
!4 = !{!"air.struct_type_info", !8, i32 0, i32 8, i32 0, !"texture2d_wrapper", !"wrapper", !"air.indirect_argument", i32 7, i32 8, i32 2, i32 0, !"short", !"radius", !"air.indirect_argument", !6}
!8 = !{i32 0, i32 8, i32 0, !"texture2d<float, write>", !"output", !"air.indirect_argument", !5}
!5 = !{i32 0, !"air.texture", !"air.location_index", i32 3, i32 1, !"air.write", !"air.arg_type_name", !"texture2d<float, write>", !"air.arg_name", !"output"}
!6 = !{i32 1, !"air.indirect_constant", !"air.location_index", i32 1, i32 1, !"air.arg_type_name", !"short", !"air.arg_name", !"radius"}
!7 = !{i32 1, !"air.thread_position_in_grid", !"air.arg_type_name", !"uint2", !"air.arg_name", !"coord"}
"#;

const WRAPPED_SAMPLE: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

%Args = type <{ %Wrapper }>
%Wrapper = type { %"struct.metal::texture2d" }
%"struct.metal::texture2d" = type { ptr addrspace(1) }

define <4 x float> @k(ptr addrspace(2) %args, ptr addrspace(2) %smp) local_unnamed_addr #0 {
entry:
  %tf = getelementptr inbounds %Args, ptr addrspace(2) %args, i64 0, i32 0, i32 0, i32 0
  %tex = load ptr addrspace(1), ptr addrspace(2) %tf, align 8
  %s = call { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1) %tex, ptr addrspace(2) %smp, <2 x float> zeroinitializer, i1 true, <2 x i32> zeroinitializer, i1 false, float 0.000000e+00, float 0.000000e+00, i32 0)
  %c = extractvalue { <4 x float>, i8 } %s, 0
  ret <4 x float> %c
}

declare { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1), ptr addrspace(2), <2 x float>, i1, <2 x i32>, i1, float, float, i32)

attributes #0 = { convergent nounwind }

!air.fragment = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{!9}
!9 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!2 = !{!3, !6}
!3 = !{i32 0, !"air.indirect_buffer", !"air.buffer_size", i32 8, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.struct_type_info", !4, !"air.arg_type_name", !"Args", !"air.arg_name", !"args"}
!4 = !{!"air.struct_type_info", !8, i32 0, i32 8, i32 0, !"texture2d_wrapper", !"wrapper", !"air.indirect_argument", i32 0}
!8 = !{i32 0, i32 8, i32 0, !"texture2d<float, sample>", !"tex", !"air.indirect_argument", !5}
!5 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.sample", !"air.arg_type_name", !"texture2d<float, sample>", !"air.arg_name", !"tex"}
!6 = !{i32 1, !"air.sampler", !"air.location_index", i32 0, i32 1, !"air.arg_type_name", !"sampler", !"air.arg_name", !"smp"}
"#;

const WRAPPED_1D: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

%Args = type <{ %Wrapper }>
%Wrapper = type { %"struct.metal::texture1d" }
%"struct.metal::texture1d" = type { ptr addrspace(1) }

define <4 x float> @k(ptr addrspace(2) %args, ptr addrspace(2) %smp) local_unnamed_addr #0 {
entry:
  %tf = getelementptr inbounds %Args, ptr addrspace(2) %args, i64 0, i32 0, i32 0, i32 0
  %tex = load ptr addrspace(1), ptr addrspace(2) %tf, align 8
  %s = call { <4 x float>, i8 } @air.sample_texture_1d.v4f32(ptr addrspace(1) %tex, ptr addrspace(2) %smp, float 0.000000e+00, i1 false, float 0.000000e+00, i32 0)
  %c = extractvalue { <4 x float>, i8 } %s, 0
  ret <4 x float> %c
}

declare { <4 x float>, i8 } @air.sample_texture_1d.v4f32(ptr addrspace(1), ptr addrspace(2), float, i1, float, i32)

attributes #0 = { convergent nounwind }

!air.fragment = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{!9}
!9 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!2 = !{!3, !6}
!3 = !{i32 0, !"air.indirect_buffer", !"air.buffer_size", i32 8, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.struct_type_info", !4, !"air.arg_type_name", !"Args", !"air.arg_name", !"args"}
!4 = !{!"air.struct_type_info", !8, i32 0, i32 8, i32 0, !"texture1d_wrapper", !"wrapper", !"air.indirect_argument", i32 0}
!8 = !{i32 0, i32 8, i32 0, !"texture1d<float, sample>", !"tex", !"air.indirect_argument", !5}
!5 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.sample", !"air.arg_type_name", !"texture1d<float, sample>", !"air.arg_name", !"tex"}
!6 = !{i32 1, !"air.sampler", !"air.location_index", i32 0, i32 1, !"air.arg_type_name", !"sampler", !"air.arg_name", !"smp"}
"#;

const WRAPPED_ARRAY: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

%Args = type <{ [2 x %Wrapper] }>
%Wrapper = type { %"struct.metal::texture2d" }
%"struct.metal::texture2d" = type { ptr addrspace(1) }

define <4 x float> @k(ptr addrspace(2) %args, ptr addrspace(2) %smp) local_unnamed_addr #0 {
entry:
  %tf = getelementptr inbounds %Args, ptr addrspace(2) %args, i64 0, i32 0, i32 0, i32 0, i32 0
  %tex = load ptr addrspace(1), ptr addrspace(2) %tf, align 8
  %s = call { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1) %tex, ptr addrspace(2) %smp, <2 x float> zeroinitializer, i1 true, <2 x i32> zeroinitializer, i1 false, float 0.000000e+00, float 0.000000e+00, i32 0)
  %c = extractvalue { <4 x float>, i8 } %s, 0
  ret <4 x float> %c
}

declare { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1), ptr addrspace(2), <2 x float>, i1, <2 x i32>, i1, float, float, i32)

attributes #0 = { convergent nounwind }

!air.fragment = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{!9}
!9 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!2 = !{!3, !6}
!3 = !{i32 0, !"air.indirect_buffer", !"air.buffer_size", i32 16, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.struct_type_info", !4, !"air.arg_type_name", !"Args", !"air.arg_name", !"args"}
!4 = !{!"air.struct_type_info", !8, i32 0, i32 8, i32 2, !"texture2d_wrapper", !"wrappers", !"air.indirect_argument", i32 0}
!8 = !{i32 0, i32 8, i32 0, !"texture2d<float, sample>", !"tex", !"air.indirect_argument", !5}
!5 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.sample", !"air.arg_type_name", !"texture2d<float, sample>", !"air.arg_name", !"tex"}
!6 = !{i32 1, !"air.sampler", !"air.location_index", i32 0, i32 1, !"air.arg_type_name", !"sampler", !"air.arg_name", !"smp"}
"#;

const FLAT_C_ARRAY: &str = r#"target triple = "air64_v28-apple-macosx26.5.0"

%Args = type { [32 x %"struct.metal::texture2d"] }
%"struct.metal::texture2d" = type { ptr addrspace(1) }

define <4 x float> @k(ptr addrspace(2) %args, ptr addrspace(2) %smp, i32 %slice) local_unnamed_addr #0 {
entry:
  %w = zext i32 %slice to i64
  %tf = getelementptr inbounds %Args, ptr addrspace(2) %args, i64 0, i32 0, i64 %w, i32 0
  %tex = load ptr addrspace(1), ptr addrspace(2) %tf, align 8
  %s = call { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1) %tex, ptr addrspace(2) %smp, <2 x float> zeroinitializer, i1 true, <2 x i32> zeroinitializer, i1 false, float 0.000000e+00, float 0.000000e+00, i32 0)
  %c = extractvalue { <4 x float>, i8 } %s, 0
  ret <4 x float> %c
}

declare { <4 x float>, i8 } @air.sample_texture_2d.v4f32(ptr addrspace(1), ptr addrspace(2), <2 x float>, i1, <2 x i32>, i1, float, float, i32)

attributes #0 = { convergent nounwind }

!air.fragment = !{!0}
!0 = !{ptr @k, !1, !2}
!1 = !{!9}
!9 = !{!"air.render_target", i32 0, i32 0, !"air.arg_type_name", !"float4"}
!2 = !{!3, !6, !7}
!3 = !{i32 0, !"air.indirect_buffer", !"air.buffer_size", i32 256, !"air.location_index", i32 0, i32 1, !"air.read", !"air.address_space", i32 2, !"air.struct_type_info", !4, !"air.arg_type_name", !"Args", !"air.arg_name", !"args"}
!4 = !{i32 0, i32 8, i32 32, !"texture2d<float, sample>", !"slices", !"air.indirect_argument", !5}
!5 = !{i32 0, !"air.texture", !"air.location_index", i32 0, i32 1, !"air.sample", !"air.arg_type_name", !"texture2d<float, sample>", !"air.arg_name", !"slices"}
!6 = !{i32 1, !"air.sampler", !"air.location_index", i32 0, i32 1, !"air.arg_type_name", !"sampler", !"air.arg_name", !"smp"}
!7 = !{i32 2, !"air.fragment_input", !"generated(5slicej)", !"air.flat", !"air.arg_type_name", !"uint", !"air.arg_name", !"slice"}
"#;

#[test]
fn a_flat_embedded_texture_still_binds() {
    let spv = translate_sanitized_native(FLAT, Stage::Kernel, &tmp()).expect("translate");
    let asm = disassemble(&spv).expect("disassemble");
    assert!(
        asm.contains("Binding 480") && asm.contains("OpImageWrite"),
        "the flat member is surfaced as a storage image and written:\n{asm}"
    );
    let meta = parse_air_kernel_meta(FLAT).expect("parse");
    assert!(
        meta.unsurfaced_embedded_resources.is_empty(),
        "nothing about the flat form is unsurfaced: {:?}",
        meta.unsurfaced_embedded_resources
    );
}

#[test]
fn a_wrapped_embedded_texture_binds_at_the_summed_argument_id() {
    let meta = parse_air_kernel_meta(WRAPPED).expect("parse");
    assert!(
        meta.unsurfaced_embedded_resources.is_empty(),
        "a wrapped 2D texture is surfaced, not refused: {:?}",
        meta.unsurfaced_embedded_resources
    );
    let texture = match meta.embedded_textures.as_slice() {
        [only] => *only,
        other => panic!("exactly one embedded texture, got {other:?}"),
    };
    assert_eq!(texture.argument_index, 10);
    assert_eq!(texture.field_offset, 0);
    assert_eq!(texture.field_ordinal, 0);

    let spv = translate_sanitized_native(WRAPPED, Stage::Kernel, &tmp()).expect("translate");
    let asm = disassemble(&spv).expect("disassemble");
    assert!(
        asm.contains("Binding 480") && asm.contains("OpImageWrite"),
        "the wrapped member is surfaced as a storage image and written:\n{asm}"
    );
}

#[test]
fn a_wrapped_embedded_texture_samples_its_own_image() {
    let spv =
        translate_sanitized_native(WRAPPED_SAMPLE, Stage::Fragment, &tmp()).expect("translate");
    let asm = disassemble(&spv).expect("disassemble");
    assert!(
        asm.contains("OpSampledImage") && asm.contains("OpImageSample"),
        "the wrapped texture is sampled through its own descriptor:\n{asm}"
    );
}

#[test]
fn a_wrapped_texture_keeps_the_dimension_its_type_name_names() {
    let spv = translate_sanitized_native(WRAPPED_1D, Stage::Fragment, &tmp()).expect("translate");
    let asm = disassemble(&spv).expect("disassemble");
    assert!(
        asm.contains(" 1D 0 0 0 1 Unknown") && asm.contains("OpImageSample"),
        "the wrapped 1D texture is sampled through a 1D image:\n{asm}"
    );
}

#[test]
fn an_arrayed_wrapper_still_refuses() {
    let error = translate_sanitized_native(WRAPPED_ARRAY, Stage::Fragment, &tmp())
        .expect_err("one of several declared textures must not be the only one bound");
    assert!(
        error.contains("does not surface") && error.contains("air.texture"),
        "the refusal names the resource that was missed: {error}"
    );
}

#[test]
fn a_flat_c_array_member_is_indexed_rather_than_collapsed_to_element_zero() {
    let meta = metal2vulkan::meta::parse_air_fragment_meta(FLAT_C_ARRAY).expect("parse");
    assert!(
        meta.unsurfaced_embedded_resources.is_empty(),
        "a C-array texture member is surfaced, not refused: {:?}",
        meta.unsurfaced_embedded_resources
    );
    let texture = match meta.embedded_textures.as_slice() {
        [only] => *only,
        other => panic!("exactly one embedded texture argument, got {other:?}"),
    };
    assert_eq!(texture.array_length, Some(32));

    let spv = translate_sanitized_native(FLAT_C_ARRAY, Stage::Fragment, &tmp()).expect("translate");
    let asm = disassemble(&spv).expect("disassemble");
    let module = Disassembly::parse(&asm);

    let (array_ty, image_ty) = module
        .results()
        .find_map(|(id, op, args)| {
            let [element, length] = args else { return None };
            (op == "OpTypeArray" && module.constant_value(length) == Some(32))
                .then_some((id, *element))
        })
        .unwrap_or_else(|| panic!("a 32-element descriptor array is declared:\n{asm}"));
    assert_eq!(
        module.opcode(image_ty),
        Some("OpTypeImage"),
        "the array's element is the image type:\n{asm}"
    );
    let array_ptr = module
        .results()
        .find_map(|(id, op, args)| {
            (op == "OpTypePointer" && args.first() == Some(&array_ty)).then_some(id)
        })
        .unwrap_or_else(|| panic!("a pointer to the array type is declared:\n{asm}"));
    let array_var = module
        .results()
        .find_map(|(id, op, args)| {
            (op == "OpVariable" && args.first() == Some(&array_ptr)).then_some(id)
        })
        .unwrap_or_else(|| panic!("the array is bound as one variable, not one image:\n{asm}"));

    let element = module
        .results()
        .find_map(|(id, op, args)| match args {
            [_, base, index] if op == "OpAccessChain" && *base == array_var => {
                assert!(
                    module.constant_value(index).is_none(),
                    "the index is the runtime value, not a constant:\n{asm}"
                );
                Some(id)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("the array is indexed before it is loaded:\n{asm}"));
    let loaded = module
        .results()
        .find_map(|(id, op, args)| (op == "OpLoad" && args.get(1) == Some(&element)).then_some(id))
        .unwrap_or_else(|| panic!("the selected element is loaded:\n{asm}"));
    assert!(
        module
            .results()
            .any(|(_, op, args)| op == "OpSampledImage" && args.get(1) == Some(&loaded)),
        "and the loaded element is what gets sampled:\n{asm}"
    );
}

struct Disassembly {
    defs: Vec<(u32, String, Vec<u32>)>,
    constants: std::collections::HashMap<u32, u32>,
}

impl Disassembly {
    fn parse(asm: &str) -> Self {
        let mut defs = vec![];
        let mut constants = std::collections::HashMap::new();
        for line in asm.lines() {
            let Some((result, rest)) = line.split_once('=') else {
                continue;
            };
            let Some(result) = result
                .trim()
                .strip_prefix('%')
                .and_then(|id| id.parse().ok())
            else {
                continue;
            };
            let mut tokens = rest.split_whitespace();
            let Some(opcode) = tokens.next() else {
                continue;
            };
            if opcode == "OpConstant" {
                if let Some(value) = tokens.clone().nth(1).and_then(|tok| tok.parse().ok()) {
                    constants.insert(result, value);
                }
            }
            let ids = tokens
                .filter_map(|tok| tok.strip_prefix('%')?.parse().ok())
                .collect();
            defs.push((result, opcode.to_string(), ids));
        }
        Self { defs, constants }
    }

    fn results(&self) -> impl Iterator<Item = (u32, &str, &[u32])> {
        self.defs
            .iter()
            .map(|(id, op, args)| (*id, op.as_str(), args.as_slice()))
    }

    fn opcode(&self, id: u32) -> Option<&str> {
        self.results()
            .find(|(def, ..)| *def == id)
            .map(|(_, op, _)| op)
    }

    fn constant_value(&self, id: &u32) -> Option<u32> {
        self.constants.get(id).copied()
    }
}
