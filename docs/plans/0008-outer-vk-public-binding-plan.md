# Outer VK Public Binding Plan

## Status

Implementation status as of the current Stage 1 / Week 5+ direct outer lane:

- implemented end-to-end in the canonical outer statement, planning/package
  layers, backend adaptation path, and outer circuit semantics
- the old mirror-only constructor path was replaced in favor of the stronger
  explicit statement form
- the witness-side inner verification key is now bound in-circuit to one public
  verification-key commitment

This document started as an implementation plan. It now also records the final
decisions that were actually landed in code.

## Purpose

This document describes an implementation plan for strengthening the public
claim made by the canonical outer wrapper circuit.

Current issue:

- the current outer proof exposes the mirrored inner public-input vector
- the inner Groth16 proof and the inner verification key remain witness-side
- the current public claim is therefore existential with respect to the inner
  verification key

More precisely, the current outer proof shows:

- there exists an inner Groth16 proof, an inner verification key, and an
  ordered public-input vector such that the inner verifier accepts and the
  outer statement matches the mirrored public-input view

It does not show:

- that verification happened against one publicly identified verification key

Primary goal:

- bind the outer public claim to a specific inner verification key without
  exposing the full verification key as public inputs

Recommended direction:

- keep the full inner verification key witness-side
- add a public commitment of that verification key to the outer statement
- constrain the witness-side verification key to match that public commitment

This is an implementation plan plus implementation-status record for the
current Stage 1 / Week 5+ direct outer lane.

Current outer-lane policy:

- the public-facing official lane is `BLS12-381`
- the BN254-hosted outer lane is retained as a compatibility/testing lane
- the statement semantics and VK binding described here are lane-independent
  and intentionally shared across both hosted lanes

## Landed Design

The implementation that landed in the repository makes the following concrete
choices:

1. the outer statement is modeled explicitly, not as an unnamed flat vector
2. the semantic outer statement contains:
   - one mirrored inner public-input component
   - one explicit `vk_commitment` component
3. the semantic VK commitment is one BN254 base-field element
4. that semantic field element is flattened to canonical host-lane public-input
   limbs for exposure in Halo2 instance columns
5. the mirror-only constructor path was replaced rather than preserved for
   compatibility

Relevant code:

- `crates/wrapper-circuits/src/groth16/commitment.rs`
- `crates/wrapper-circuits/src/outer/statement.rs`
- `crates/wrapper-circuits/src/outer/input.rs`
- `crates/wrapper-circuits/src/outer/semantics.rs`
- `crates/wrapper-core/src/package.rs`
- `crates/wrapper-backends/src/groth16.rs`
- `crates/wrapper-backends/src/outer/direct/adaptation.rs`

## Final Commitment Definition

The implementation does not use raw JSON bytes, filesystem bytes, or a
byte-oriented artifact hash.

Instead, it commits to the normalized Rust object
`Groth16Bn254VerifyingKey` by:

1. traversing the semantic VK fields in this order:
   - `alpha_g1`
   - `beta_g2`
   - `gamma_g2`
   - `delta_g2`
   - `ic` in verifier order
2. flattening those points into BN254 base-field coordinates in this order:
   - G1 as `(x, y)`, with identity encoded as `(0, 0)`
   - G2 as `(x.c0, x.c1, y.c0, y.c1)`
3. hashing the resulting coordinate stream with a canonical Poseidon-based
   commitment over `BN254::Fq`, implemented in
   `crates/wrapper-circuits/src/groth16/commitment.rs`

More precisely, the landed commitment is:

- a Poseidon x^5 based sequential commitment over the canonical VK coordinate
  stream
- defined directly over `BN254::Fq`
- recomputed in-circuit through the existing non-native BN254 field chip

This commitment is:

- semantic rather than artifact-byte-based
- stable across outer host lanes
- representable as one semantic field element
- flattened to canonical public-input limbs through the existing foreign-field
  public-input encoding

## Final Statement Shape

The semantic outer statement is no longer just:

- `field_names`
- `public_inputs`

The landed `OuterStatementInput` now carries:

- `semantics`
- `mirrored_field_names`
- `mirrored_public_inputs`
- `vk_commitment`
- derived flat `field_names`
- derived flat `public_inputs`

The flat public-input vector exposed to Halo2 is derived as:

1. all mirrored public inputs in caller-supplied order
2. all flattened limbs of the semantic `vk_commitment` field element

Current flattened naming convention:

- mirrored fields keep their existing names
- VK commitment limbs are named `vk_commitment_limb_<index>`

## In-Circuit Binding Rule

The landed circuit-side enforcement is:

1. assign the witness-side normalized inner verification key as non-native BN254
   values
2. recompute the canonical VK commitment inside the outer circuit
3. constrain that computed commitment to equal the explicit semantic
   `vk_commitment` carried by the outer statement
4. keep the existing narrow Groth16 verifier relation otherwise unchanged

This means the public claim is now bound to one publicly identified inner
verification key, not just to some existential witness-side key.

## Why This Matters

The current design is sufficient for local experimentation, profiling, and
pipeline bring-up, but it leaves the public statement under-specified.

Without a public binding to the verification key, an external verifier learns:

- some inner verification key was used
- some inner proof verified against that key
- the mirrored public-input vector matched the outer statement

The verifier does not learn:

- which Groth16 circuit the claim refers to
- whether the accepted proof was checked against the intended verification key

For any interface that is meant to be reused outside local experiments, this is
too weak. The outer proof should be bound to a stable identifier of the inner
verification key.

## Non-Goals

This plan does not propose:

- exposing the full inner verification key as public input
- exposing the full inner proof as public input
- changing the narrow Groth16 verifier relation
- redesigning the outer proof backend
- introducing a broad statement DSL
- making the canonical R1CS lane the primary implementation path

## Recommended Public Claim

After this change, the intended public claim should be:

- the supplied outer public statement mirrors the ordered inner public inputs
- the inner Groth16 proof verified successfully
- the witness-side inner verification key hashes to the public verification-key
  commitment included in the outer statement

That is still an existential claim with respect to the full proof witness, but
it is no longer existential with respect to the identity of the verified inner
circuit.

## Candidate Designs

### A. Expose the full verification key publicly

Pros:

- strongest and simplest semantic statement
- no ambiguity about what key was used

Cons:

- large public-input footprint
- poor operational ergonomics
- expands the public boundary more than needed
- does not match the current narrow outer-statement design

Verdict:

- reject for the current phase

### B. Fix the verification key as a circuit constant

Pros:

- very strong binding
- no extra public input beyond the existing statement

Cons:

- ties one outer circuit instance to one specific inner key
- poor fit for the current artifact-driven workflow
- complicates reuse across fixtures and future integrations

Verdict:

- reject as the default design
- may remain useful for future application-specific lanes

### C. Expose a public commitment of the verification key

Pros:

- strong enough semantic binding for the current lane
- much smaller public footprint than exposing the full key
- compatible with the existing witness-side artifact flow
- compatible with both BN254-hosted and BLS12-381-hosted outer lanes

Cons:

- requires a stable canonical serialization and commitment definition
- adds one new public binding component to the outer statement

Verdict:

- recommended

## Commitment Design Constraints

The verification-key commitment must satisfy the following constraints:

1. it must be computed from a canonical serialization of the narrow
   `Groth16Bn254VerifyingKey`
2. it must be stable across host lanes
3. it must not depend on outer proof backend details
4. it must not depend on JSON formatting quirks from `snarkjs`
5. it must be representable in the outer public statement as a compact field
   element or a small fixed tuple of field elements

## Canonical Serialization Scope

The commitment should be derived from the semantic verification key, not from
raw artifact bytes.

That means the commitment should be computed over the normalized Rust object:

- `alpha_g1`
- `beta_g2`
- `gamma_g2`
- `delta_g2`
- `ic`

and not over:

- raw JSON text
- filesystem layout
- backend-specific wrapper metadata

This keeps the binding stable even if the artifact ingestion surface changes in
ways that preserve semantic equivalence.

## Statement-Surface Design

The current `OuterStatementInput` only models:

- `semantics`
- `field_names`
- `public_inputs`

That is enough for the mirror-only statement, but not enough for a richer
public claim that includes a VK commitment.

The cleanest next step is to extend the outer statement model so it can expose:

1. mirrored inner public inputs
2. a public verification-key commitment

Recommended semantic shape:

- preserve the current mirror semantics as one explicit sub-claim
- add one explicit public field or fixed field tuple for the VK commitment

This can be expressed either by:

- extending `OuterStatementSemantics`
- or extending `OuterStatementInput` with additional public-claim material

The plan below assumes the semantic model is widened explicitly rather than
smuggling the commitment in as an unnamed extra public input.

## Hash / Commitment Direction

This question is now resolved for the landed implementation.

Chosen direction:

- a Poseidon x^5 based canonical commitment over the normalized VK object
- one semantic BN254 base-field element
- flattened to canonical public-input limbs only at the host-lane exposure
  boundary

Why this was chosen:

- deterministic and easy to reproduce off-circuit
- implementable in the current circuit stack with the existing non-native
  BN254 field chip, without introducing a broader byte-hash gadget effort
- stable across BN254-hosted and BLS12-381-hosted outer lanes
- compatible with an explicit statement model and in-circuit verification

## Implementation Phases

## Phase 1. Define the public semantic contract

Goal:

- specify exactly what new public statement the outer circuit should expose

Tasks:

1. define the desired claim in `docs/outer-wrapper-circuit-layered-walkthrough.md`
2. choose whether the outer statement should:
   - append a VK commitment field to the mirrored inputs
   - or model mirrored inputs and VK commitment as named subcomponents
3. define naming conventions for the new public field or fields
4. define how the current mirror-only statement maps to the new structure

Acceptance criteria:

- one precise public-claim definition exists in docs
- the statement shape is unambiguous for both fixtures and future integrations

Status:

- completed

## Phase 2. Define canonical VK serialization and commitment computation

Goal:

- define the exact bytes or field elements committed by the public VK binding

Tasks:

1. add a canonical serialization helper for `Groth16Bn254VerifyingKey`
2. define the field ordering and point-coordinate ordering explicitly
3. define the commitment function over that canonical representation
4. add host-side tests showing that semantically identical VKs produce the same
   commitment

Files likely involved:

- `crates/wrapper-circuits/src/groth16.rs`
- `crates/wrapper-backends/src/groth16.rs`
- possibly `crates/wrapper-core/src/output.rs` or adjacent statement-facing
  types if commitment values travel through planning/export surfaces

Acceptance criteria:

- one canonical serialization exists
- one stable commitment helper exists
- the commitment definition is documented in code and docs

Status:

- completed

## Phase 3. Extend outer statement types

Goal:

- make the VK commitment a first-class part of the outer public statement

Tasks:

1. extend `OuterStatementSemantics` as needed
2. extend `OuterStatementInput` so the VK commitment is explicit
3. update validation so the statement contract includes both:
   - mirror of inner public inputs
   - VK commitment consistency requirements
4. update helper constructors such as `OuterWrapperCircuitInput::mirrored(...)`
   or replace them with a more precise constructor

Files expected to change:

- `crates/wrapper-circuits/src/outer/statement.rs`
- `crates/wrapper-circuits/src/outer/input.rs`
- `crates/wrapper-circuits/src/outer/mod.rs`
- tests under `crates/wrapper-circuits/src/outer/tests.rs`

Acceptance criteria:

- statement construction cannot silently omit the VK commitment
- validation fails on arity or commitment-shape mismatch

Status:

- completed

## Phase 4. Compute and carry the commitment through planning layers

Goal:

- ensure the commitment exists before circuit synthesis and is available to the
  outer statement builder

Tasks:

1. compute the VK commitment when building outer circuit input material
2. ensure the bundle -> job -> package path carries enough semantic data to
   reconstruct the outer statement
3. update fixture helpers and backend adaptation code to populate the new
   statement field

Files expected to change:

- `crates/wrapper-backends/src/groth16.rs`
- `crates/wrapper-backends/src/outer.rs`
- `crates/wrapper-core/src/job.rs`
- `crates/wrapper-core/src/package.rs`
- fixture helpers in `crates/wrapper-tests/src/lib.rs`

Acceptance criteria:

- both `circom_multiplier2` and `semaphore` can construct outer inputs with the
  VK commitment present
- planning/export surfaces remain coherent

Status:

- completed

## Phase 5. Constrain the witness-side VK against the public commitment

Goal:

- enforce inside the outer circuit that the witness-side VK matches the public
  commitment

Tasks:

1. decide whether the first implementation will:
   - compute the full commitment inside the circuit
   - or use a smaller staged binding if full in-circuit hashing is too heavy
2. add the circuit-side check before or alongside the current verifier
   semantics
3. ensure the check is host-lane independent
4. keep the actual Groth16 verifier path unchanged apart from the new binding

Files expected to change:

- `crates/wrapper-circuits/src/outer/semantics.rs`
- any new helper module used for VK commitment checking
- possibly `crates/wrapper-circuits/src/groth16.rs` if VK traversal helpers are
  reused

Acceptance criteria:

- changing the witness-side VK without changing the public commitment causes the
  outer circuit to fail
- a matching VK and commitment pair still synthesizes successfully

Status:

- completed

## Phase 6. Update public exposure and CLI surfaces

Goal:

- make the new public claim visible in the current developer workflow

Tasks:

1. update any CLI display or export path that prints outer statement values
2. document how the new statement fields are ordered
3. update fixture README files if their public statement interpretation changes

Files expected to change:

- `crates/wrapper-cli/src/main.rs`
- fixture docs under `crates/wrapper-tests/fixtures/groth16/`
- `docs/outer-wrapper-circuit-layered-walkthrough.md`

Acceptance criteria:

- operator-facing commands can explain the new public statement shape
- fixture docs no longer imply that mirrored public inputs are the whole public
  claim

Status:

- partially completed

Notes:

- the outer-circuit walkthrough was updated to describe the stronger claim and
  the explicit statement shape
- the core planning/export surfaces now materialize the stronger statement
- broader CLI wording and every fixture-facing README were not fully rewritten
  as part of the initial landing

## Phase 7. Add regression coverage

Goal:

- lock the stronger public claim into tests

Required tests:

1. valid proof + valid VK + valid commitment succeeds
2. valid proof + mutated mirrored public input fails
3. valid proof + mutated public VK commitment fails
4. valid proof + mutated witness-side VK with unchanged public commitment fails
5. commitment computation is stable across both host lanes

Likely test locations:

- `crates/wrapper-circuits/src/outer/tests.rs`
- `crates/wrapper-backends/src/groth16.rs`
- `crates/wrapper-tests/src/lib.rs`

Acceptance criteria:

- the public-binding claim is protected by tests, not only by documentation

Status:

- completed for the landed statement/backend/circuit surfaces

## Design Questions to Resolve Early

These questions were resolved by the landed implementation:

1. commitment function:
   - canonical Poseidon-based VK commitment defined in
     `crates/wrapper-circuits/src/groth16/commitment.rs`
2. commitment shape:
   - one semantic BN254 field element
   - flattened to host-lane public-input limbs for exposure
3. statement model:
   - explicit semantic model with mirrored inputs and VK commitment separated
4. mirror-only constructor:
   - replaced rather than retained

## Risks

### 1. Statement-shape churn

Risk:

- the current public statement is a simple mirrored vector
- adding a VK commitment changes public-input order and semantics

Mitigation:

- make the new structure explicit in types and docs
- do not rely on unnamed appended fields

### 2. Circuit cost growth

Risk:

- a full in-circuit VK commitment check may be nontrivial in cost

Mitigation:

- make commitment design a first-class decision
- prefer a design that is simple to verify correctly before optimizing it

Current note:

- this risk remains relevant for performance tuning, but it is no longer a
  semantic blocker because the stronger binding has already landed

### 3. Serialization ambiguity

Risk:

- two semantically equal VKs may serialize differently if normalization is not
  fixed

Mitigation:

- commit to the normalized Rust object representation, not raw artifact bytes

### 4. Workflow inconsistency

Risk:

- planning, CLI, fixtures, and docs may disagree about the new public claim

Mitigation:

- treat Phase 4 and Phase 6 as required, not optional polish

Current note:

- the planning/package/backend path is aligned
- the main remaining cleanup area is broader CLI and fixture-facing wording

## Recommended Delivery Order

Recommended order:

1. public-claim definition
2. canonical VK serialization
3. host-side commitment helper
4. outer statement type changes
5. planning / adaptation / fixture wiring
6. circuit-side commitment check
7. regression tests
8. CLI and documentation cleanup

This order was followed in substance by the landed implementation, with some
documentation cleanup continuing after the semantic and circuit work landed.

## Success Criteria

This plan is complete when:

1. the outer statement contains a public identifier of the inner verification
   key
2. the witness-side inner verification key is constrained to match that public
   identifier
3. the inner proof still verifies through the existing narrow Groth16 path
4. both committed fixtures continue to run through the direct outer lane
5. docs describe the stronger public claim precisely

Current status:

- items 1 through 4 are implemented
- item 5 is implemented for the core outer-circuit walkthrough and code-facing
  semantics docs, with some broader operator-facing cleanup still available as
  follow-up polish

## Minimal First Milestone

If the full end state is too large for one pass, the minimum worthwhile
milestone is:

1. define canonical VK serialization
2. compute a host-side VK commitment
3. include that commitment in the outer statement and all planning/export paths
4. update docs to state the intended stronger claim

That milestone would not yet fully enforce the VK binding inside the circuit,
but it would establish the public contract and the surrounding plumbing.

The full semantic win arrives once the circuit constrains the witness-side VK
against the public commitment.

That condition is now satisfied by the landed implementation.
