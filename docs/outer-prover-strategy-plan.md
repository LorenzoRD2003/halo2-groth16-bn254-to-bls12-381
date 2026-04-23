# Outer Prover Strategy Plan

## Purpose

This document captures the next technical decision needed to unblock real outer
artifact production after the current Halo2/Midnight outer wrapper circuit was
landed.

The repository now has:

- a real outer wrapper circuit in `wrapper-circuits`
- a frozen outer statement contract
- strict produced-artifact types in `wrapper-core`
- backend-side adaptation, setup planning, proof planning, and shape validation
  in `wrapper-backends`

What it still does not have is a concrete prover/setup/verification engine that
can turn the current Halo2/Midnight outer circuit into real Groth16 BLS12-381
artifacts.

## Current Constraint

The canonical outer circuit now lives in Halo2/Midnight.

That means the remaining blocker is **not** "rewrite the circuit in arkworks".
The blocker is: choose and integrate a prover/backend path that can take the
current Halo2/Midnight circuit surface and produce:

- `wrapper-verification-key.json`
- `wrapper-proof.json`
- backend-level verification results

without breaking the current artifact contract.

## Goal

Choose one prover strategy for the Halo2/Midnight outer circuit that makes
steps 6-8 of `docs/real-circom-wrapper-integration-plan.md` implementable
without re-opening the circuit/source-of-truth question.

The decision must answer:

1. How setup will be run for the current outer circuit.
2. How proving will be run for the current outer circuit.
3. How verification will be run for the produced proof/VK/public-input bundle.
4. How those concrete backend outputs will be serialized into the existing
   `snarkjs`-like outer artifact model.

## Non-Goals

This plan is not about:

- changing the frozen outer statement contract
- changing the current outer circuit semantics
- replacing Halo2/Midnight as the outer circuit source of truth
- widening the outer artifact model beyond the current `snarkjs`-like shape

## Decision Options To Evaluate

### Option A: Native Halo2/Midnight proving path that already targets the required artifact family

Questions to answer:

- Does Midnight or an immediately compatible proving layer already support
  Groth16 BLS12-381 output for a Halo2/Midnight-authored circuit?
- If yes, can setup/proof/VK objects be serialized into the current
  `pi_a/pi_b/pi_c`, `nPublic`, `IC` model without lossy translation?

Use this option if it exists with acceptable integration cost.

### Option B: Translation path from Halo2/Midnight circuit to another Groth16-capable proving layer

Questions to answer:

- Is there a stable, auditable path from the current circuit representation to
  a Groth16 BLS12-381 prover?
- Does that translation preserve one canonical circuit identity for CRS
  purposes?
- Can the translated proof/VK still map cleanly into the current output model?

Use this option only if Option A is unavailable.

### Option C: Change the outer proof system target instead of forcing Groth16 BLS12-381

Questions to answer:

- Is Groth16 BLS12-381 still the right outer artifact target for this repo?
- If not, what target matches the current Halo2/Midnight circuit stack better?
- What contract changes would be required in `wrapper-core`, CLI, and docs?

Use this option only if neither Option A nor B is viable.

## Recommended Evaluation Order

1. Check whether the current Halo2/Midnight stack already exposes a setup/prove
   surface compatible with the desired outer proof target.
2. If not, check for a minimal translation/proving bridge that keeps the outer
   circuit canonical in `wrapper-circuits`.
3. Only if both fail, revisit the outer proof-system target.

## Acceptance Criteria For The Decision

The prover strategy is considered selected only when all of the following are
written down in the same change:

- one concrete backend stack or bridge is named
- setup input/output ownership is defined
- proof generation ownership is defined
- verification ownership is defined
- artifact serialization ownership is defined
- the decision states whether new dependencies are required
- the decision states whether `wrapper-core` contracts remain unchanged
- the decision states whether `wrapper-circuits` remains the sole circuit source
  of truth

## Expected Follow-On Work After Selection

Once the prover strategy is chosen:

1. Implement real `setup()` in `wrapper-backends/src/outer.rs`.
2. Materialize a real `ProducedOuterGroth16VerificationKeyJson`.
3. Implement real `prove()` in `wrapper-backends/src/outer.rs`.
4. Materialize a real `ProducedOuterGroth16ProofJson`.
5. Implement real `verify()` in `wrapper-backends/src/outer.rs`.
6. Promote the real fixture lane to one end-to-end produced-bundle integration
   test.

## Current Default Assumption

Until a better option is proven viable, the repository should continue to treat
the Halo2/Midnight outer circuit as canonical and avoid building a second,
parallel circuit implementation in another framework just to satisfy the prover
integration.
