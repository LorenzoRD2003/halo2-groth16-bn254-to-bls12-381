# Architecture

## Purpose

This repository is structured for staged development of a Halo2-based wrapper around Groth16 BN254 proofs. The current repository state now includes a circuit-backed BN254 primitive layer: Week 1 delivered Fp and minimal G1 support, Week 2 / Week 3 added the first Fp2/Fp6/Fp12 and narrow G2 slices, and Week 4 now reaches the pairing core through the real Miller loop, final exponentiation, and a narrow pairing-product check. It still does not implement subgroup checks, scalar multiplication, broad verifier-facing pairing APIs, verifier logic, or a production wrapper circuit.

## Intended Data Flow

The expected long-term flow is:

1. Backend adapters load or normalize external artifacts such as proof metadata, verification key material, or ecosystem-specific formats.
2. `wrapper-core` expresses stable domain concepts for those artifacts, wrapper configuration, capability declarations, and execution boundaries.
3. `wrapper-circuits` consumes domain-level configuration and normalized metadata to construct Halo2-facing circuit descriptions.
4. The CLI or future orchestration layers coordinate configuration loading, validation, inspection, and eventually proof-related workflows.

The current implementation includes enough BN254 arithmetic to validate Week 1 interfaces, circuit wiring, and layout measurements, while still stopping well short of a wrapper verifier.

## Why `wrapper-core` Stays Domain-Oriented

`wrapper-core` is the anchor for stable concepts that should outlive changes in circuit frameworks or backend adapters. Keeping it mostly independent from Halo2 has several advantages:

- domain modeling can evolve without dragging proving-system dependencies into every consumer
- CLI validation and backend parsing can remain lightweight
- tests can exercise core logic without requiring cryptographic crates
- future rewrites of circuit internals do not force broad public API churn

## Why Circuits and Backends Are Separate

Circuit code and backend integration change for different reasons.

`wrapper-circuits` will eventually own:

- Halo2 circuit composition
- chip and gadget organization
- layout and witness-shape planning
- outer wrapper circuit boundary definitions
- the BN254 foreign-field layer introduced in Week 1 and extended in Week 2 / Week 3 with Fp2, Fp6, and Fp12
- the BN254 G1 abstraction layer introduced in Week 1
- the BN254 G2 affine representation layer introduced in Week 2
- the BN254 G2 Jacobian projective layer introduced in Week 2
- the BN254 G2 Miller-path line-extraction layer introduced in Week 3

`wrapper-backends` will eventually own:

- artifact loading
- verification key ingestion
- proof metadata parsing
- compatibility adapters for other libraries and ecosystems

Separating these concerns prevents parser logic, serialization quirks, or artifact format churn from leaking into circuit modules.

## Halo2 Boundary Strategy

The project expects Halo2-specific code to live primarily in `wrapper-circuits`. Week 1 now uses `midnight-circuits` and `midnight-proofs` directly for a first real non-native BN254 layer, while keeping the supported surface intentionally small. This gives the project real circuit feedback without overcommitting to later-stage pairing or verifier APIs.

When Halo2 is introduced later:

- `wrapper-core` should still avoid direct dependence unless a boundary cannot be represented otherwise
- `wrapper-circuits` should absorb the proving-system integration surface
- `wrapper-backends` should remain focused on external artifact and ecosystem concerns

## BN254 Foreign-Field Layer

Week 1 adds an `AssignedFp` abstraction in `wrapper-circuits`, and Week 2 begins by layering `AssignedFp2` on top of it.

Current properties:

- Midnight-backed assigned BN254 base-field values
- circuit-backed `add`, `sub`, `neg`, `mul`, and `square`
- circuit-backed BN254 Fp2 values represented as `(c0, c1)` for `c0 + c1 * u`
- Fp2 `add`, `sub`, `neg`, `mul`, and specialized `square` expressed through the existing `AssignedFp` layer
- tuple-based host/reference arithmetic shared across the BN254 tower now lives in `wrapper-circuits/src/bn254/host.rs` instead of being redefined in each extension-field or G2 module
- real row and layout measurements via `midnight_proofs::dev::cost_model`
- arkworks-backed randomized correctness tests

Current limitations:

- no production-oriented optimization or custom layout tuning yet
- no pairing-specific arithmetic yet
- row and query reporting is real, but still only for the narrow implemented circuits

## BN254 Fp6 Layer

The next extension-field slice adds an `AssignedFp6` abstraction in `wrapper-circuits`.

Current properties:

- Fp6 elements represented as `(c0, c1, c2)` for `c0 + c1 * v + c2 * v^2`
- exact arkworks BN254 tower: `Fp2 = Fp[u] / (u^2 + 1)` and `Fp6 = Fp2[v] / (v^3 - (9 + u))`
- exact arkworks BN254 cubic nonresidue `9 + u`
- circuit-backed `add`, `sub`, `neg`, `mul`, and `square`
- deterministic arkworks-backed randomized correctness tests
- real layout metrics for `fp6_add`, `fp6_mul`, and `fp6_square`

Current limitations:

- no inversion in this slice
- no pairing-specific line-function or Miller-loop logic yet

## BN254 Fp12 Layer

The current Week 3 slice adds an `AssignedFp12` abstraction in `wrapper-circuits`.

Current properties:

- Fp12 elements represented as `(c0, c1)` for `c0 + c1 * w`
- exact arkworks BN254 tower: `Fp2 = Fp[u] / (u^2 + 1)`, `Fp6 = Fp2[v] / (v^3 - (9 + u))`, and `Fp12 = Fp6[w] / (w^2 - v)`
- exact arkworks BN254 quadratic nonresidue `v = Fp6(0, 1, 0)`
- circuit-backed `add`, `sub`, `neg`, `mul`, and `square`
- deterministic arkworks-backed randomized and structured correctness tests
- real layout metrics for `fp12_add`, `fp12_mul`, and `fp12_square`

Current limitations:

- no inversion in this slice
- no Miller-loop or final-exponentiation logic yet

## BN254 G1 Abstraction Layer

Week 1 also adds an `AssignedG1` abstraction in `wrapper-circuits`.

Current properties:

- Midnight-backed assigned BN254 G1 points
- circuit-backed complete point addition
- coordinate-to-point construction with on-curve enforcement
- deterministic arkworks-backed correctness tests
- real layout metrics for the Week 1 G1 addition circuit

Current limitations:

- no public Week 1 MSM surface
- no subgroup-check or cofactor-clearing workflow yet
- no G2 or pairing support

## BN254 G2 Affine Layer

Week 2 adds a minimal `AssignedG2Affine` abstraction in `wrapper-circuits`.

Current properties:

- G2 affine points represented as `(x, y)` over `AssignedFp2`
- circuit-backed non-infinity assignment from Fp2 coordinates
- circuit-backed negation
- circuit-backed equality checks
- explicit twist on-curve validation against the BN254 G2 equation from arkworks
- real layout metrics for narrow `g2 on_curve` and `g2 neg` sanity circuits

Current limitations:

- no identity/infinity representation in this slice
- no subgroup checks yet
- no pairing support

## BN254 G2 Projective Layer

The current narrow Week 2 slice adds an `AssignedG2Projective` abstraction in `wrapper-circuits`.

Current properties:

- Jacobian coordinates `(X : Y : Z)` over `AssignedFp2`
- affine model `x = X / Z^2`, `y = Y / Z^3` for `Z != 0`
- explicit reserved infinity encoding via the conventional Jacobian representative `(1 : 1 : 0)`
- circuit-backed `from_affine` embedding with `Z = 1`
- circuit-backed `neg`
- circuit-backed doubling with the standard short-Weierstrass Jacobian doubling formula for `a = 0` (`dbl-2009-l`)
- circuit-backed Jacobian-Jacobian addition with the standard incomplete formula (`add-2007-bl`)
- affine-equivalence checks used in tests and sanity circuits instead of full in-circuit normalization
- deterministic arkworks-backed randomized correctness tests
- real layout metrics for `g2_proj_from_affine`, `g2_proj_double`, and `g2_proj_add`

Current limitations:

- arithmetic is intentionally incomplete and only intended for non-identity points
- `add` does not yet support identity operands, `P = Q`, or `P = -Q`
- no subgroup checks yet
- no scalar multiplication yet
- no pairing support

## BN254 G2 Miller-Step Layer

The current Week 3 line-extraction slice adds a dedicated Miller-path G2 step state and sparse line coefficients in `wrapper-circuits`.

Current properties:

- a dedicated `AssignedG2MillerPoint` homogeneous-projective state `(X : Y : Z)` with affine model `x = X / Z`, `y = Y / Z`
- this state is intentionally separate from `AssignedG2Projective`, which remains Jacobian for the narrow general-purpose G2 arithmetic slice
- a dedicated `AssignedG2LineCoeffs` type with the Miller-ready sparse layout `(ell_0, ell_w, ell_vw)`
- a dedicated `AssignedMillerAccumulator` type as the public consumption boundary for those coefficients
- the line layout is chosen for the BN254 D-twist sparse Fp12 embedding
  `ell_0 * y_P + ell_w * x_P * w + ell_vw * v * w`
- `double_with_line` follows the homogeneous-projective BN prepared-G2 doubling step used by arkworks `G2HomProjective::double_in_place`
- `mixed_add_with_line` follows the homogeneous-projective BN prepared-G2 mixed-add step used by arkworks `G2HomProjective::add_in_place`
- the public consumption boundary is `AssignedG2LineCoeffs -> AssignedMillerAccumulator::mul_by_line(...)`
- sparse line evaluation into Fp12 remains an internal accumulator detail rather than an `AssignedFp12`-level public helper
- the public `mul_by_line(...)` accumulator path now uses an internal sparse-specialized D-twist multiplication path instead of paying a near-full generic `Fp12` multiply
- the previous generic line-consumption path remains available only as an explicit baseline circuit/metric so optimization progress stays measurable
- a narrow accumulator-driven Miller loop now exists over the real fixed BN254 optimal-ate prepared-step schedule
- the loop driver keeps step scheduling explicit and deterministic through a dedicated host-side BN254 schedule representation rather than witness-driven branching
- the implemented loop shape now matches arkworks BN254 prepared-G2 traversal, including the fixed Frobenius tail
- a narrow final exponentiation now exists on top of that Miller output using the standard BN easy-part / hard-part decomposition aligned with arkworks
- a narrow multi-pairing product check now exists: compute each real Miller loop, multiply the Miller outputs together, apply exactly one final exponentiation, and compare the total product against the target-group identity
- deterministic arkworks-backed reference tests cover point updates, extracted coefficients, sparse Fp12 embedding, and unsupported edge cases
- real layout metrics for `g2_double_with_line`, `g2_mixed_add_with_line`, `miller accumulator square`, `miller accumulator mul_by_line` (generic baseline), `miller accumulator mul_by_line sparse` (optimized path), the narrow `miller loop` sanity circuit, the narrow `final exponentiation` sanity circuit, and the narrow `pairing check` sanity circuit

Current limitations:

- the Miller-path state is intentionally non-identity only in this slice
- `mixed_add_with_line` is intentionally unsupported for exceptional cases such as `P = Q` and `P = -Q`
- the current pairing slice now covers single-pair Miller accumulation, final exponentiation, and a narrow multi-pairing product check for supported non-exceptional inputs
- this is still not a broad public full-pairing or multi-pairing API beyond the narrow product-check boundary
- no Groth16 verification path or wrapper verifier circuit exists yet

## Current Architectural Contracts

The current skeleton defines:

- wrapper phases and status reporting
- wrapper capabilities and implementation status markers
- repository configuration parsing and validation
- layout descriptors for future circuit inspection
- backend registry and artifact loader interfaces
- BN254 field, Fp2, G1, and minimal G2 affine foundations with real layout visibility
- a canonical primitive registry in `wrapper-circuits/src/planning.rs` that drives measured primitive metadata for CLI reporting and benchmark-info output

These contracts are intentionally conservative and meant to support staged development rather than predict final cryptographic APIs in detail.
