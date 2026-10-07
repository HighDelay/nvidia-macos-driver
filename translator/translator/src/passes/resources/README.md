# Resource mechanisms

This subsystem reads the retained typed module and decoded AIR metadata to identify resource
shapes and construct their descriptor ABI:

- `bindings.rs` owns descriptor-set/input-attachment decoration, fixed/dynamic binding allocation,
  and the public reflection-ABI base mapping.
- `collapse.rs` applies parameter bindings, splices resource values, and collapses resource/buffer
  wrappers while preserving typed sidecar provenance.
- `discovery.rs` classifies buffer access structure and texture dimensions/component/storage use.
- `buffer_addresses.rs` consumes typed buffer-address sidecar facts and materializes the reflected
  address-table resource.
- `texture_array.rs` materializes runtime-indexed `array_ref<texture>` element loads from the
  descriptor arrays created by `ImageArray` bindings.
- `rewrites/` owns resource-rooted raw-word access, Private atomic lowering, AIR struct-member
  remapping, and structural-load repair, together with their focused fixtures.

This pass owns resource identity and the descriptor ABI, not consumer footprint inference. Reflected
buffer byte footprints are derived later in `src/reflect/footprint.rs` from the final module that
passed validation and retry selection.
