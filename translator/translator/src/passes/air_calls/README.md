# AIR calls

This subsystem lowers residual stable AIR/LLVM intrinsic calls in the retained SPIR-V module:

- `dispatch_texture.rs` owns the single call-site dispatcher and texture/query/control intrinsics.
- `integer_simd.rs`, `float_imageblock.rs`, `reduce_bitops.rs`, and `matrix_shuffle.rs` own numeric,
  subgroup, imageblock, matrix, and shuffle families.
- `agx_emask.rs` owns the structurally decoded AGX execution-mask intrinsic family.
- `rawbyte_unary.rs`, `bfloat_glsl.rs`, and `conversions.rs` own raw-resource, unary math, bfloat,
  and conversion mechanisms.
- `images/` owns structural image resolution, coordinate/bounds construction, sampling, gather,
  query/offset, depth, read, and write helpers used by those call families.

Capability inventory and dispatch share the canonical exact-symbol contract in
`crate::air_intrinsics`. Lowerings still validate operand/result structure. In particular,
`matrix_shuffle.rs` owns both the scalarized 8x8 form and the distributed 32-lane 16x16x16 form;
the latter's ABI parser is shared with inventory so recognizing a new element encoding cannot drift
away from the implementation that consumes it.
