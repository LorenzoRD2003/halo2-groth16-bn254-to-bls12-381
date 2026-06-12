# Semaphore Direct Execution Playbook

## Purpose

This note turns the committed Semaphore fixture into an explicit direct-lane
operator playbook in the same style already used for the smaller
`circom_multiplier2` fixture.

It is intentionally operational:

- where to persist artifacts
- which host lane to try first
- which commands to run
- what semantic public-input names to pass
- what memory/performance risks to expect

It does not claim that Semaphore is already as easy or as stable to run as
`circom_multiplier2`.

## Current Recommendation

For the first real direct-lane Semaphore runs in this repository:

- start with `midnight-bls12381-host`
- keep artifacts under `artifacts/direct-profile-semaphore/`
- start with `--h-poly-row-chunk-size 13`
- treat that chunk size as an initial operational default, not as a proven
  optimum for Semaphore

Why this is the current recommendation:

- the current architecture already supports the Semaphore fixture as a normal
  `snarkjs` Groth16 BN254 bundle
- the direct lane already supports semantic public-input names
- on this codebase and machine, the BLS12-381-hosted outer lane proved
  materially faster than the BN254-hosted outer lane for `circom_multiplier2`
- Semaphore is materially larger than `circom_multiplier2`, so the safer and
  faster currently-hardened host lane is the better first bet

Current lane policy:

- `midnight-bls12381-host` is the official outer lane for this repository's
  operator-facing flow
- the BN254-hosted outer lane is retained for compatibility/testing work, not
  as the preferred operational target

## Public Input Names

The committed Semaphore fixture uses this ordered verifier public-input vector:

1. `merkle_root`
2. `nullifier`
3. `message_hash`
4. `scope_hash`

Those names are already the canonical order used by the test harness.

Whenever you want the direct CLI outputs to preserve those semantic labels, add:

```bash
--public-input-name merkle_root \
--public-input-name nullifier \
--public-input-name message_hash \
--public-input-name scope_hash
```

This is primarily a readability and artifact-hygiene improvement:

- direct execution already works on the ordered values alone
- semantic names make resulting JSON artifacts and job/package views much
  easier to inspect later

## Persistent Artifact Layout

Create one dedicated directory:

```bash
mkdir -p artifacts/direct-profile-semaphore
```

Recommended artifact paths:

- setup manifest:
  `artifacts/direct-profile-semaphore/semaphore-setup.json`
- setup proving-key sidecar:
  `artifacts/direct-profile-semaphore/semaphore-setup.json.pk`
- persisted first-stage trace:
  `artifacts/direct-profile-semaphore/semaphore-trace.bin`
- finalized proof bundle:
  `artifacts/direct-profile-semaphore/semaphore-proof-bundle.json`

## Commands

### 1. Setup

```bash
cargo run -p wrapper-cli -- execute-wrapper-direct-setup \
  --id semaphore-depth-10 \
  --proof crates/wrapper-tests/fixtures/groth16/semaphore/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/semaphore/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/semaphore/verification_key.json \
  --public-input-name merkle_root \
  --public-input-name nullifier \
  --public-input-name message_hash \
  --public-input-name scope_hash \
  --backend midnight-bls12381-host \
  --output artifacts/direct-profile-semaphore/semaphore-setup.json
```

### 2. Prove Trace

```bash
cargo run -p wrapper-cli -- execute-wrapper-direct-prove-trace \
  --id semaphore-depth-10 \
  --proof crates/wrapper-tests/fixtures/groth16/semaphore/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/semaphore/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/semaphore/verification_key.json \
  --public-input-name merkle_root \
  --public-input-name nullifier \
  --public-input-name message_hash \
  --public-input-name scope_hash \
  --backend midnight-bls12381-host \
  --log-mode efficient \
  --setup artifacts/direct-profile-semaphore/semaphore-setup.json \
  --output artifacts/direct-profile-semaphore/semaphore-trace.bin
```

### 3. Prove Finalize

```bash
cargo run -p wrapper-cli -- execute-wrapper-direct-prove-finalize \
  --id semaphore-depth-10 \
  --proof crates/wrapper-tests/fixtures/groth16/semaphore/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/semaphore/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/semaphore/verification_key.json \
  --public-input-name merkle_root \
  --public-input-name nullifier \
  --public-input-name message_hash \
  --public-input-name scope_hash \
  --backend midnight-bls12381-host \
  --log-mode efficient \
  --setup artifacts/direct-profile-semaphore/semaphore-setup.json \
  --trace artifacts/direct-profile-semaphore/semaphore-trace.bin \
  --h-poly-row-chunk-size 13 \
  --output artifacts/direct-profile-semaphore/semaphore-proof-bundle.json
```

### 4. Verify

```bash
cargo run -p wrapper-cli -- execute-wrapper-direct-verify \
  --id semaphore-depth-10 \
  --proof crates/wrapper-tests/fixtures/groth16/semaphore/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/semaphore/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/semaphore/verification_key.json \
  --public-input-name merkle_root \
  --public-input-name nullifier \
  --public-input-name message_hash \
  --public-input-name scope_hash \
  --backend midnight-bls12381-host \
  --bundle artifacts/direct-profile-semaphore/semaphore-proof-bundle.json
```

## Live Log Commands

### `prove-trace`

```bash
tail -f "$HOME/tmp/execute-wrapper-direct-prove-trace-semaphore-depth-10-midnight-direct-halo2-outer-backend-bls12-host.log"
```

### `prove-finalize`

```bash
tail -f "$HOME/tmp/execute-wrapper-direct-prove-finalize-semaphore-depth-10-midnight-direct-halo2-outer-backend-bls12-host.log"
```

## Architecture Fit

The current architecture already supports Semaphore as a first-class example.

What is already in place:

- the fixture is committed under
  `crates/wrapper-tests/fixtures/groth16/semaphore/`
- the generic `snarkjs` Groth16 BN254 bundle loader accepts it without a
  Semaphore-specific parser branch
- the wrapper job / package model already carries named public inputs
- the outer statement contract only mirrors ordered verifier public inputs, so
  a four-input Semaphore statement fits without changing the outer circuit

So the main risk is not architectural incompatibility.

The main risk is still operational:

- larger setup artifacts
- larger persisted traces
- more finalize memory pressure
- longer prove wall-clock

## Chunk Size Guidance

Current status:

- `13` is the operationally recommended row-chunk exponent for
  `circom_multiplier2`
- that recommendation should be used as the starting point for Semaphore
- it is not yet a measured Semaphore-specific optimum

Interpretation:

- if Semaphore finishes with `13`, then `13` becomes the first stable baseline
- if Semaphore still hits OOM, try smaller exponents
- if Semaphore finishes comfortably, only then consider larger exponents for
  throughput exploration

Do not assume in advance that a larger chunk size is a win.

## Expected Follow-up Risk

Semaphore is a larger circuit than `circom_multiplier2`, so more direct-lane
performance work may still be required after the first operational attempt.

Most likely follow-up themes:

- `prove-finalize` memory pressure
- `multi_open` pressure if the run reaches openings
- setup/trace artifact size discipline
- chunk-size tuning after the first real result

Treat this playbook as "ready to run", not as a promise that the first run will
be cheap or effortless.

## First Measured Split-Prove Baseline

The repository now has one first measured split direct-lane baseline for the
committed Semaphore fixture on the recommended BLS12-381 host lane.

Measured commands:

- `execute-wrapper-direct-prove-trace`
- `execute-wrapper-direct-prove-finalize`

Host/backend:

- `midnight-direct-halo2-outer-backend-bls12-host`

Chunk setting used for finalize:

- `--h-poly-row-chunk-size 13`

Measured wall-clock:

- `prove-trace`: `507734 ms`
- `prove-finalize`: `751588 ms`
- total split `prove`: `1259322 ms`

Converted time:

- `prove-trace`: about `8.46 min`
- `prove-finalize`: about `12.53 min`
- total split `prove`: about `20.99 min`

Useful interpretation:

- the current direct Semaphore run is much closer to the
  `circom_multiplier2` direct-lane cost than one might expect from the inner
  circuit size alone
- the largest cost is still in `prove-finalize`, but `prove-trace` remains a
  substantial part of the total wall-clock
- this baseline should be treated as the current first operational reference,
  not as a permanent performance target
