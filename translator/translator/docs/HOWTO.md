# How to translate and integrate a shader

This guide takes one Metal AIR or sanitized LLVM-IR module from input bytes to a validated Vulkan
shader and the host state needed to use it. For the complete field-by-field reflection contract, see
[Shader reflection for consumers](REFLECTION.md).

## 1. Install the tools

Product translation uses two external executables:

- `llvm-dis` converts AIR bitcode to LLVM IR. It is not needed when the input is already textual
  `.ll`.
- `spirv-val` validates the single constructed output under the Vulkan 1.2 environment. Its verdict
  cannot trigger a repair or select another representation.

Put both tools on `PATH`, or set an absolute per-tool override:

```sh
export METAL2VULKAN_LLVM_DIS=/path/to/llvm-dis
export METAL2VULKAN_SPIRV_VAL=/path/to/spirv-val
```

Install the CLI with reflection JSON enabled:

```sh
cargo install metal2vulkan --features serde
```

## 2. Translate from the command line

The default `--stage auto` reads `!air.vertex`, `!air.fragment`, or `!air.kernel` metadata from the
module:

```sh
metal2vulkan input.air output.spv --emit-meta reflection.json
```

Successful output contains `PASS`; the process exits `0` after writing validated SPIR-V and schema-
versioned reflection. Use an explicit stage only when appropriate:

```sh
metal2vulkan input.ll output.spv --stage vertex
metal2vulkan input.ll output.spv --stage fragment
metal2vulkan input.ll output.spv --stage kernel
metal2vulkan input.ll passthrough.spv --stage passthrough
```

Passthrough generation has no Metal interface metadata, so it cannot be combined with
`--emit-meta`. The accepted alias `--stage compute` is equivalent to `kernel`.

These options affect pipeline- or dispatch-dependent lowering:

| Option | Supply it when |
|---|---|
| `--raster-samples 1|2|4|8|16|32|64` | Fragment AIR calls `air.get_num_samples.i32`; use the exact graphics-pipeline sample count |
| `--local X,Y,Z` | Kernel dispatches use a threadgroup size other than the default `64,1,1` |
| `--threads-per-grid X,Y,Z` | Reflect one fixed Metal `dispatchThreads` grid for region planning |
| `--threads-per-grid-push-constant OFFSET` | Move the default 48-byte dispatch-region payload from offset 0 to `OFFSET` |
| `--whole-workgroups` | Assert every kernel launch covers complete workgroups and use one fixed-local-size pipeline |

Every `air.simd_*` lowering keeps Metal's 32-lane simdgroup contract unconditionally; there is no
option for it, because there is no correct module that wants the driver's subgroup width instead.
Do not use either option as a workaround for an unrelated translation failure.

## 3. Translate from Rust

Use `translate_reflected` for an `.air`/`.ll` path and
`translate_sanitized_native_reflected` when sanitized LLVM IR is already in memory. All translation
entry points require caller-owned scratch space. Give concurrent calls different directories and
remove each directory on success or failure.

The repository includes a complete, compiled example that does this cleanup and writes both output
files:

```sh
cargo run --features serde --example translate_reflected -- \
  input.air output.spv reflection.json auto
```

The core call is:

```rust
use metal2vulkan::passes::{Stage, TransformOptions};
use std::path::Path;

fn translate_ir(
    sanitized_ll: &str,
    scratch: &Path,
) -> Result<(Vec<u8>, metal2vulkan::reflect::ShaderReflection), String> {
    metal2vulkan::translate_sanitized_native_reflected(
        sanitized_ll,
        Stage::Kernel,
        scratch,
        TransformOptions::default(),
    )
}
```

For path input, call `detect_stage(path, scratch)` and pass the resulting `Stage` to
`translate_reflected`. Translation already validates the final module; calling `spirv-val` again is
useful only as an independent deployment check or when bytes have changed after translation.

## 4. Create the descriptor-set layout

Reflected descriptors use `reflection.descriptor_layout.set` (set `0` in the default layout), and
come from three top-level places. A complete
consumer must inspect all three:

1. `bindings[*].descriptor`
2. `implicit_imageblock_attachments[*].binding`
3. `fragment_imageblock.members[*].binding` when non-null

The latter two are single storage-image descriptors in the effective set. For an entry in `bindings`, use
`descriptor.set`, `descriptor.binding`, and `descriptor.count` exactly as reported:

| `ResourceBinding.kind` | Vulkan descriptor type |
|---|---|
| `Buffer`, `KernelStageInput`, `AccelerationStructureShadow`, `PrimitiveAccelerationStructure`, `BufferAddressTable` | Storage buffer |
| `Texture`, `EmbeddedArgBufferTexture` | Sampled image; a `Texture` whose reflected dimension is `Buffer` uses a uniform texel buffer |
| `TextureArray` with `access: Sampled` | Sampled-image array; reflected dimension `Buffer` uses a uniform texel-buffer array |
| `TextureArray` with `access: Storage` | Storage-image array; reflected dimension `Buffer` uses a storage texel-buffer array |
| `StorageImage` | Storage image; reflected dimension `Buffer` uses a storage texel buffer |
| `Sampler`, `StaticSampler` | Sampler |
| `ColorInput` | Input attachment |

`ThreadgroupBuffer`, `EmbeddedArgBufferBuffer`, visible/intersection function tables, and any other
entry with `descriptor: null` do not consume a Vulkan descriptor. Do not derive binding numbers
from list positions or Metal indices; synthesized resources are deliberately assigned by the
translator and reflection is authoritative.

Static samplers are ordinary Vulkan sampler descriptors whose creation state comes from
`static_sampler`. Dynamically bound samplers whose pipeline state affects shader legality must be
translated with `TransformOptions::with_runtime_sampler`; create their descriptors from the matching
`runtime_sampler_specializations` entries returned with the executable module. Never substitute the
default Vulkan binding (`160 + n`) for the API's Metal sampler index `n`. Embedded argument-buffer
textures use `embedded_source` to identify their owner, field offset, and Metal argument-encoder index.
Embedded buffers use matching entries in `argument_buffer_fields`; write the Vulkan device address
into the reflected owner field rather than allocating a separate descriptor.

Writable runtime textures must likewise be translated with
`TransformOptions::with_runtime_storage_image`, keyed by the Metal texture index for a top-level
binding or by the reflected `metal_index` for an `EmbeddedArgBufferTexture`. Embedded indices are
translator-assigned; use reflection instead of recomputing them from field positions. Pass the
bound format and the device's per-format storage/atomic support plus enabled
read/write-without-format features. Use the returned `runtime_storage_image_specializations` entry
to create the matching image view. The reflected `spirv_format` and
`texture_shape.storage_format` are `None` only when translation proved that the shader's exact
operations are covered by the supplied formatless features.

When independently translated graphics stages would otherwise reuse the same Metal indices, give
each stage a complete layout whose resource bands do not collide in the pipeline layout:

```rust
use metal2vulkan::passes::TransformOptions;
use metal2vulkan::reflect::{DescriptorBindingRange, DescriptorLayout};

let fragment_layout = DescriptorLayout {
    set: 1,
    buffers: DescriptorBindingRange::from_base_count(1000, 32)?,
    sampled_textures: DescriptorBindingRange::from_base_count(1032, 128)?,
    samplers: DescriptorBindingRange::from_base_count(1160, 32)?,
    color_inputs: DescriptorBindingRange::from_base_count(1192, 8)?,
    imageblocks: DescriptorBindingRange::from_base_count(1200, 24)?,
    fragment_imageblocks: DescriptorBindingRange::from_base_count(1224, 256)?,
    storage_textures: DescriptorBindingRange::from_base_count(1480, 128)?,
    synthetic: DescriptorBindingRange::from_base_count(1640, 32)?,
    ..DescriptorLayout::default()
};
let options = TransformOptions::default().with_descriptor_layout(fragment_layout)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Keep the same `set` for stages sharing one Vulkan descriptor-set layout, choose disjoint bands where
their resources are unrelated, and use the returned effective layout rather than recomputing it.

## 5. Stage only the buffer bytes the shader can reach

Successful reflected translation attaches `footprint` to descriptor-backed `Buffer`,
`KernelStageInput`, and `AccelerationStructureShadow` resources. It describes accesses in the final
constructed SPIR-V module returned by translation.

Apply this decision in order:

1. If `footprint` is null, there is no final-module proof. Retain the complete caller-provided
   window.
2. If `has_unbounded_access` is true, retain the complete window. Static or strided entries remain
   diagnostic only.
3. Otherwise, stage the union of `static_ranges` and every draw/dispatch-bounded
   `strided_accesses` range. An empty union means the final module does not access the binding.

Each static item `{ offset, size }` is the half-open interval `[offset, offset + size)`. Each strided
item describes accesses of `access_size` bytes at:

```text
base_offset + sum(actual_index_value(source) * stride)
```

Use the exact values generated by the current command, not counts alone. In particular,
`VertexIndex`, `InstanceIndex`, and `WorkgroupId` may include nonzero base values. Local invocation
IDs range over the pipeline's local size; global invocation IDs range over the dispatched grid. A
consumer can enumerate and coalesce the exact accesses, or conservatively copy the interval from
the minimum reachable address through the maximum reachable address plus `access_size`.

All offset, multiply, and end calculations must use checked arithmetic. Overflow means the
consumer cannot prove a bound and must retain the complete window. Clip computed ranges to neither
`declared_size` nor `extent`: `declared_size` may be only one pointee's size, while an unbounded AIR
pointer can legally index beyond it.

`Object { bytes }` is an independent AIR-metadata guarantee. If a supposedly bounded object's final
footprint extends beyond `bytes`, do not hide the inconsistency by clipping it: retain the complete
caller window and report the translator/reflection mismatch.

`access` answers read/write/unused classification when it can be proven. It does not replace the
footprint soundness gate. Treat `access: null` conservatively as read-write.

## 6. Configure the stage interface

- Create the Vulkan pipeline with entry point `"main"`; `entry_point` contains the original Metal
  function name for identity and diagnostics.
- Vertex stages use `vertex_attributes`, `varyings`, and `vertex_builtins`.
- A reflected `TessellationEvaluation` stage also uses `tessellation`; its control-point locations
  are arrays and its other listed locations carry patch data.
- Fragment stages use `varyings`, `render_targets`, `depth_members`, `stencil_members`, and
  `depth_qualifier`.
- Kernel stages use `local_size`. `imageblock_layouts` and threadgroup bindings describe Workgroup
  storage rather than descriptors.
- For kernels, obey `kernel_dispatch`. For `ThreadsDynamic` and `ThreadsFixed`, call
  `KernelDispatch::plan`, create one pipeline per distinct region `local_size` using
  `KERNEL_LOCAL_SIZE_SPEC_IDS`, and issue every returned region. Before each dispatch, write
  `KernelDispatchPlan::push_constants` into the reflected
  `KERNEL_DISPATCH_PUSH_CONSTANT_SIZE` range. Its four `u32[3]` fields are the logical thread grid,
  thread base, threadgroup base, and logical threadgroup grid. `Workgroups` is the only single
  fixed-local-size path and asserts that every workgroup is complete.
- Use `function_constants` to discover exact indices and Metal ABI type encodings. When exact scalar
  or vector payloads are supplied, translate sanitized AIR with
  `translate_sanitized_native_specialized_with_options`; specialization must precede metadata,
  resource-interface, and CFG construction. Metadata-only tooling must pass those same payloads to
  `reflect_sanitized_specialized`; reflecting the default AIR can omit a resource selected by a
  non-default value. A consumer that BINDS the specialized module should take its reflection from
  `translate_sanitized_native_specialized_reflected_with_options` instead: `reflect_sanitized*`
  constructs no module, so the image type behind a texture binding stays at the AIR type name, and a
  `texturecube` that is only ever texel-read is bound as a 2D array image. Exact predicates also remove false-gated resources from the specialized
  interface, so consumers bind the selected contract rather than the unspecialized union. A
  generated fullscreen companion must use `translate_passthrough_specialized` with the same
  payloads so its vertex outputs match the selected fragment inputs.

## 7. Cache outputs safely

Cache SPIR-V and reflection as one atomic result. A practical cache key includes:

- the exact input bytes;
- stage and every `TransformOptions` value;
- the metal2vulkan crate/binary version; and
- `reflection_version`.
- `descriptor_layout.version` (also already covered when hashing every `TransformOptions` value).

Invalidate both artifacts together when any key changes. Reflection is byte-neutral, but it is tied
to the exact final module returned by the same call. Metadata-only `reflect_sanitized` deliberately
has no final-module footprint and is not a substitute for reflected translation in a staging path.

## 8. Handle failures

The Rust API returns `Err(String)`. The CLI prints `FALLBACK`, exits nonzero, and writes a self-
contained repro bundle under `$TMPDIR/metal2vulkan-repros` unless `METAL2VULKAN_REPRO_DIR` overrides
the base directory. Unsupported input must remain a fallback; do not continue with partial metadata
or a construction that returned an error.

For local diagnosis:

```sh
METAL2VULKAN_RETRY_DEBUG=1 metal2vulkan input.air output.spv
METAL2VULKAN_WHY=1 metal2vulkan input.air output.spv
```

The first command traces construction phases (the environment-variable name is retained for
compatibility). The second reports structured-CFG admission decisions. These diagnostics do not
authorize a different product translation path.

## 9. Verify the integration

At minimum:

```sh
spirv-val --target-env vulkan1.2 output.spv
cargo test -p metal2vulkan --all-features
```

`spirv-val` checks structural validity, not Metal equivalence. For a semantic claim, follow the
[validation playbook](VALIDATION.md): use an owned synthetic regression, exact byte A/B where
appropriate, and an authored Metal/Vulkan case when output behavior changes.
