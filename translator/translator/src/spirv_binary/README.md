# Owned SPIR-V binary parser

This directory owns metal2vulkan's serialized SPIR-V grammar and parser boundary for the
SPIR-V core grammar shipped by **Vulkan SDK 1.4.341** (matching the direct `spirv` 0.4.0
dependency).

## Generated tables

`*_generated.rs` files are **regenerated** from the Khronos SPIR-V core grammar JSON
(`SPIRV-Headers` tag `vulkan-sdk-1.4.341.0`) using the public
[rspirv autogen](https://github.com/gfx-rs/rspirv/tree/master/autogen) tool, then adapted
for this crate. Do not hand-edit them.

```sh
./scripts/regen-spirv-grammar/regen-spirv-grammar.sh
```

The generator is a build-time utility only; it is not a crate dependency. The checked-in
outputs are mechanical grammar tables, not a vendored copy of the rspirv library.

## Hand-written modules

`decoder.rs`, `parser.rs`, `grammar.rs`, `type_tracker.rs`, and `mod.rs` are crate-owned
code that consumes those tables and produces crate-owned `spirv_module` nodes.
