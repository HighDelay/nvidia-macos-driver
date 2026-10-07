# metal2vulkan architecture

This document maps the AIR→SPIR-V product pipeline. The central construction contract is simple:
representation selection happens from AIR and owned-module facts, then one serialized module is
validated. Validator output is a verdict, never a code-generation input.

## Pipeline

```text
.air | sanitized .ll
  └─ llvm-dis when needed + AIR sanitization
  └─ shared pre-parse AIR lowering
  └─ stage metadata and target-layout parsing
  └─ typed LLVM/AIR parse
  └─ primary structured CFG and typed interface emission
  └─ owned SpirvModule + EmitSidecar
  └─ stage, resource, memory, and interface passes
  └─ owned-module construction
       ├─ close pointer/value and opaque-image representations
       ├─ prune statically unreachable CFG
       ├─ construct selected CFG functions with the bounded relooper
       ├─ check load/store/select and Logical-pointer type invariants
       └─ check descriptor ABI invariants
  └─ if an owned invariant rejects the primary representation:
       construct one structurally selected alternate representation
  └─ canonical assembly
  └─ spirv-val --target-env vulkan1.2
  └─ optional read-only reflection over the returned bytes
```

There is no production loop from `spirv-val` back into emission or rewriting. A validation failure
is returned as `Err` / CLI `FALLBACK`. The product does not parse the failed output and does not use
validator wording to select another representation.

`translate_native_no_retry` retains its compatibility name. It constructs the primary
representation up to, but not including, external validation and does not select an alternate when
an owned invariant rejects that representation. `translate_native_primary_validated` validates
that same primary-only construction. Ordinary `translate*` entry points may select an alternate
before validation from the structural facts described below.

## Typed ownership

The production body path is parse once → typed IR → owned SPIR-V module:

- LLVM text is consumed by `native/parse.rs`, `native/lex.rs`, and the AIR metadata parser.
- Instruction semantics are carried by typed `TirInst` / `TirBlock` fields. Unsupported opcodes
  fail visibly instead of being recovered from textual spellings.
- CFG construction mutates typed `BodyBlock` carriers. Loop, selection, and terminal roles are
  structural tags rather than name-based decisions.
- The native emitter and all retained passes share the crate-owned `SpirvModule`. Successful
  product construction does not serialize and reparse between passes.
- Within each chosen representation, raw buffer scope is fixed from typed AIR before function
  emission, including incompatible multi-root pointer merges and integer atomics whose source
  pointee cannot be addressed as an `i32` Logical pointer. A function is emitted once; emitter
  errors never rerun it with a broader buffer model.
- `EmitSidecar` carries typed facts that do not belong in SPIR-V instructions, including AIR
  layouts, buffer-address words, pointer-field stores, and CFG ownership rejections. Passes remap
  those facts when IDs change.

Four source transforms intentionally operate before typed parsing: async-copy lowering,
vector/scalar pointer-merge lowering, SROA, and internal-helper inlining. Alternate representations
receive the same already-lowered source, so they do not observe a different program.

## Validity by construction

`finish_module` is the common owned-module boundary for primary and alternate emission. It performs
the following work before assembly:

1. Run interface, resource, access, workgroup, stage-input, stage-output, finalization, and module
   cleanup passes.
2. Close cross-binding pointer operations in the value domain where their load/store closure is
   representable.
3. Retype Workgroup atomic storage and construct opaque-image selects using the consumers' image
   types.
4. Lower remaining representable cross-binding pointer closures to the address domain.
5. Canonicalize IDs while remapping the sidecar.
6. On the primary representation, remove invalid value chains made unreachable by static CFG
   pruning.
7. Reconstruct only CFG functions selected by source ownership or final owned-CFG facts.
8. On the selected raw-relooper representation, carry address-domain selection into final resource
   construction, then rebuild the selected CFG and canonicalize the completed graph.
9. Reject malformed result-type IDs and core type-declaration graphs before any instruction can
   treat an arbitrary defined ID as a type.
10. Reject owned arithmetic, comparison, shift, numeric-bitcast, width-conversion, bit-count,
   Boolean reduction, float-classification, vector-algebra, derivative, copy, select, phi, load,
   and store type disagreement; atomic pointer-pointee, result, value, comparator, scope, and
   memory-semantics type disagreement; unknown GLSL.std.450 opcodes and extended-instruction
   arity or type-shape disagreement; sampled-image result, image, and sampler type disagreement;
   image-query result, dimension, array, multisample, sampled/storage mode, LOD, and coordinate disagreement; derivatives
   and implicit-LOD queries reachable outside a Fragment call tree; texel fetch/read/write image mode,
   component, coordinate, result, texel, LOD, and sample disagreement; sampled-image operation result,
   coordinate, component, LOD, constant-offset, and stage disagreement; image-texel-pointer result,
   image, coordinate, sample, and atomic-format disagreement; barrier scope, memory-semantics, and
   execution-model disagreement; subgroup vote, ballot, shuffle, arithmetic, scope, index,
   group-operation, and cluster-size disagreement; access-chain base/result storage, integer index,
   structure-member, and result-pointee disagreement; invalid branch and switch
   controls; inconsistent function signatures, calls, parameters, and returns; inconsistent
   composite constituents, index paths, dynamic vector operations, and shuffle lanes; invalid
   structure-member indices; non-32-bit Vulkan 1.2 bit-field/count/reverse bases; and Logical pointer
   nulls, cross-root selects, or pointer-valued variables.
11. Reject descriptor bindings outside the configured ABI.
12. Assemble once.

The owned checks are deliberately narrower than a second SPIR-V validator. They enforce invariant
classes the translator itself has enough information to guarantee and use failures as construction
facts while the source and module are still owned. `spirv-val` remains the independent final
backstop for the complete Vulkan SPIR-V contract.

### Representation selection

Alternate construction is selected before validation:

| Owned fact | Constructed representation |
|---|---|
| Source CFG ownership rejection or final owned-CFG construction failure | raw-buffer relooper feed |
| Internal helper directly consumes a pointer-select result | consuming-helper inline + bounded CFG construction |

If the selected alternate cannot be constructed, translation fails honestly. The implementation
does not serialize multiple candidates and ask the validator which one to adopt.
Malformed declarations, load/store/select/access typing, ordinary value typing, environment
contracts, function contracts, and value-composite contracts are non-repairable validity failures.
They cannot select an alternate representation merely because they are discovered at the same
owned-module boundary as CFG facts. Raw buffer scope is chosen from typed AIR before emission.

Validator-message classifiers and byte-level repair adapters are deliberately absent. Diagnostics
inspect AIR or an owned module directly; validator text is only returned to the caller.

## CFG structurization

1. `native::cfg` builds a loop/selection forest from the AIR CFG.
2. `structured_plan` admits forests expressible as Vulkan structured control flow.
3. Admitted functions emit `OpLoopMerge` / `OpSelectionMerge`; phi incoming materializations belong
   to predecessor edges and are inserted before their merge/terminator instructions.
4. Rejected bounded CFGs use typed construct-tree ownership, regional dispatch, or a whole-CFG
   dispatcher where their scalar state is representable.
5. Source ownership rejections are recorded in the sidecar. After interface lowering and static CFG
   pruning, the owned module also checks conditional/switch merge ownership and dominance
   backedges. Only affected functions are reconstructed before assembly.
6. Reconstruction tries nesting first. `native::reloop_nest` derives the relooper shape tree for a
   reducible function and writes it as genuinely nested loop and selection constructs, keeping
   ordinary values in registers; an edge that has to leave more than one construct is staged
   through a flow variable read by a dispatch at each construct's merge. Whatever it declines
   stays on the bounded state-machine relooper.

Nesting keeps the source CFG's paths but not necessarily its dominance: a staged edge reaches its
destination from a merge dispatch rather than from the block that left, so a definition inside the
construct can stop reaching a use beyond it. The state machine repaired that incidentally, by
demoting every crossing value. The nesting pass therefore checks the function it emitted against
the same value-flow contract the owned module is held to and discards a nesting that fails it.

The whole-function state-machine representation has a hard block ceiling because a module can be
SPIR-V-valid yet pathological for a driver compiler: every block becomes a sibling switch case, so
every crossing value is demoted to a function-scope variable whose live range spans one loop
containing the program. That is the shape nesting exists to avoid. Exceeding that construction
ceiling remains a visible failure; validation success is not used to waive it.

Diagnose planner admission with `METAL2VULKAN_WHY=1`. Diagnostic environment variables may report
facts, but product representation selection is not environment-gated.

## Layout and interfaces

Sanitization preserves `target datalayout`. Vector ABI alignment, allocation-size advancement, and
successfully mapped `air.struct_type_info` offsets flow through the sidecar and are shared by
decorations and byte-address walkers.

The default descriptor ABI ranges are owned by `reflect`: buffers `0..32`, sampled textures
`32..160`, samplers `160..192`, colors `192..200`, and storage textures `480..608`, all in set 0.
Resource lowering and reflection consume the same checked contract. See [REFLECTION.md](REFLECTION.md).

Kernel interface lowering also owns the dispatch-grid contract. Whole-workgroup dispatch derives
`threads_per_grid` from `NumWorkgroups * LocalSize`; exact dispatch uses a fixed or dynamic grid and
partitions partial workgroups without divergent early returns around barriers.

Metal vertex entries with `air.patch` metadata become tessellation-evaluation entries. Patch domain,
control-point count, interface roles, and array extents come from AIR structure and metadata, never
shader names.

## Reflection

Reflected translation shares one parsed `StageMeta` with emission and passes. After the sole module
has validated, read-only analysis derives conservative buffer footprints from the returned bytes.
The reflected and non-reflected entry points return identical SPIR-V bytes for identical options.

## Verification

Match evidence to the claim. The complete developer ladder is in [VALIDATION.md](VALIDATION.md).

| Check | Catches |
|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | warnings and common defects |
| serial product and validation tests | construction and tooling regressions |
| primary-validity tests | violations escaping owned construction |
| byte A/B | unintended output drift |
| authored Metal/Vulkan execution | semantic disagreement beyond SPIR-V validity |
| bounded release-mode corpus translation | time, memory, and unsupported-shape coverage |

`spirv-val` validity is necessary but not sufficient for semantic correctness. A type rewrite that
only silences the validator is unsound. Construction decisions must follow IR types, storage
classes, CFG ownership, and stable AIR/LLVM ABI structure; unknown semantics remain `FALLBACK`.
