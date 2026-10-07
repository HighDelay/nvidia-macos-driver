# Stage output

This subsystem owns the remaining stage-interface closure:

- rewrite fragment and vertex return values into Vulkan output variables and stores;
- decorate output locations and builtins;
- materialize AIR static sampler globals as descriptor-backed sampler resources.

The pipeline invokes output rewriting immediately after `stage_input`, using the exact pre-input
type-definition snapshot returned by that pass.

Vulkan forbids 8-bit Input/Output components. Byte scalar/vector user outputs are zero-extended to uint32 transport slots; fragment entry truncation restores their original signed or unsigned byte bits.
