# Typed access lowering

This subsystem owns structural pointer, access-chain, and typed-memory normalization over the
retained SPIR-V module:

- `access_provenance.rs` traces rooted access chains, local pointer fields, pointees, and composed
  indices.
- `access_chain.rs` normalizes index widths, scalar pointer arithmetic, strides, and over-indexed
  chains.
- `index_remap.rs`, `dynamic_reinterpret.rs`, and `raw_byte.rs` remap AIR aggregate indices and
  replay structurally proven typed/raw reinterpretation paths.
- `vector_subword.rs` and `byte_aggregate.rs` lower cross-member, subword, and aggregate byte
  accesses.
- `scalar_store.rs` normalizes scalar stores whose pointer carrier needs structural repair.
- `private_memory.rs` owns Private null/placeholder handling, local atomics, function-variable
  hoisting, and arithmetic guards.
- `workgroup.rs` remodels structurally identified Workgroup aggregate and atomic layouts.

Structured CFG ownership belongs to the native emitter's typed structurizer. Final entry
construction belongs to `passes/finalize.rs`, with
dead-global/debug cleanup and capability closure in `passes/module_cleanup.rs`.
