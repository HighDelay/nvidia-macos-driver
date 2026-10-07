# Stage input

This subsystem turns decoded AIR entry parameters into Vulkan stage-input, builtin, and resource
variables, then applies the completed binding plan to the entry body.

- `mod.rs` owns parameter-role planning and the ordered stage-input/resource construction pass.
- `air_layout.rs` contains AIR-type and builtin-shape adapters used during that plan.
- `kernel_values.rs` materializes compute dispatch builtins and constant dispatch values.
- `decorations.rs` owns stage location, interpolation, and builtin decorations.
- `layout.rs` decorates resource blocks and isolates explicit layout from Workgroup aliases.

The pass returns its original type-definition snapshot to the immediately-following output rewrite,
preserving the former `build_interface` ordering and lookup basis.

Byte scalar/vector varyings use flat uint32 transport slots shared with vertex output. Entry loads truncate to the original AIR byte type before executing the shader body.
