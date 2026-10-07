# metal2vulkan

[![License: LGPL-3.0-or-later](https://img.shields.io/badge/License-LGPL%203.0%20or%20later-blue.svg)](LICENSE)

> **Alpha.** The public API, CLI, reflection schema, and emitted SPIR-V may change during the `0.x`
> series. The project is suitable for experimentation and integration pilots, but consumers should
> version their cached outputs and expect upgrades to require integration work.

`metal2vulkan` translates **Metal AIR**—LLVM bitcode or sanitized LLVM IR—into Vulkan 1.2 SPIR-V
with a native Rust emitter and retained-SPIR-V passes.

## What it provides

- Vertex, post-tessellation vertex, fragment, compute-kernel, and generated passthrough interfaces
- A Rust library and an `in → out.spv` command-line interface
- Structurally selected construction with one final validation pass; unsupported inputs fail
  visibly instead of returning known-invalid SPIR-V
- Consumer reflection for descriptors, stage interfaces, function constants, argument buffers, and
  conservative buffer byte footprints
- An optional unpublished validation workspace for authored semantic cases, GPU evidence, and exact
  translator A/B

## Requirements

- Rust 1.87 or newer
- `llvm-dis` when the input is AIR bitcode
- `spirv-val` from SPIRV-Tools for product translation

Tool paths are found through the usual search path. Override an individual tool with
`METAL2VULKAN_LLVM_DIS`, `METAL2VULKAN_SPIRV_VAL`, or the corresponding
`METAL2VULKAN_<TOOL>` variable.

## Install

```sh
# CLI without reflection JSON
cargo install metal2vulkan

# CLI with --emit-meta JSON support
cargo install metal2vulkan --features serde
```

For a library dependency:

```toml
[dependencies]
metal2vulkan = { version = "0.1", features = ["serde"] } # serde is optional
```

## Quick start

The CLI auto-detects vertex, fragment, or kernel stage metadata by default:

```sh
metal2vulkan input.air output.spv
metal2vulkan input.air output.spv --emit-meta reflection.json
```

Use an explicit stage when the input does not carry stage metadata or when generating a passthrough
vertex shader:

```sh
metal2vulkan input.ll output.spv --stage kernel
metal2vulkan input.ll output.spv --stage passthrough
```

`--emit-meta` requires a build with the `serde` feature and is unavailable for passthrough output,
which has no Metal stage metadata. On success the CLI writes validated SPIR-V, prints `PASS`, and
exits `0`. On failure it prints `FALLBACK`, exits nonzero, and writes a reproducer under
`$TMPDIR/metal2vulkan-repros` by default. Set `METAL2VULKAN_REPRO_DIR` to choose another location.

Fragment AIR that calls `air.get_num_samples.i32` also needs the graphics pipeline's exact sample
count because Vulkan has no equivalent shader-side query:

```sh
metal2vulkan fragment.air fragment.spv --raster-samples 4
```

Accepted values are `1`, `2`, `4`, `8`, `16`, `32`, and `64`. Library callers set
`TransformOptions::raster_sample_count`; translation fails visibly when the query is present and
the value is unknown.

Metal `dispatchThreads` can make boundary workgroups smaller. Exact-thread kernels therefore expose
specializable local sizes and a reflected `KernelDispatch::plan`: create a compute pipeline for
each distinct region `local_size`, specializing `KERNEL_LOCAL_SIZE_SPEC_IDS`, then dispatch every
region with its 48-byte logical-grid payload. This produces at most eight rectangular dispatches,
with no surplus invocations and uniform workgroup barriers.

For a fixed grid, the CLI can instead bake the exact bounds into a pipeline variant:

```sh
metal2vulkan kernel.air kernel.spv --local 8,8,1 --threads-per-grid 57,9,1
```

`--threads-per-grid-push-constant OFFSET` moves the default dynamic dispatch payload.
Library callers select an override with `TransformOptions::kernel_dispatch`; reflected translation
returns the effective contract in `ShaderReflection::kernel_dispatch`. Callers that can prove every
launch covers complete workgroups may explicitly select `KernelDispatch::Workgroups` or pass
`--whole-workgroups` to retain a single fixed-local-size dispatch.

## Library

Library translation takes caller-owned scratch space. Use a unique directory for concurrent calls
and remove it when the operation finishes:

```rust
use metal2vulkan::passes::Stage;
use std::path::Path;

fn translate_kernel(sanitized_ll: &str, scratch: &Path) -> Result<Vec<u8>, String> {
    metal2vulkan::translate_sanitized_native(sanitized_ll, Stage::Kernel, scratch)
}
```

For a complete executable that auto-detects the stage, writes SPIR-V and reflection JSON, and cleans
up its scratch directory on every return path:

```sh
cargo run --features serde --example translate_reflected -- \
  input.air output.spv reflection.json
```

## Documentation

| Document | Start here when… |
|---|---|
| [How to translate and integrate a shader](docs/HOWTO.md) | You want an end-to-end CLI or Rust integration recipe |
| [Shader reflection](docs/REFLECTION.md) | You need the complete reflection schema and binding contract |
| [Architecture](docs/ARCHITECTURE.md) | You are changing the emitter, retained module, passes, or retries |
| [Validation playbook](docs/VALIDATION.md) | You are checking byte stability or authored semantic behavior |
| [Contributing](CONTRIBUTING.md) | You need the repository layout and required development gates |
| [Validation crate](validation/README.md) | You are operating the unpublished corpus tooling |
| [Changelog](CHANGELOG.md) | You need unreleased API and behavior changes |

The generated Rust API documentation is available from `cargo doc --all-features --open`.

## Coverage policy

Grammar and lowering behavior is locked with owned synthetic Rust tests. This repository does not
ship third-party captured shaders. Optional private system-metallib harvest, authored manifests,
and dependency-exact evidence stay in the validation workspace and are described in the
[validation playbook](docs/VALIDATION.md).

Passing `spirv-val` proves structural validity, not agreement with Metal or correct pixels. Use the
authored validation ladder for semantic claims.

## License

Licensed under the [GNU Lesser General Public License v3.0 or later](LICENSE)
(`LGPL-3.0-or-later`).

Metal is a trademark of Apple Inc. metal2vulkan is an independent project and is not affiliated
with, sponsored by, or endorsed by Apple Inc.
