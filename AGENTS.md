# AGENTS.md

## Project Purpose

This repository is a Rust workspace for a staged research and engineering effort around a Halo2-based wrapper that may eventually verify Groth16 BN254 proofs inside an outer Halo2 proof system.

The project is intentionally incremental. The current codebase now includes a circuit-backed BN254 primitive layer covering Week 1 foundations, the narrow Week 2 slices, the Week 3 extension-field slice, the Week 4 pairing-core slice through real optimal-ate Miller traversal, final exponentiation, and a narrow multi-pairing product check, the Week 5 Groth16 BN254 verifier slice on top of that pairing core, and the direct Halo2/Midnight outer setup/prove/verify lane built on the canonical `OuterWrapperCircuit`. It is still far from a broad or production-ready wrapper verifier.

## Current Phase and Scope Boundaries

Current phase: Stage 1 / Week 5+ direct outer setup/prove/verify lane.

Implemented in scope today:

- Workspace structure, crate boundaries, docs, CLI, CI, and benchmark conventions
- BN254 foreign-field support in `wrapper-circuits` backed by `midnight-circuits` / `midnight-proofs`
- Circuit-backed `fp add`, `fp mul`, and related minimal field wiring
- Circuit-backed BN254 `fp2` support represented as `a + bu` with `u^2 = -1`
- `AssignedFp2` over two `AssignedFp` coordinates with `new`, assignment, `zero`, `one`, `add`, `sub`, `neg`, `mul`, `square`, and equality helpers
- Circuit-backed BN254 `fp6` support represented as `c0 + c1 * v + c2 * v^2`
- `AssignedFp6` over three `AssignedFp2` coordinates with `new`, assignment, `zero`, `one`, `add`, `sub`, `neg`, `mul`, `square`, and equality helpers
- Circuit-backed BN254 `fp12` support represented as `c0 + c1 * w`
- `AssignedFp12` over two `AssignedFp6` coordinates with `new`, assignment, `zero`, `one`, `add`, `sub`, `neg`, `mul`, `square`, and equality helpers
- Shared internal field/circuit traits in `wrapper-circuits/src/bn254/traits.rs`
- Shared host-side constant/reference arithmetic in `wrapper-circuits/src/bn254/host/`
- `AssignedFieldExt` now captures the common `zero` / `one` / `add` / `sub` / `neg` / equality surface across `AssignedFp`, `AssignedFp2`, `AssignedFp6`, and `AssignedFp12`
- `AssignedCircuitValue` plus shared unary/binary synthesize helpers now back the small `Fp2*Circuit`, `Fp6*Circuit`, and `Fp12*Circuit` wrappers
- Host-side reference formulas and arkworks/Midnight conversion helpers are centralized in `wrapper-circuits/src/bn254/tests/support.rs`
- Minimal BN254 G1 support backed by Midnight foreign ECC chips
- Circuit-backed G1 addition
- Coordinate-to-point construction with on-curve enforcement
- Minimal BN254 G2 affine support backed by `AssignedFp2`
- `AssignedG2Affine` with assignment, `neg`, `assert_equal`, and explicit twist `assert_on_curve`
- Narrow BN254 G2 projective support in Jacobian coordinates over `AssignedFp2`
- `AssignedG2Projective` with reserved identity encoding plus `from_affine`, `neg`, `double`, and incomplete `add`
- Miller-path BN254 G2 step support in homogeneous projective coordinates over `AssignedFp2`
- `AssignedG2MillerPoint` with non-identity `from_affine`, `double_with_line`, and `mixed_add_with_line`
- Miller-ready sparse line coefficients via `AssignedG2LineCoeffs = (ell_0, ell_w, ell_vw)`
- `AssignedMillerAccumulator` is now the public consumption boundary for line coefficients, with `mul_by_line(...)`
- sparse line evaluation into `Fp12` is now an internal accumulator detail rather than a public `AssignedG2LineCoeffs` API
- real BN254 optimal-ate Miller traversal shape backed by a fixed deterministic prepared schedule
- narrow BN254 final exponentiation over Miller-loop output, aligned with arkworks on supported non-exceptional single-pair inputs
- narrow multi-pairing product check that multiplies Miller outputs first, applies exactly one shared final exponentiation, and compares the result against the target-group identity
- narrow end-to-end pairing-core correctness against arkworks on supported non-exceptional 1-term, 2-term, and 3-term products
- narrow Groth16 BN254 verifier types in `wrapper-circuits/src/groth16.rs`
- verifier-only BN254 G1 IC accumulation using fixed public-input scalars over the existing Midnight G1 chip
- real snarkjs Groth16 BN254 JSON parsing in `wrapper-backends/src/snarkjs.rs`
- generic snarkjs Groth16 BN254 artifact-set loading in `wrapper-backends/src/groth16.rs`
- outer proof backend contracts in `wrapper-backends/src/outer.rs`
- a real Halo2/Midnight outer wrapper circuit in `wrapper-circuits/src/outer/`
- domain-level wrapper-job planning in `wrapper-core/src/job.rs`
- serializable wrapper execution packages in `wrapper-core/src/package.rs`
- expected honest direct outer artifact modeling in `wrapper-core/src/output.rs`
- execution result modeling in `wrapper-core/src/execution.rs`
- verifier-equation reduction to one multi-pairing product check using `e(A, B) * e(-alpha, beta) * e(-vk_x, gamma) * e(-C, delta) = 1`
- a real Circom/snarkjs fixture under `crates/wrapper-tests/fixtures/groth16/circom_multiplier2/`
- a real Semaphore Groth16 BN254 fixture under `crates/wrapper-tests/fixtures/groth16/semaphore/`
- end-to-end valid / invalid Groth16 verifier tests on top of the existing pairing core
- bundle -> job -> package plus direct setup/prove/verify validation on top of the Semaphore fixture
- Real layout and row visibility through the Halo2/Midnight cost model
- Deterministic arkworks-backed tests for `Fp`, `Fp2`, `Fp6`, `Fp12`, G1, and the current narrow G2 affine / Jacobian / Miller-step behavior
- Criterion sanity benchmarks for the currently implemented primitive circuits
- a canonical primitive registry in `wrapper-circuits/src/planning.rs` now drives measured primitive metadata for CLI reporting and benchmark-info output
- a single authoritative BN254 primitive path in `wrapper-circuits/src/bn254/`

Out of scope right now:

- G2 subgroup checks
- scalar multiplication on G2
- broad public full-pairing or multi-pairing APIs beyond the narrow pairing-check boundary
- broad Groth16 verifier frameworks beyond the first narrow BN254 slice
- MSM as a public supported layer
- wrapper verifier circuit composition
- production optimization of layout/cost beyond the narrow implemented sanity circuits

Do not treat the current code as a full verifier foundation. It is a deliberately narrow primitive-plus-first-verifier slice.

Week 5 verifier-memory notes:

- the committed real fixture lives under `crates/wrapper-tests/fixtures/groth16/circom_multiplier2/`
- it comes from `circom` + `snarkjs` and keeps the raw `proof.json`, `public.json`, and `verification_key.json` artifacts in the snarkjs `bn128` format
- snarkjs G1 points in that fixture use projective `[x, y, z]`; the parser accepts affine `z = 1` plus the snarkjs G1 identity encoding `[0, 1, 0]`
- the current Groth16 pairing reduction is `e(A, B) * e(-alpha, beta) * e(-vk_x, gamma) * e(-C, delta) = 1`
- the current IC accumulation path is verifier-only and uses fixed public-input scalars over the existing Midnight G1 chip; it is not a broad public MSM API
- the current generic artifact-set path is `snarkjs artifacts -> Groth16Bn254ArtifactBundle -> WrapperJob -> WrapperExecutionPackage -> WrapperExecutionResult`
- the current delivery lane is `Groth16Bn254 -> Halo2Outer` over the canonical Halo2/Midnight outer circuit
- the current expected outer artifact model is honest to the direct backend: `halo2-plonkish` / `bn254`, with `serde` JSON carrying hex-encoded proof, VK, and verifier-param payloads
- the current execution model includes both the legacy stub result and the real direct CLI execution result payload
- the current CLI surfaces for that lane are `inspect-groth16-bundle`, `plan-wrapper-job`, `export-wrapper-job`, `export-wrapper-package`, `execute-wrapper-stub`, and `execute-wrapper-direct`
- the current placeholder outer backend is `PlannedHalo2OuterBackend`, which materializes the honest direct-output contract without generating a real proof
- the selected concrete outer backend lane is `MidnightDirectOuterBackend`, and it treats the Halo2/Midnight outer circuit as canonical
- the current outer backend lane can now adapt artifact bundles into the canonical outer circuit input, build a real outer circuit, plan setup/proof outputs, and validate produced proof/VK shapes
- the repository now also exposes a direct canonical outer-circuit backend surface in `wrapper-backends/src/outer.rs` through `CanonicalOuterCircuitProofBackend`, `plan_direct_outer_circuit_setup(...)`, and `plan_direct_outer_circuit_proof(...)`
- the repository now also contains a canonical R1CS line under `crates/wrapper-circuits/src/r1cs/`, including deterministic lowering, canonical identity hashing, a zkInterface-style export bridge, and a first Arkworks adapter
- that canonical R1CS line should currently be treated as an alternate backend / later phase, not the critical path for the first real `.circom` -> outer-wrapper flow
- the remaining blockers are mainly performance/CI ergonomics and broader production hardening, not missing direct setup/prove/verify plumbing
- the prover-strategy design pass for that blocker lives in `docs/outer-prover-strategy-plan.md`

## Quick Context Routes

Choose the shortest route that matches the task instead of reading the whole
repo every time.

If you need the current truth fast:

1. `README.md`
2. `AGENTS.md` up through `Fast Context Load`
3. `docs/architecture.md`

If you need Groth16 verifier context:

1. `crates/wrapper-circuits/src/groth16.rs`
2. `crates/wrapper-backends/src/snarkjs.rs`
3. `crates/wrapper-tests/fixtures/groth16/circom_multiplier2/README.md`
4. `crates/wrapper-circuits/src/groth16/profiling.rs`

If you need wrapper planning / execution-package context:

1. `crates/wrapper-backends/src/groth16.rs`
2. `crates/wrapper-core/src/job.rs`
3. `crates/wrapper-core/src/package.rs`
4. `crates/wrapper-core/src/output.rs`
5. `crates/wrapper-core/src/execution.rs`
6. `crates/wrapper-cli/src/main.rs`

If you need the current Semaphore migration fixture context:

1. `crates/wrapper-tests/fixtures/groth16/semaphore/README.md`
2. `crates/wrapper-tests/src/lib.rs`
3. `crates/wrapper-backends/src/groth16.rs`
4. `crates/wrapper-cli/src/main.rs`

If you need the ZK Email integration study context:

1. `docs/zk-email-integration-plan.md`
2. `crates/wrapper-tests/fixtures/groth16/semaphore/README.md`
3. `crates/wrapper-tests/src/lib.rs`
4. `crates/wrapper-backends/src/groth16.rs`
5. `docs/plutus-aiken-integration-plan.md`

If you need the remaining path to a real `.circom` end-to-end wrapper flow:

1. `docs/outer-prover-strategy-plan.md`
2. `docs/real-circom-wrapper-integration-plan.md`
3. `docs/r1cs-backend-status.md`
4. `docs/decisions/0003-direct-outer-setup-cost-reduction.md`
5. `crates/wrapper-backends/src/outer.rs`
6. `crates/wrapper-circuits/src/outer/mod.rs`
7. `crates/wrapper-core/src/output.rs`
8. `crates/wrapper-core/src/execution.rs`

If you need outer wrapper circuit context:

1. `crates/wrapper-circuits/src/outer/mod.rs`
2. `crates/wrapper-circuits/src/outer/input.rs`
3. `crates/wrapper-circuits/src/outer/statement.rs`
4. `crates/wrapper-backends/src/outer.rs`
5. `docs/outer-prover-strategy-plan.md`

If you need pairing-core / final-exponentiation context:

1. `crates/wrapper-circuits/src/bn254/g2/miller.rs`
2. `crates/wrapper-circuits/src/bn254/host/pairing_host.rs`
3. `crates/wrapper-circuits/src/bn254/tests/pairing.rs`
4. `docs/midnight-local-optimization-notes.md`
5. `docs/profiling.md`
6. `docs/midnight-local-optimization-notes.md`

If you need Midnight-local optimization context:

1. `docs/midnight-local-optimization-notes.md`
2. `docs/decisions/0002-bn254-local-optimization-policy.md`
3. `docs/plans/0005-halo2-row-optimization-plan.md`
4. `crates/wrapper-circuits/src/bn254/types.rs`
5. `crates/wrapper-circuits/src/bn254/fp2.rs`
6. `crates/wrapper-circuits/src/bn254/fp6.rs`
7. `crates/wrapper-circuits/src/bn254/g2/miller.rs`
8. `docs/profiling.md`
9. `docs/plans/0002-cyclotomic-unitary-kernel-design.md`

If you need BN254 primitive structure / ownership context:

1. `crates/wrapper-circuits/src/bn254/mod.rs`
2. `crates/wrapper-circuits/src/bn254/traits.rs`
3. `crates/wrapper-circuits/src/bn254/host/mod.rs`
4. `docs/architecture.md`

If you need CLI / measurement context:

1. `crates/wrapper-circuits/src/planning.rs`
2. `crates/wrapper-circuits/src/groth16/profiling.rs`
3. `crates/wrapper-cli/src/main.rs`
4. `docs/profiling.md`
5. `docs/benchmarking.md`
6. `docs/midnight-local-optimization-notes.md`

If you need stage boundaries / "is this in scope?" context:

1. `AGENTS.md` `Current Phase and Scope Boundaries`
2. `docs/roadmap.md`
3. `docs/architecture.md`

## Fast Context Load

When you need to build context quickly, read in this order:

1. `crates/wrapper-circuits/src/groth16.rs`
2. `crates/wrapper-backends/src/snarkjs.rs`
3. `crates/wrapper-tests/fixtures/groth16/circom_multiplier2/README.md`
4. `crates/wrapper-circuits/src/bn254/mod.rs`
5. `crates/wrapper-circuits/src/bn254/traits.rs`
6. `crates/wrapper-circuits/src/bn254/host/mod.rs`
7. `crates/wrapper-circuits/src/bn254/fp2.rs`, `fp6.rs`, `fp12.rs`
8. `crates/wrapper-circuits/src/bn254/g2/mod.rs`
9. `crates/wrapper-circuits/src/bn254/g2/affine.rs`
10. `crates/wrapper-circuits/src/bn254/g2/jacobian.rs`
11. `crates/wrapper-circuits/src/bn254/g2/miller.rs`
12. `crates/wrapper-circuits/src/bn254/host/pairing_host.rs`
13. `crates/wrapper-circuits/src/bn254/tests/support.rs`
14. `crates/wrapper-circuits/src/bn254/tests/pairing.rs`
15. `crates/wrapper-circuits/src/groth16/profiling.rs`
16. `crates/wrapper-circuits/src/outer/mod.rs`
17. `crates/wrapper-backends/src/groth16.rs`
18. `crates/wrapper-backends/src/outer.rs`
19. `crates/wrapper-core/src/job.rs`, `package.rs`, `output.rs`, `execution.rs`
20. `crates/wrapper-circuits/src/planning.rs`, `crates/wrapper-cli/src/main.rs`
21. `docs/outer-prover-strategy-plan.md`
22. `docs/profiling.md`
23. `docs/midnight-local-optimization-notes.md`

This is the highest-signal order for understanding the current primitive surface, reusable helpers, and measured costs.

## Document Roles

Use each top-level doc for one job:

- `README.md`: fastest repo snapshot, workspace map, contributor commands, and entry points
- `AGENTS.md`: binding scope, architectural boundaries, staged constraints, and code-touching rules
- `docs/architecture.md`: crate ownership, data flow, and primitive-layer boundaries
- `docs/roadmap.md`: what stage the repo is in and what remains explicitly out of scope
- `docs/profiling.md`: how to measure layout cost and compare optimization baselines
- `docs/benchmarking.md`: benchmark naming, bench-info wiring, and benchmark/reporting sync rules
- `docs/plans/0005-halo2-row-optimization-plan.md`: ordered implementation plan for row-count optimization work on the current BN254 pairing-core lane, with a ready-to-start Phase 1
- `docs/midnight-local-optimization-notes.md`: prioritized Midnight primitives and local optimization candidates that already proved useful or look promising for the BN254 tower / pairing path
- `docs/decisions/0002-bn254-local-optimization-policy.md`: durable retained/rejected optimization decisions for the BN254 pairing-core lane
- `docs/decisions/0003-direct-outer-setup-cost-reduction.md`: accepted direction for reducing direct outer setup cost via a lean setup artifact and later params caching
- `docs/decisions/0004-local-midnight-proofs-patch.md`: accepted rationale for carrying a local `midnight-proofs` patch to support richer direct setup/prove artifacts
- `docs/cyclotomic-unitary-kernel-design.md`: proposed compressed-torus-region design for repeated `cyclotomic * unitary_inverse(cyclotomic)` sites in the hard part
- `docs/zk-email-integration-plan.md`: phased plan for the first larger Circom-origin integration track using ZK Email as the reference case
- `docs/real-circom-wrapper-integration-plan.md`: phased implementation plan for finishing the real `.circom` -> outer-wrapper end-to-end path
- `docs/r1cs-backend-status.md`: current status of the canonical R1CS line and why it is currently an alternate backend / later phase
- `docs/outer-prover-strategy-plan.md`: current proving-strategy decision and direct backend surface for the canonical Halo2/Midnight outer circuit

When adding a new major doc, update this list and at least one context route so
future agents know when to read it.

## Repository Map

- `crates/wrapper-core`: domain models, traits, config, errors, metadata, capability/status reporting, wrapper-job planning, execution packages, expected output-artifact shapes, and execution results
- `crates/wrapper-circuits`: Halo2-facing code, Midnight-backed BN254 primitive layer, planning, layout reporting
- `crates/wrapper-backends`: backend adapter placeholders, artifact parsing entry points, generic Groth16 artifact-set loading, and bundle-to-wrapper planning adapters
- `crates/wrapper-cli`: developer commands and diagnostics
- `crates/wrapper-tests`: shared fixtures, benchmark entry points, and integration helpers
- `docs/architecture.md`: intended layering and current primitive boundaries
- `docs/roadmap.md`: staged implementation plan
- `docs/benchmarking.md`: benchmark structure and conventions
- `docs/profiling.md`: reproducible layout-profiling workflow for the current Groth16 slice
- `docs/midnight-local-optimization-notes.md`: local Midnight-backed optimization guidance for repeated tower operations and fixed-constant arithmetic
- `docs/outer-prover-strategy-plan.md`: strategy document for the remaining prover/backend decision on the outer Halo2/Midnight circuit
- `docs/decisions/0001-initial-workspace-structure.md`: ADR for the workspace split
- `docs/decisions/0002-bn254-local-optimization-policy.md`: ADR for retained versus rejected BN254 local pairing-core optimizations

## Crate Responsibilities

`wrapper-core`

- Must remain mostly domain-oriented.
- Prefer no Halo2 dependency unless a boundary cannot be expressed otherwise.
- Own shared enums, traits, config structs, metadata, capabilities, and stable public concepts.
- Own wrapper-job planning, execution-package, expected-output, and execution-result concepts when they can stay proving-system-agnostic.
- Must not absorb chip-specific or region-specific logic.

`wrapper-circuits`

- Own Halo2-facing code, Midnight integration, circuit planning, and primitive gadget boundaries.
- Own the canonical outer wrapper circuit under `src/outer/`; backend crates may adapt inputs into that circuit but must not define a second competing circuit source of truth.
- Currently owns the BN254 `AssignedFp`, `AssignedFp2`, `AssignedFp6`, `AssignedFp12`, `AssignedG1`, `AssignedG2Affine`, narrow `AssignedG2Projective`, Miller-path `AssignedG2MillerPoint`, `AssignedG2LineCoeffs`, and `AssignedMillerAccumulator` circuit-backed layer.
- Keeps the active BN254 primitive implementation under `src/bn254/`, split by concern instead of one monolithic file.
- The current `g2/` subtree is split by model:
  `g2/affine.rs`, `g2/jacobian.rs`, `g2/miller.rs`, with `g2/mod.rs` holding shared aliases, constants, helpers, and re-exports.
- The current host-side support is split by concern under `bn254/host/`:
  `host/mod.rs` for the shared tower surface,
  `host/g2_host.rs` for G2/Jacobian/Miller host constants,
  `host/pairing_host.rs` for final-exponentiation host formulas.
- Reuse `bn254/host/` before duplicating tuple-based host/reference arithmetic across `fp2.rs`, `fp6.rs`, `fp12.rs`, or `g2/mod.rs`.
- Reuse `bn254/traits.rs` before adding more tiny wrapper-specific circuit boilerplate in `fp2.rs`, `fp6.rs`, or `fp12.rs`.
- The current BN254 test tree is split by concern under `bn254/tests/`:
  `tests/mod.rs` as the root,
  `tests/support.rs` for shared arkworks/Midnight helpers and test fixtures,
  `tests/field_and_tower.rs` for field/Fp2/Fp6/Fp12 coverage,
  `tests/curve.rs` for G1/G2/projective/line-extraction coverage,
  `tests/accumulator.rs` for accumulator/sparse-line/mixed-add-consumption coverage,
  `tests/pairing.rs` for the pairing-core lane.
- Reuse `bn254/tests/support.rs` before adding new arkworks/Midnight conversion helpers or duplicating host-side reference formulas in test modules.
- Keep expensive pairing-core assertions in `tests/pairing.rs` and cheaper primitive/G2 coverage in `tests/field_and_tower.rs`, `tests/curve.rs`, and `tests/accumulator.rs` so the slow lane remains explicit.
- Prefer short public methods over formula-heavy bodies: keep APIs as orchestration layers and move algebraic steps into internal helpers with explicit names.
- Should depend on `wrapper-core`.
- Must not absorb artifact parsing or backend-specific concerns.
- Keep dead compatibility shims and obsolete host-side leftovers out of the crate.

`wrapper-backends`

- Own parsing, loaders, serialization adapters, and future ecosystem bridges.
- Should depend on `wrapper-core`.
- Must not define circuit semantics independently of `wrapper-circuits`.
- It now owns the generic `ArtifactSetLoader` contract plus the current `snarkjs` Groth16 BN254 bundle loader.
- It may adapt parsed bundles into domain-level wrapper jobs/packages, but should not absorb application-specific public-input naming.

`wrapper-cli`

- Own user-facing commands, output formatting, and developer diagnostics.
- Must report missing functionality honestly.
- Should expose measured primitive status without overstating what is implemented.
- The current narrow optimization-baseline surface is `profile-layout`, which emits TSV layout metrics for Groth16, pairing-term scaling, public-input scaling, and existing pairing-core blocks.
- Treat `profile-layout` as layout/constraint profiling, not runtime benchmarking.
- The current planning/execution surfaces for wrapper experiments are `inspect-groth16-bundle`, `plan-wrapper-job`, `export-wrapper-job`, `export-wrapper-package`, `execute-wrapper-stub`, `execute-wrapper-direct`, `execute-wrapper-direct-setup`, `execute-wrapper-direct-prove`, `execute-wrapper-direct-prove-trace`, `execute-wrapper-direct-prove-finalize`, and `execute-wrapper-direct-verify`.
- Direct execution commands now enforce a `24 GiB` process memory limit.
- The direct setup artifact now persists verification materials plus a proving-key sidecar.
- The repo now carries a local `[patch.crates-io]` override for `midnight-proofs` under `patches/midnight-proofs`.
- That local patch adds `BaseProvingKey`, `keygen_pk_base(...)`, `create_proof_from_base(...)`, `create_proof_trace_from_base(...)`, and `finalise_proof_from_base_trace(...)` so richer direct setup/prove artifacts can be expressed without waiting on upstream.
- The direct prove path now avoids rerunning `keygen_pk(...)` in the wrapper backend.
- Current known limitation: the next suspected memory hotspot is eager coset materialization inside `compute_h_poly(...)` (`advice_cosets` / `instance_cosets`) in the patched prover.
- The split `execute-wrapper-direct-prove-trace` / `execute-wrapper-direct-prove-finalize` flow exists so the pre-`compute_h_poly(...)` phase can be cached as an artifact between experiments, but it should currently be treated as broken.
- Latest valid split failure to remember in future conversations: `execute-wrapper-direct-prove-trace` fails with `midnight create_proof_trace_from_base failed: The constraint system is not satisfied`, and the last reliable backend log line is `prove-trace: entering create_proof_trace_from_base`.
- One successful measured lean setup run on `circom_multiplier2` produced `circuit_k = 21`, `public_input_count = 1`, and `setup_elapsed_ms = 1554572` (about `25m 54s`).

`wrapper-tests`

- Own fixtures, shared test helpers, and benchmark entry points.
- Should host cross-crate integration coverage and Criterion runners.
- Should not become a dumping ground for reusable circuit logic that belongs in `wrapper-circuits`.

## Rules for Architectural Changes

- Preserve the separation between domain, circuits, and backends unless there is a documented reason to change it.
- Update `docs/architecture.md` and the relevant ADR when changing public architecture or ownership boundaries.
- Prefer narrow interfaces in `wrapper-core` over leaking circuit implementation details across crates.
- Keep the current primitive layer small and explicit.
- Do not introduce speculative abstractions for later pairing/verifier work unless they are needed immediately.

## Rules for Adding Dependencies

- Add dependencies conservatively.
- Prefer workspace-managed dependency versions.
- Heavy cryptographic dependencies must earn their place through current-stage use, tests, and CI viability.
- Document why a new dependency belongs now, not later.
- Feature-gate stage-specific dependencies when appropriate.
- For current circuit work, prefer existing Midnight and arkworks infrastructure over inventing parallel stacks.

## Rules for Implementing Cryptographic Code

- Start from documented interfaces and roadmap items.
- Add tests and docs alongside any cryptographic implementation.
- Keep arithmetic, ECC, pairing, and verifier logic explicit and reviewable.
- Do not hide critical behavior behind abstractions that obscure invariants or cost.
- Record major cryptographic architecture choices in `docs/decisions/`.
- Treat retained or rejected local pairing-core optimization directions as ADR-worthy once they materially change the measured baseline or future search space.
- Prefer extending the existing Midnight-backed BN254 layer over creating a second incompatible primitive stack.

## Specific Current Primitive Guidance

The current primitive layer is built around:

- `midnight-circuits`
- `midnight-proofs`
- `midnight-curves`
- arkworks as the reference implementation in tests

When touching the current BN254 primitive code:

- keep `fp` work limited to the currently supported primitive surface unless the roadmap explicitly expands it
- keep `fp2` work aligned with the current representation `Fq2(c0, c1)` and `u^2 = -1`
- keep `fp6` work aligned with the current representation `Fq6(c0, c1, c2)` and `v^3 = 9 + u`
- keep `fp12` work aligned with the current representation `Fq12(c0, c1)` and `w^2 = v`
- keep extension-field wrapper circuits aligned with the shared `AssignedCircuitValue` synthesize helpers unless there is a clear reason not to
- keep G1 work limited to the currently supported primitive surface unless the roadmap explicitly expands it
- keep G2 work limited to the currently supported affine plus narrow Jacobian projective surface unless the roadmap explicitly expands it
- keep Miller-path G2 work aligned with the homogeneous prepared-step formulas used by arkworks BN prepared-G2 generation
- keep final exponentiation work aligned with the standard BN easy-part / hard-part decomposition used by arkworks unless a measured circuit-oriented rewrite clearly improves the current slice
- keep pairing-check work verifier-shaped: accumulate Miller outputs first, apply exactly one final exponentiation to the total product, and avoid per-term final exponentiation
- for final-exponentiation and local tower optimization work, read `docs/midnight-local-optimization-notes.md` first so you inherit the current proved-useful Midnight primitives and ruled-out local paths
- when looking for local tower wins, read `docs/midnight-local-optimization-notes.md` before inventing new gadgets; it records which `midnight-circuits` primitives (`mul_by_constant`, `linear_combination`, `add_constant`, etc.) already paid off in this repo, which ones were explicitly tried and ruled out, and which wins are only local instead of pairing-core-wide
- when a public method contains a full algebraic step, prefer extracting the formula into a well-named internal helper such as `double_step_jacobian`, `double_step_hom_projective`, or `mixed_add_step_hom_projective`
- preserve real layout measurement support
- keep benchmarks honest and tied to actually implemented circuits
- keep CLI reporting aligned with the measured state of the codebase

Concrete BN254 conventions already in use:

- `AssignedFp2` follows the standard BN254 extension representation `a + bu`
- `Fq2` coordinate order is `(c0, c1)` to match arkworks
- `u^2 = -1`
- `AssignedFp6` follows `c0 + c1 * v + c2 * v^2`
- `Fq6` coordinate order is `(c0, c1, c2)` to match arkworks
- the cubic nonresidue is `9 + u`, so `Fp6 = Fp2[v] / (v^3 - (9 + u))`
- `AssignedFp12` follows `c0 + c1 * w`
- `Fq12` coordinate order is `(c0, c1)` to match arkworks
- the quadratic nonresidue is `v = Fp6(0, 1, 0)`, so `Fp12 = Fp6[w] / (w^2 - v)`
- Miller-path G2 line coefficients use the sparse BN254 D-twist layout `(ell_0, ell_w, ell_vw)`
- evaluating those coefficients at a G1 affine point `(x_P, y_P)` yields
  `ell_0 * y_P + ell_w * x_P * w + ell_vw * v * w`
- that sparse embedding maps directly into Fp12 slots `(c0, c3, c4)` for the later `mul_by_034`-style Miller accumulator path
- the public boundary for that consumption is `AssignedMillerAccumulator::mul_by_line(...)`, not a direct public helper on `AssignedFp12`
- Miller-path `double_with_line` and `mixed_add_with_line` follow the homogeneous-projective BN prepared-G2 formulas used by arkworks / Midnight, not the Jacobian formulas used by `AssignedG2Projective`
- final exponentiation follows the standard BN254 easy-part / hard-part split used by arkworks over the Miller-loop output
- the narrow pairing-check path computes each real Miller loop, multiplies the Miller outputs in `Fp12`, applies exactly one final exponentiation, and checks equality with the `Fp12` multiplicative identity
- the current Groth16 verifier route precomputes Miller-step line coefficients off-circuit for constant verifier-key G2 terms (`beta_g2`, `gamma_g2`, `delta_g2`) and feeds those prepared lines into the interleaved multi-Miller loop; only the proof term stays on the variable G2 path
- the current final-exponentiation code now exposes `final_exponentiation_easy_part(...)` and `final_exponentiation_hard_part(...)` as audit-friendly internal helpers without changing semantics
- the current hard-part hotspot is still the repeated `exp_by_neg_x(...)` lane; read `docs/profiling.md` plus `docs/midnight-local-optimization-notes.md` before changing it so you inherit the current measured state and the local Midnight primitives that already paid off
- the current best class of local wins came from replacing generic constant multiplies in repeated tower helpers with Midnight-backed `mul_by_constant(...)`; check `docs/midnight-local-optimization-notes.md` before changing repeated `Fp2` / `Fp6` / `Fp12` transforms
- the current repo evidence says foreign-field `linear_combination(...)` is not an automatic optimization win: an April 27, 2026 pass that rewrote `AssignedFp2::mul_by_constant(...)`, `AssignedFp6::mul_by_nonresidue_fp2(...)`, and the Fp12 `3t +/- 2z` helpers regressed `fp12 cyclotomic square` (`1622 -> 1886`), `final exponentiation` (`587420 -> 678119`), and `pairing check` (`1682524 -> 1805233`), so treat that exact rewrite family as ruled out unless you have a materially different constraint shape
- if you revisit `linear_combination(...)` in the BN254 tower, compare against the retained `mul_by_constant(...)` baseline and do not keep the rewrite unless `wrapper-cli doctor` or `profile-layout --family blocks` shows a clear row win
- the current repo evidence says `add_constant(...)` does have one retained win: folding the fixed BN254 twist coefficient directly into `AssignedG2Affine::assert_on_curve(...)` improved `g2 on_curve` (`400 -> 378`), `g2 neg` (`930 -> 886`), `g2 proj from_affine` (`970 -> 948`), `g2 proj double` (`2594 -> 2550`), `g2 proj add` (`4582 -> 4516`), `g2 double_with_line` (`2698 -> 2654`), and `g2 mixed_add_with_line` (`3374 -> 3330`)
- treat that `add_constant(...)` result as a local G2 / Miller-prep win, not as proof that pairing-core blocks will move; `miller loop`, `final exponentiation`, and `pairing check` rows stayed unchanged in `profile-layout --family blocks`
- the current repo evidence says the obvious `select` / `is_equal*` / `is_zero` cleanup on the final GT identity check is row-neutral: an April 27, 2026 pass that wrapped the manual coordinate checks into composite `Fp2` / `Fp6` / `Fp12` boolean equality helpers left both `wrapper-cli doctor` and `profile-layout --family blocks` unchanged, so do not keep that rewrite for performance alone
- the current retained `exp_by_neg_x(...)` chain is now a signed-window schedule from `35` with steps `<<6,-35`, `<<9,+101`, `<<8,-83`, `<<9,+37`, `<<9,+105`, `<<11,+79`, `<<5,+17`; it wins because one extra cyclotomic square in precomputation is cheaper than the saved main-chain multiplication
- the next retained subgroup-aware win after that was compressed cyclotomic squaring inside the repeated square blocks of `exp_by_neg_x(...)`; it improved `final_exponentiation_hard_part` from `561254` to `492083` and `final_exponentiation` from `574562` to `505391`
- the first torus-style prototype for `cyclotomic * unitary_inverse(cyclotomic)` was also a non-win when applied only to `y7` inside the hard part: it regressed `final_exponentiation_hard_part` from `561254` to `571604` and `final_exponentiation` from `574562` to `584912`, so do not retry isolated call-site torus substitutions
- the broad `CyclotomicFp12MulChip` rollout over `y3`, `y9`, `y10`, and `y11` was also a non-win: it regressed `final_exponentiation_hard_part` from `561254` to `561344` and `final_exponentiation` from `574562` to `574652`, so do not retry chip-level repackaging of the ambient Fp12 multiplication formula without a genuinely different subgroup arithmetic kernel
- the fixed BN254 `exp_by_neg_x(...)` recipe now lives in `crates/wrapper-circuits/src/bn254/final_exp_chain.rs` and is consumed by both host/reference code and the circuit path; keep that module canonical
- minimal G2 affine on-curve checks use the arkworks BN254 twist equation `y^2 = x^3 + b`
- the twist coefficient is `b = 3 / (u + 9)` with the exact arkworks value
  `Fq2(19485874751759354771024239261021720505790618469301721065564631296452457478373, 266929791119991161246907387137283842545076965332900288569378510910307636690)`

Current measured primitive costs from `wrapper-cli doctor`:

- `fp add`: 40 rows / 58 queries, `k=9`
- `fp mul`: 38 rows / 58 queries, `k=9`
- `fp2 add`: 80 rows / 58 queries, `k=9`
- `fp2 mul`: 152 rows / 58 queries, `k=9`
- `fp2 square`: 114 rows / 58 queries, `k=9`
- `fp6 add`: 240 rows / 58 queries, `k=9`
- `fp6 mul`: 1252 rows / 58 queries, `k=11`
- `fp6 square`: 736 rows / 58 queries, `k=10`
- `fp12 add`: 480 rows / 58 queries, `k=9`
- `fp12 mul`: 4076 rows / 58 queries, `k=12`
- `fp12 square`: 2594 rows / 58 queries, `k=12`
- `fp12 cyclotomic square`: 1622 rows / 58 queries, `k=11`
- `g1 add`: 319 rows / 105 queries, `k=9`
- `g2 on_curve`: 378 rows / 58 queries, `k=9`
- `g2 neg`: 886 rows / 58 queries, `k=10`
- `g2 proj from_affine`: 948 rows / 58 queries, `k=10`
- `g2 proj double`: 2550 rows / 58 queries, `k=12`
- `g2 proj add`: 4516 rows / 58 queries, `k=13`
- `g2 double_with_line`: 2654 rows / 58 queries, `k=12`
- `g2 mixed_add_with_line`: 3330 rows / 58 queries, `k=12`
- `miller accumulator square`: 2714 rows / 58 queries, `k=12`
- `miller accumulator mul_by_line`: 4248 rows / 58 queries, `k=13`
- `miller accumulator mul_by_line sparse`: 2592 rows / 58 queries, `k=12`
- `miller loop narrow`: 457060 rows / 58 queries, `k=19`
- `final exponentiation`: 505391 rows / 58 queries, `k=19`
- `pairing check`: 1600495 rows / 94 queries, `k=21`

Interpretation guidance:

- `g2 neg` is not a measure of a raw sign flip alone; the current benchmark circuit includes assignment, on-curve checks, negation, and equality against the expected output
- `fp12 mul` and `fp12 square` are measurements of the actual sanity circuits over the implemented tower, not optimized pairing-ready kernels
- `fp12 cyclotomic square` is a subgroup-only specialization for the final-exponentiation hard part; it must not be treated as a general Fp12 square
- `g2 double_with_line` and `g2 mixed_add_with_line` are measurements of the actual Miller-step sanity circuits, not a full Miller loop
- `miller accumulator mul_by_line` is the generic baseline path, while `miller accumulator mul_by_line sparse` is the optimized public accumulator path for the current BN254 D-twist `(ell_0, ell_w, ell_vw)` layout
- `miller loop narrow` now measures the real fixed single-pair BN254 optimal-ate Miller traversal, not the earlier synthetic schedule
- `final exponentiation` measures the narrow single-pair BN254 final-exponentiation sanity circuit over a Miller-loop output, not a verifier-facing full pairing API
- `profile-layout --family blocks` now also exposes `final exponentiation easy part` and `final exponentiation hard part`; the current measured split is `12288` rows / `k=14` for the easy part and `492083` rows / `k=19` for the hard part, so future optimization work should focus overwhelmingly on the hard part
- `docs/midnight-local-optimization-notes.md` is the canonical short list of Midnight primitives and local optimization targets; keep it updated when a new `midnight-circuits` primitive proves useful or a local candidate is ruled out
- `pairing check` should always be described as the narrow verifier-shaped product-check slice with one shared final exponentiation, not as a full pairing engine or Groth16 verifier
- as of the current repo state, local accumulator-square rewrites that only swap formulas inside the existing Fp12 tower did not beat the generic `miller accumulator square` cost; future square optimization likely needs a more structural/cross-step design rather than a small algebraic rewrite, so do not keep partial `square_optimized` experiments in the tree unless they measurably win in `wrapper-cli doctor`
- as of the current repo state, the obvious foreign-field `linear_combination(...)` replacements for short `Fp2` affine transforms are also ruled out by measurement; do not re-land that family of rewrites without fresh `doctor` / `profile-layout --family blocks` evidence
- as of the current repo state, the retained `add_constant(...)` win is specifically the fixed-twist-coefficient path inside G2 on-curve arithmetic; if a new `add_constant(...)` idea does not involve a truly fixed offset already present in the formula, be skeptical and measure it before keeping the rewrite
- as of the current repo state, the tested `select` / `is_equal*` / `is_zero` cleanup for GT-identity checking is performance-neutral; only keep a rewrite in that family if it buys clarity or unlocks later branching logic, not because it is expected to lower rows by itself
- as of the current repo state, the most promising structural local lever after the easy wins was indeed the `exp_by_neg_x(...)` chain itself; signed windows are now the retained direction, so future attempts should compare against the current signed schedule rather than the older all-positive one
- as of the current repo state, torus/compressed representations for cyclotomic-unitary products remain only a design path, not a retained optimization; the `y7`-only prototype already lost, so future work must amortize compression across a longer region or it is unlikely to win
- as of the current repo state, an explicit `CyclotomicFp12MulChip` that simply packages the current quadratic-over-`Fp6` product as a subgroup-aware gadget is also ruled out by measurement; future Halo2-side gadget work must bring genuinely different arithmetic, not just a different wrapper over the same tower operations
- as of the current repo state, compressed cyclotomic squaring *is* the retained subgroup-aware arithmetic optimization; future work should compare against that implementation before revisiting any torus or explicit-mul-chip design
- cost numbers should always be described as measurements of the actual sanity circuits, not abstract algebraic lower bounds

## Coding Standards

- Prefer explicit, readable Rust over cleverness.
- Use crate-level docs and module docs when they clarify purpose.
- Keep comments purposeful and sparse.
- Keep circuit-backed adapters thin where possible.
- Avoid duplicate primitive stacks or parallel APIs for the same concept.
- Prefer removing obsolete compatibility files once the Midnight-backed path replaces them.
- Do not leave misleading stubs that imply verifier completeness.
- Delete files that have become genuinely unused instead of keeping stale alternative paths around.

## Error Handling Standards

- Use `thiserror` for library-facing error types.
- Use `anyhow` at CLI or orchestration boundaries where context aggregation is helpful.
- Errors should state what failed, at what boundary, and whether the feature is intentionally unimplemented.
- If a circuit path is deliberately unsupported at this stage, say so explicitly instead of faking behavior.
- Do not keep custom error layers that are no longer used after an integration shift.

## Testing Standards

- Every new public behavior should have at least one test at the owning layer.
- Keep unit tests near the crate that owns the behavior.
- Use arkworks as the reference implementation for BN254 field, Fp2, G1, and minimal G2 affine sanity checks when appropriate.
- Keep randomized tests deterministic via fixed seeds unless there is a strong reason not to.
- Use `wrapper-tests` for shared fixtures, integration coverage, and benchmark entry points.
- Do not add tests that imply pairing or verifier support before those stages exist.
- Keep the default local test lane practical. Expensive pairing-core `MockProver` tests in `tests/pairing.rs` should be marked `#[ignore = "slow pairing-core"]` unless they are truly cheap smoke coverage.
- The intended split is:
  - always-run: field arithmetic, narrow G1/G2 primitives, Miller-step / accumulator tests, and cheap host-side pairing-core structure checks
  - slow pairing-core: real Miller-loop, final-exponentiation, and pairing-check `MockProver` end-to-end tests
- To run the slow pairing-core lane explicitly, use `cargo test -p wrapper-circuits -- --ignored`.

Current test expectations for the primitive layer:

- `Fp2` tests should include algebra identities, deterministic randomized add/mul/square checks, and edge-oriented real/imaginary cases
- `Fp6` tests should include algebra identities, deterministic randomized add/mul/square checks, and structured single-coordinate cases
- `Fp12` tests should include algebra identities, deterministic randomized add/mul/square checks, and structured `c0`-only / `c1`-only cases
- minimal G2 tests should include valid affine points, negative on-curve cases, negation validity, and equality behavior
- narrow G2 projective tests should stay explicit about the supported domain: `from_affine`, `neg`, `double`, incomplete `add`, and reserved identity encoding
- Miller-path G2 tests should cover `double_with_line`, `mixed_add_with_line`, sparse `Fp12` embedding, and explicitly unsupported exceptional cases such as `P = Q`
- for the current narrow Miller slice, keep a few stable fixed fixtures alongside deterministic randomized checks: generator-based `double_with_line`, generator-based `double + add`, baseline-vs-sparse `mul_by_line` cross-checks, and at least one longer deterministic prepared schedule
- explicitly keep unsupported Miller mixed-add cases documented by tests for both `P = Q` and `P = -Q`; do not silently widen support claims just because randomized tests pass
- if a test needs a host-side reference formula, put the logic in `tests/support.rs` and keep the domain files focused on cases/assertions
- if a test-local helper becomes shared across multiple test groups, move it into `tests/support.rs` in the same refactor rather than leaving partial duplicates behind

## Benchmarking Standards

- Use Criterion.
- Keep benchmark names in the `bench_<module>_<operation>` form.
- Benchmarks must reflect real implemented circuits, not aspirational future behavior.
- Do not make performance claims beyond what the current benchmark actually measures.
- When changing benchmark structure, update `docs/benchmarking.md` and `wrapper-cli bench-info`.
- For current Groth16 optimization baselines, prefer `wrapper-cli profile-layout` over ad hoc timing or new benchmark scaffolding.
- `profile-layout` output is TSV and intended to be redirected to a file for before/after diffs.
- The `groth16`, `pairing-terms`, and `all` profiling families are intentionally heavier than `blocks` and `public-inputs`; let them finish before inspecting the output file, or the TSV may appear empty/incomplete.
- The `blocks` profiling family now includes `bn254_final_exponentiation_easy_part`, `bn254_final_exponentiation_hard_part`, and total `bn254_final_exponentiation`; use those rows before changing the final-exponentiation chain.
- The current `pairing-terms` profiling family models one variable proof-like G2 term and the remaining terms as prepared constant verifier-key-style G2 terms, so it is intended as a Groth16-relevant scaling proxy rather than an all-variable pairing benchmark.

Current benchmark entry points include:

- `bench_fp_add`
- `bench_fp_mul`
- `bench_fp2_add`
- `bench_fp2_mul`
- `bench_fp2_square`
- `bench_fp6_add`
- `bench_fp6_mul`
- `bench_fp6_square`
- `bench_fp12_add`
- `bench_fp12_mul`
- `bench_fp12_square`
- `bench_g1_add`
- `bench_g2_on_curve`
- `bench_g2_neg`
- `bench_g2_proj_from_affine`
- `bench_g2_proj_double`
- `bench_g2_proj_add`
- `bench_g2_double_with_line`
- `bench_g2_mixed_add_with_line`
- `bench_miller_accumulator_square`
- `bench_miller_accumulator_mul_by_line`
- `bench_miller_accumulator_mul_by_line_sparse`
- `bench_miller_loop_narrow`

Benchmark/metrics integration rules that have already bitten this repo:

- `wrapper-cli bench-info` is derived from the canonical primitive registry in `crates/wrapper-circuits/src/planning.rs`; if a new primitive is missing from `bench-info`, fix the registry/layer wiring before touching docs text
- when adding a new measured primitive, keep `crates/wrapper-tests/benches/...`, `crates/wrapper-tests/benches/primitives.rs`, `crates/wrapper-circuits/src/planning.rs`, `wrapper-cli bench-info`, and `docs/benchmarking.md` in sync in the same turn
- use explicit honest names for Miller work such as `*_narrow`, `*_sparse`, or `*_baseline` when the slice is not a full pairing pipeline
- when changing Groth16 optimization-baseline reporting, keep `crates/wrapper-circuits/src/groth16/profiling.rs`, `crates/wrapper-cli/src/main.rs`, `docs/profiling.md`, and the relevant README/AGENTS references in sync in the same turn
- keep profiling identifiers stable: `family`, `id`, and `label` should remain diff-friendly across runs unless there is a deliberate reporting-schema change
- when changing final-exponentiation decomposition or local-Midnight optimization guidance, keep `crates/wrapper-circuits/src/bn254/g2/miller.rs`, `crates/wrapper-circuits/src/bn254/host/pairing_host.rs`, `crates/wrapper-circuits/src/bn254/metrics.rs`, `docs/profiling.md`, and `docs/midnight-local-optimization-notes.md` in sync in the same turn

## Documentation Standards

- Update the README when implemented scope or contributor workflow changes.
- Update `docs/architecture.md` when circuit boundaries or ownership changes.
- Update `docs/roadmap.md` when stage boundaries or sequencing change.
- Add or amend ADRs for architectural decisions that affect crate ownership or public interfaces.
- Be explicit about what is circuit-backed, what is reference-tested, and what is still missing.
- When cleanup removes obsolete files or paths, reflect the new simpler state in contributor docs.

When refactoring `wrapper-circuits/src/bn254/`:

- keep the public API stable through `bn254/mod.rs` re-exports when possible
- prefer splitting by concept, for example `types.rs`, `field.rs`, `fp2.rs`, `g2/mod.rs`, `g2/affine.rs`, `g2/jacobian.rs`, `g2/miller.rs`, `host/mod.rs`, `host/pairing_host.rs`, `metrics.rs`, `tests/mod.rs`, `tests/pairing.rs`
- if docs mention the primitive path, keep them pointed at `src/bn254/`, not the old deleted `src/bn254.rs`
- if primitive metadata, measured labels, or bench-info output changes, update the canonical registry in `wrapper-circuits/src/planning.rs` first and derive downstream surfaces from it
- after any structural refactor, update `AGENTS.md` in the same turn so it reflects the new module boundaries, reuse points, and context-loading order

## How to Propose a Change

1. Identify the stage and boundary the change belongs to.
2. Check whether the change fits an existing crate responsibility.
3. Update docs first when the architecture or scope changes.
4. Implement the smallest honest increment.
5. Add tests that prove the increment, not future claims.
6. Verify `cargo check`, `cargo test`, and relevant benches or CLI paths when applicable.

## What Not To Do

- Do not collapse crates for convenience.
- Do not place Halo2-specific concerns in `wrapper-core` without strong justification.
- Do not move application-specific public-input naming such as Semaphore field labels into generic backend parsing.
- Do not implement broad verifier-facing full pairings, multi-pairings beyond the narrow product-check slice, or Groth16 verifier logic unless the task explicitly asks for that stage.
- Do not jump from minimal G2 affine support to G2 arithmetic or subgroup logic unless the task explicitly asks for it.
- Do not write placeholder code that pretends proofs are verified.
- Do not add a second BN254 primitive implementation path that competes with the Midnight-backed one without a documented reason.
- Do not overclaim performance or soundness from the current sanity circuits.

## Explicit Warning for Current Tasks

For tasks in the current repository state, do not assume that because `fp add`, `fp mul`, `fp2`, minimal G1, and minimal G2 affine support exist, the project is ready for:

- generalized `Fp12` helper optimizations for pairing workloads
- pairing gadgets
- Groth16 verification
- wrapped verifier composition
- public-input verifier logic
- G2 arithmetic beyond the currently implemented narrow Jacobian `from_affine` / `neg` / `double` / incomplete `add` slice
- extending Miller-path G2 steps into a full Miller loop without a dedicated design pass
- full MSM infrastructure

Those remain future-stage work unless the task explicitly advances the roadmap.

## Preferred Incremental Workflow for Next Stages

1. Extend `wrapper-core` only with the minimal new domain concept.
2. Expand `wrapper-circuits` in the narrowest possible way around the existing Midnight-backed foundation.
3. Preserve or improve real layout visibility when adding new circuit-backed primitives.
4. Add arkworks-backed or equivalent reference tests.
5. Add or update benchmarks only for code that truly exists.
6. Document the design decision before scaling implementation breadth.
