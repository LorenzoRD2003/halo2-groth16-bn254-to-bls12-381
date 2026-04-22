# Halo2 Wrapper Workspace

This repository is a Rust workspace for a staged research and engineering effort around a Halo2-based outer proof system that may eventually verify Groth16 BN254 proofs inside a Halo2 wrapper.

The current phase is still intentionally narrow, but it is no longer just repository bootstrap: the project now includes a circuit-backed BN254 primitive layer built on `midnight-circuits` and `midnight-proofs`, together with CI, benchmarks, CLI diagnostics, and contributor documentation. Week 2 now includes a first Fp2 slice, a minimal G2 affine slice, and a narrow Jacobian-style G2 projective slice for `from_affine`, `neg`, `double`, and incomplete `add`. Week 3 now includes the extension-field slices in Fp6 and Fp12 plus Miller-oriented G2 `double_with_line` / `mixed_add_with_line` extraction. Pairings remain out of scope.

## Current Status

What the repository currently contains:

- A multi-crate Rust workspace with explicit boundaries between domain logic, circuit-facing code, backend adapters, CLI tooling, and test harness code.
- Week 1 BN254 foundations in `wrapper-circuits`, organized under `src/bn254/`: Midnight-backed foreign-field and G1 circuits with real layout measurements.
- A first Week 2 BN254 Fp2 layer in `wrapper-circuits`, also organized under `src/bn254/`, built from two circuit-backed `AssignedFp` coordinates.
- A minimal BN254 Fp6 layer in `wrapper-circuits`, built as three circuit-backed `AssignedFp2` coordinates using the arkworks BN254 tower.
- A minimal BN254 Fp12 layer in `wrapper-circuits`, built as two circuit-backed `AssignedFp6` coordinates using the same arkworks / Midnight BN254 tower.
- A minimal Week 2 BN254 G2 affine layer in `wrapper-circuits`, backed by `AssignedFp2` coordinates with circuit-backed negation and twist on-curve validation.
- A narrow Week 2 BN254 G2 projective layer in Jacobian coordinates `(X:Y:Z)` with affine embedding, negation, doubling, incomplete addition, measured costs, and arkworks-backed sanity tests.
- A Week 3 BN254 Miller-path G2 step layer with a dedicated homogeneous-projective state, `double_with_line`, `mixed_add_with_line`, and Miller-ready sparse line coefficients.
- Placeholder outer-wrapper planning and backend integration boundaries that are honest about what is still missing.
- Contributor-oriented documentation covering architecture, roadmap, and initial design decisions.
- A `wrapper-cli` binary with honest developer commands for environment inspection and configuration validation.

What is explicitly not implemented yet:

- Pairing gadgets or pairing arithmetic
- Groth16 verifier logic
- G2 subgroup checks or scalar multiplication
- Real backend adapters to arkworks, Midnight, `blst`, or `snarkjs`
- Cryptographic soundness claims of any kind

This repository now includes Week 1 arithmetic foundations, but it is still far from a Groth16 wrapper verifier.

## Planned Architecture

The intended shape of the project is:

- `wrapper-core`: domain-oriented types, traits, config, errors, metadata, and public architectural contracts
- `wrapper-circuits`: Halo2-facing circuits, current Midnight-backed BN254 primitive layer, layout descriptions, and future gadget integration points
- `wrapper-backends`: artifact loading, parser adapters, proof/VK material ingestion, and future external backend bridges
- `wrapper-cli`: developer-facing commands for validation, inspection, and future orchestration
- `wrapper-tests`: shared fixtures, helpers, and future end-to-end integration coverage

The design keeps `wrapper-core` mostly independent from Halo2 so project concepts can stay stable even as proof-system integration evolves.

## Workspace Map

```text
.
├── Cargo.toml
├── README.md
├── AGENTS.md
├── docs/
│   ├── architecture.md
│   ├── benchmarking.md
│   ├── roadmap.md
│   └── decisions/0001-initial-workspace-structure.md
└── crates/
    ├── wrapper-core/
    ├── wrapper-circuits/
    ├── wrapper-backends/
    ├── wrapper-cli/
    └── wrapper-tests/
```

## Build Instructions

Requirements:

- Rust stable toolchain
- `cargo`, `rustfmt`, and `clippy`

Commands:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps
```

## CI Status

Basic GitHub Actions CI is defined in `.github/workflows/ci.yml`.

The workflow currently runs:

- `cargo check --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo doc --no-deps`

Run the same commands locally before opening a PR.

## Benchmarking

Criterion benchmarks are scaffolded under `crates/wrapper-tests/benches/` and currently cover the implemented BN254 sanity-check circuits:

- `field/`
- `ecc/`

Run them with:

```bash
cargo bench
```

## Implemented Primitive Layer

What works now:

- BN254 foreign-field values wrapped as `AssignedFp` over Midnight `FieldChip`
- Circuit-backed `fp add` and `fp mul` in Halo2 via `midnight-circuits`
- BN254 Fp2 values wrapped as `AssignedFp2 = (c0, c1)` with `u^2 = -1`
- Circuit-backed `fp2 add`, `fp2 sub`, `fp2 neg`, `fp2 mul`, and `fp2 square`
- BN254 Fp6 values wrapped as `AssignedFp6 = (c0, c1, c2)` with `c0 + c1 * v + c2 * v^2`
- BN254 Fp6 tower matching arkworks exactly: `Fp6 = Fp2[v] / (v^3 - (9 + u))`
- Circuit-backed `fp6 add`, `fp6 sub`, `fp6 neg`, `fp6 mul`, and `fp6 square`
- BN254 Fp12 values wrapped as `AssignedFp12 = (c0, c1)` with `c0 + c1 * w`
- BN254 Fp12 tower matching arkworks exactly: `Fp12 = Fp6[w] / (w^2 - v)` with quadratic nonresidue `v = Fp6(0, 1, 0)`
- Circuit-backed `fp12 add`, `fp12 sub`, `fp12 neg`, `fp12 mul`, and `fp12 square`
- Minimal BN254 G1 support wrapped as `AssignedG1` over Midnight `ForeignEccChip`
- Circuit-backed G1 addition plus coordinate-to-point on-curve enforcement
- Minimal BN254 G2 affine support wrapped as `AssignedG2Affine = (x, y)` over `AssignedFp2`
- Circuit-backed G2 affine negation plus twist on-curve validation
- Narrow BN254 G2 Jacobian support wrapped as `AssignedG2Projective = (X, Y, Z)` with affine model `x = X / Z^2`, `y = Y / Z^3`
- Circuit-backed `from_affine`, `neg`, `double`, and incomplete Jacobian-Jacobian `add`
- Miller-path G2 state wrapped as `AssignedG2MillerPoint = (X, Y, Z)` in homogeneous projective coordinates with affine model `x = X / Z`, `y = Y / Z`
- Miller-ready BN254 line coefficients wrapped as `AssignedG2LineCoeffs = (ell_0, ell_w, ell_vw)`
- Miller-loop accumulator state wrapped as `AssignedMillerAccumulator`
- Line-coefficient layout chosen for BN254 D-twist sparse `Fp12` consumption:
  `ell_0 * y_P + ell_w * x_P * w + ell_vw * v * w`
- Circuit-backed `double_with_line` and `mixed_add_with_line` following the same homogeneous-projective prepared-G2 formulas used by arkworks / Midnight for BN prepared-G2 generation
- The public consumption boundary is now `AssignedG2LineCoeffs -> AssignedMillerAccumulator::mul_by_line(...)`
- `AssignedFp12` stays relatively clean; sparse line evaluation remains an internal detail of the accumulator for now
- Tuple-based host/reference arithmetic for the BN254 tower is centralized in `wrapper-circuits/src/bn254/host.rs`
- Deterministic randomized tests against arkworks reference behavior
- Real row/layout measurements via `midnight_proofs::dev::cost_model`
- Small Criterion benchmark hooks over the actual Week 1 sanity circuits
- A canonical primitive registry in `wrapper-circuits/src/planning.rs` now drives measured primitive metadata for `wrapper-cli doctor` and `wrapper-cli bench-info`
- CLI reporting that reflects measured primitive layout data
- A single authoritative BN254 implementation path in `wrapper-circuits/src/bn254/` without leftover host-side compatibility modules

What still does not exist:

- pairings
- G2 subgroup checks or scalar multiplication
- full Miller loop line accumulation
- final exponentiation
- MSM
- Groth16 verification
- wrapper verifier logic
- production-focused optimization or proof-system integration work
- real proof or verification-key backend adapters

## Running the CLI

The CLI is intentionally small and honest about the current phase. In particular, `doctor` reports measured primitive layout metrics and the still-missing verifier pieces.

```bash
cargo run -p wrapper-cli -- about
cargo run -p wrapper-cli -- doctor
cargo run -p wrapper-cli -- print-layout
cargo run -p wrapper-cli -- validate-config --config crates/wrapper-tests/fixtures/example-config.toml
cargo run -p wrapper-cli -- bench-info
```

## Development Workflow

1. Keep domain modeling in `wrapper-core` first.
2. Add Halo2-facing types in `wrapper-circuits` only when they truly require circuit integration, and prefer extending `src/bn254/` over reintroducing parallel primitive wrappers.
3. Put proof artifact loading and ecosystem adapters in `wrapper-backends`.
4. Expose orchestration and diagnostics through `wrapper-cli`.
5. Add regression coverage in `wrapper-tests` before growing implementation scope.

For the current primitive-foundation phase, prefer correctness and measured layout visibility over optimization.

## Roadmap / Phases

- Initialization: workspace scaffold, docs, CLI, placeholders, tests
- Stage 1 / Week 1: current phase; Midnight-backed BN254 `fp add` / `fp mul`, minimal G1 addition, arkworks sanity checks, layout visibility
- Stage 1 / Week 2 slice 1: BN254 `fp2` arithmetic over the existing Midnight-backed `AssignedFp` layer, with measured add/mul/square costs
- Stage 1 / Week 2 slice 2: minimal BN254 G2 affine assignment, equality, negation, and twist on-curve validation
- Stage 1 / Week 2 slice 3: narrow BN254 G2 Jacobian projective embedding, negation, doubling, incomplete addition, and cost visibility
- Stage 1 / early Week 3 slice: BN254 `fp6` arithmetic over the existing `AssignedFp2` layer, with measured add/mul/square costs
- Stage 1 / Week 3 slice: BN254 `fp12` arithmetic over the existing `AssignedFp6` layer, with measured add/mul/square costs
- Stage 1 / Week 3 slice: BN254 G2 `double_with_line` / `mixed_add_with_line` extraction with Miller-ready sparse coefficient layout
- Later pairing work: foreign field and pairing-related gadget research
- Later wrapper verifier work: Groth16 verifier logic inside the outer proof system
- Possible Cardano integration: ecosystem-specific packaging, artifacts, and engineering constraints
- Possible Semaphore migration case study: testing a migrated application-shaped circuit use case

See [docs/roadmap.md](docs/roadmap.md) for more detail.

## Design Principles

- Keep core architecture explicit and boring.
- Avoid fake implementations and cryptographic theater.
- Separate domain concerns from Halo2 concerns.
- Isolate backend adapters so parser churn does not infect circuit code.
- Document intent before implementation complexity grows.
- Add dependencies conservatively.

## Testing Strategy

Current strategy:

- Compile all crates
- Validate CLI behavior
- Test configuration parsing and current metadata/status behavior
- Validate BN254 field and G1 behavior against arkworks
- Keep small Midnight-backed sanity benchmarks runnable so future performance work has a consistent home

Future strategy:

- Fixture-driven integration tests in `wrapper-tests`
- Golden-file checks for artifact parsing
- Cross-crate contract tests between domain, backend, and circuit layers
- Eventually, proof-generation and verification test matrices once cryptographic code exists

## Non-Goals for This Phase

- Shipping a usable wrapper proof system
- Implementing full G2 arithmetic, pairings, or Groth16 verification
- Selecting a final proving backend
- Claiming compatibility with production proof artifacts

## Disclaimer

This repository now contains a circuit-backed BN254 primitive layer using `midnight-circuits` and `midnight-proofs`, organized under `wrapper-circuits/src/bn254/`, including a first Week 2 Fp2 slice, minimal Fp6 and Fp12 slices, a minimal G2 affine slice, a narrow Jacobian G2 projective slice, and a Miller-path G2 line-extraction slice. The Fp6 and Fp12 layers support `add`, `sub`, `neg`, `mul`, and `square` over the arkworks-compatible BN254 tower; the Jacobian G2 layer supports non-identity `from_affine`, `neg`, `double`, and incomplete `add`; and the Miller-path layer supports non-identity `double_with_line` and `mixed_add_with_line` with sparse `Fp12`-facing coefficients. The repository still does not include subgroup checks, scalar multiplication, a full Miller loop, final exponentiation, pairings, Groth16 verification, or a wrapper verifier circuit. Current Criterion benchmarks are sanity-check hooks over small implemented circuits and should not be read as production cryptographic performance claims.
