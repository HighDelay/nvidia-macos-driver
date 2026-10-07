# Workgroup interface materialization

This subsystem owns the Workgroup-facing portion of interface construction:

- `types.rs` builds metadata-shaped Workgroup backing types and isolates Workgroup type graphs from
  explicit StorageBuffer layout decoration.
- `pointers.rs` materializes rooted pointer uses after entry parameters are spliced to real
  interface variables. The generic rooted-pointer machinery stays here because Workgroup and
  StorageBuffer/Private roots require the same transitive type repair.
- `zero_initialize.rs` inserts the kernel-entry zero-fill and Workgroup memory barrier that make
  the validation harness's refinement of undefined threadgroup contents deterministic.

`passes/stage_input` decides which decoded AIR parameter is Workgroup memory and invokes interface
materialization in the established stage-input order.
