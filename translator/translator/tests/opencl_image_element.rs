//! Apple's OpenCL declares every image argument as texture2d<unknown, ...>; the element type exists only in the image
//! intrinsics (air.write_texture_2d.u.v4i32 for write_imageui). Reading "unknown" as float made the storage image
//! Rgba32f and refused the uint4 write ("air.write_texture: unsupported texel shape"), so Geekbench 6's OpenCL Face
//! Detection convolve kernels never built. The fixture is tools/oracles/opencl_imageui.c's kernel as the driver got it.
use metal2vulkan::passes::Stage;
use metal2vulkan::{disassemble, translate_sanitized_native};
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("m2v_opencl_image_element_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

#[test]
fn opencl_write_imageui_into_an_unknown_image_translates_as_uint() {
    let ll = include_str!("fixtures/opencl/conv_imageui.ll");
    let spv = translate_sanitized_native(ll, Stage::Kernel, &tmp()).expect("write_imageui into texture2d<unknown, write> must translate");
    let asm = disassemble(&spv).expect("disassemble");
    let uints: Vec<&str> = asm.lines().filter_map(|l| match l.split_whitespace().collect::<Vec<_>>()[..] {
        [id, "=", "OpTypeInt", "32", "0"] => Some(id),
        _ => None,
    }).collect();
    let storage = asm.lines().filter_map(|l| match l.split_whitespace().collect::<Vec<_>>()[..] {
        [_, "=", "OpTypeImage", ty, _, _, _, _, "2", ..] => Some(ty),
        _ => None,
    }).collect::<Vec<_>>();
    assert!(!storage.is_empty(), "the written image is a storage image (sampled = 2):\n{asm}");
    assert!(storage.iter().all(|ty| uints.contains(ty)), "every storage image is uint, not float: {storage:?}\n{asm}");
}
