# ZK Email Integration Plan

## Purpose

This document proposes a concrete first integration track for a larger Circom-origin circuit, using ZK Email as the first target.

The immediate goal is:

- to integrate a real Circom-origin proof artifact set into this repository in the same operational style as the current Semaphore fixture track
- to validate that the existing wrapper planning and direct outer proving lane can ingest that artifact set
- to keep the long-term target aligned with the BLS12-381 outer proving lane and Aiken verifier generation already described in [docs/plutus-aiken-integration-plan.md](./plutus-aiken-integration-plan.md)

The immediate goal is not:

- to re-architect the repo around a non-`snarkjs` inner proof format yet
- to convert the product goal into “Groth16 forever”
- to implement the full integration in this document

Important constraint:

- the current inner-proof ingestion path in this repository is still the narrow `snarkjs` Groth16 BN254 path described in [crates/wrapper-backends/src/snarkjs.rs](../crates/wrapper-backends/src/snarkjs.rs), [crates/wrapper-backends/src/groth16.rs](../crates/wrapper-backends/src/groth16.rs), and [docs/plutus-aiken-integration-plan.md](./plutus-aiken-integration-plan.md)

That means the first ZK Email milestone should be treated as:

- a real Circom-origin integration study that reuses the current Semaphore-style artifact path
- not yet the final proof-system answer for the long-term “Halo2/PLONKish over BLS12-381 with Aiken export” product

Integration policy for this track:

- do not add a ZK Email-specific verifier path in Rust
- do not add ZK Email-specific backend semantics
- do not add application-specific parsing logic beyond optional public-input names for CLI/profiling convenience
- keep the integration as fixture-driven as the current Semaphore track

## 1. Existing Semaphore Integration

### What exists today

There are two distinct Semaphore-related layers in this repository:

- an upstream-style Circom package snapshot under [circuits/](../circuits/)
- a committed wrapper-fixture lane under [crates/wrapper-tests/fixtures/groth16/semaphore/](../crates/wrapper-tests/fixtures/groth16/semaphore/)

Those serve different purposes and we should reuse both patterns selectively.

### Semaphore circuit, build config, and circuit-level tests

The vendored Circom package is wired like this:

- The main circuit template is `Semaphore(MAX_DEPTH)` in [circuits/src/semaphore.circom](../circuits/src/semaphore.circom).
- The Circomkit circuit registry names the build target and its public inputs in [circuits/circuits.json](../circuits/circuits.json).
- The Circomkit build/setup configuration, include paths, protocol, and ptau/build directories are in [circuits/circomkit.json](../circuits/circomkit.json).
- The package scripts exposing compile/setup/test are in [circuits/package.json](../circuits/package.json).
- Circuit-level witness tests are in [circuits/tests/semaphore.test.ts](../circuits/tests/semaphore.test.ts).

The pattern here is:

1. keep the upstream-ish circuit source and its local build/test harness together
2. expose a deterministic circuit name/template/parameter tuple
3. validate the Circom circuit before worrying about wrapper ingestion

For ZK Email, this same pattern suggests we should keep any exploratory Circom wrapper circuit, input generator, and reproduction script together in a dedicated fixture directory instead of spreading them across Rust crates.

### Committed Semaphore fixture, proof generation, and verification artifacts

The committed wrapper-facing Semaphore fixture is wired like this:

- Fixture documentation and artifact provenance are in [crates/wrapper-tests/fixtures/groth16/semaphore/README.md](../crates/wrapper-tests/fixtures/groth16/semaphore/README.md).
- The reproduction script is [crates/wrapper-tests/fixtures/groth16/semaphore/generate.sh](../crates/wrapper-tests/fixtures/groth16/semaphore/generate.sh).
- The committed artifacts are `proof.json`, `public.json`, and `verification_key.json` in that same fixture directory.

The pattern here is:

1. commit a real proof tuple
2. document its provenance
3. make regeneration a single script
4. keep the artifact shape exactly in the `snarkjs` Groth16 JSON format expected by the current parser

For ZK Email, this is the most important pattern to reuse.

### Semaphore fixture loading, naming, planning, and tests

The Rust-side fixture wiring is:

- Public-input names are defined in [crates/wrapper-tests/src/lib.rs](../crates/wrapper-tests/src/lib.rs).
- The fixture enum and bundle loader are also in [crates/wrapper-tests/src/lib.rs](../crates/wrapper-tests/src/lib.rs).
- Parser / named-public-input / job-planning / package-building tests live in [crates/wrapper-backends/src/groth16.rs](../crates/wrapper-backends/src/groth16.rs).
- End-to-end host verification, mutated-public-input rejection, named-input checks, and outer-package checks live in [crates/wrapper-tests/src/lib.rs](../crates/wrapper-tests/src/lib.rs).
- CLI profiling rows for Semaphore live in [crates/wrapper-cli/src/main.rs](../crates/wrapper-cli/src/main.rs).
- Outer benchmark hooks for Semaphore live in [crates/wrapper-tests/benches/outer/mod.rs](../crates/wrapper-tests/benches/outer/mod.rs).

The reusable Rust flow is:

```text
proof.json + public.json + verification_key.json
  -> parse_snarkjs_groth16_bn254_bundle_with_names(...)
  -> Groth16Bn254ArtifactBundle
  -> WrapperJob
  -> WrapperExecutionPackage
  -> MidnightDirectOuterBackend{Bn254Host|Bls12Host}
```

This flow is implemented across:

- [crates/wrapper-backends/src/snarkjs.rs](../crates/wrapper-backends/src/snarkjs.rs)
- [crates/wrapper-backends/src/groth16.rs](../crates/wrapper-backends/src/groth16.rs)
- [crates/wrapper-backends/src/outer/direct/mod.rs](../crates/wrapper-backends/src/outer/direct/mod.rs)
- [crates/wrapper-backends/src/outer/direct/adaptation.rs](../crates/wrapper-backends/src/outer/direct/adaptation.rs)
- [crates/wrapper-cli/src/main.rs](../crates/wrapper-cli/src/main.rs)

### Exact Semaphore pattern to reuse for `zk-email`

For a new `zk-email` integration, the exact reusable pattern is:

1. Add a dedicated committed fixture directory under `crates/wrapper-tests/fixtures/groth16/<slug>/`.
2. Put `README.md`, `generate.sh`, `proof.json`, `public.json`, and `verification_key.json` there.
3. Add any small support files needed to regenerate those artifacts beside the fixture, not in the Rust crates.
4. Reuse the generic `snarkjs` loader and generic wrapper planning/execution flow exactly as-is.
5. Only add names for ordered public inputs if they are useful for CLI inspection or profiling.
6. Keep end-to-end acceptance/rejection coverage generic: parse fixture, verify fixture, mutate public input, expect rejection.
7. Add profiling and benchmarks only after the fixture is stable, and only if the fixture is small enough to justify always-run coverage.

In other words, the desired shape is:

- ZK Email-specific material lives in the fixture directory
- generic Groth16/wrapper code continues to treat it as “just another artifact bundle”
- the repo should not grow a dedicated `zk_email.rs`, dedicated backend adapter, or dedicated application semantics layer for this track

## 2. ZK Email Circuit Inspection

### Upstream circuit and package

Primary upstream sources:

- Circuit template: [`packages/circuits/email-verifier.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/email-verifier.circom)
- Circuit package README: [`packages/circuits/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/README.md)
- Input generation helpers: [`packages/helpers/src/input-generators.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/helpers/src/input-generators.ts)
- Helper README: [`packages/helpers/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/packages/helpers/README.md)
- DKIM verification helper: [`packages/helpers/src/dkim/index.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/helpers/src/dkim/index.ts)
- Usage guide: [`docs/zk-email-docs/UsageGuide/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/docs/zk-email-docs/UsageGuide/README.md)

### Template parameters and dependencies

`EmailVerifier` is declared as:

```text
EmailVerifier(
  maxHeadersLength,
  maxBodyLength,
  n,
  k,
  ignoreBodyHashCheck,
  enableHeaderMasking,
  enableBodyMasking,
  removeSoftLineBreaks
)
```

Source: [`packages/circuits/email-verifier.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/email-verifier.circom)

Direct dependencies of this circuit are:

- `circomlib/circuits/bitify.circom`
- `circomlib/circuits/poseidon.circom`
- `@zk-email/zk-regex-circom/circuits/common/body_hash_regex.circom`
- local helpers for base64, RSA, SHA, array/regex/hash/bytes utilities, and quoted-printable soft-line-break removal

Sources:

- [`packages/circuits/email-verifier.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/email-verifier.circom)
- [`packages/circuits/package.json`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/package.json)

### Build assumptions

The circuit assumes:

- `maxHeadersLength % 64 == 0`
- `maxBodyLength % 64 == 0`
- `n * k > 2048`
- `n < 255 / 2`
- recommended RSA chunking is `n = 121`, `k = 17`

Sources:

- [`packages/circuits/email-verifier.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/email-verifier.circom)
- [`packages/circuits/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/README.md)

The upstream usage guide also recommends compiling with `--O0` for safety against unintended simplification and choosing ptau size after checking `snarkjs r1cs info`. Source: [`docs/zk-email-docs/UsageGuide/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/docs/zk-email-docs/UsageGuide/README.md).

### Inputs, witnesses, and public outputs

Base private inputs to `EmailVerifier` are:

- `emailHeader[maxHeadersLength]`
- `emailHeaderLength`
- `pubkey[k]`
- `signature[k]`

If `ignoreBodyHashCheck != 1`, it also requires:

- `bodyHashIndex`
- `precomputedSHA[32]`
- `emailBody[maxBodyLength]`
- `emailBodyLength`

If `enableHeaderMasking == 1`, it also requires:

- `headerMask[maxHeadersLength]`

If `removeSoftLineBreaks == 1`, it also requires:

- `decodedEmailBodyIn[maxBodyLength]`

If `enableBodyMasking == 1`, it also requires:

- `bodyMask[maxBodyLength]`

Declared outputs include:

- `pubkeyHash`
- `shaHi`
- `shaLo`
- optional `maskedHeader[maxHeadersLength]`
- optional `maskedBody[maxBodyLength]`

Sources:

- [`packages/circuits/email-verifier.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/email-verifier.circom)
- [`packages/circuits/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/README.md)

Important integration note:

- the upstream test wrappers expose `pubkey` as a public input with `component main { public [ pubkey ] } = EmailVerifier(...)`, but that is a test convenience, not a required public API

Source: [`packages/circuits/tests/test-circuits/email-verifier-no-body-test.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/tests/test-circuits/email-verifier-no-body-test.circom)

For this repository, we should not blindly copy that public-input choice. A better first wrapper is to keep `pubkey` private and expose only the default output commitments unless we discover a product need for more public data.

### DKIM / RSA verification path

The circuit verifies:

- DKIM-style signed headers by SHA-256 hashing the padded header
- RSA signatures using `RSAVerifier65537`
- only the `rsa-sha256` algorithm

Sources:

- [`packages/circuits/email-verifier.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/email-verifier.circom)
- [`packages/circuits/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/README.md)

The upstream helper path resolves DKIM public keys off-circuit through DNS-over-HTTP, with optional fallback to the ZK Email DNS archive, and can apply sanitization retries before returning a `DKIMVerificationResult`. Sources:

- [`packages/helpers/src/dkim/index.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/helpers/src/dkim/index.ts)
- [`packages/helpers/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/packages/helpers/README.md)

This means witness generation is not purely “feed JSON into Circom”. It depends on:

- a raw `.eml`
- deterministic DKIM parsing
- deterministic DNS/public-key resolution or a pinned local fixture path

### SHA / body-hash / body preprocessing path

When body-hash checking is enabled, `EmailVerifier`:

- extracts `bh=` from the `DKIM-Signature` header using `BodyHashRegex`
- base64-decodes the header body-hash value
- computes a partial SHA-256 of the body using `precomputedSHA`
- checks that the computed body hash matches the header body hash

Sources:

- [`packages/circuits/email-verifier.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/email-verifier.circom)
- [`packages/helpers/src/input-generators.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/helpers/src/input-generators.ts)
- [`packages/helpers/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/packages/helpers/README.md)

If `removeSoftLineBreaks == 1`, the helper path rewrites quoted-printable soft line breaks and can adjust the SHA precompute selector accordingly. Sources:

- [`packages/circuits/email-verifier.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/email-verifier.circom)
- [`packages/helpers/src/input-generators.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/helpers/src/input-generators.ts)
- [`packages/circuits/tests/email-verifier-with-soft-line-breaks.test.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/tests/email-verifier-with-soft-line-breaks.test.ts)
- [`packages/circuits/tests/email-verifier-with-qp-encoded-sha-precompute-selector.test.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/tests/email-verifier-with-qp-encoded-sha-precompute-selector.test.ts)

### Masking and regex-related features

Optional masking features:

- `enableHeaderMasking` exposes `maskedHeader[maxHeadersLength]`
- `enableBodyMasking` exposes `maskedBody[maxBodyLength]`

Sources:

- [`packages/circuits/email-verifier.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/email-verifier.circom)
- [`packages/circuits/tests/email-verifier-with-header-mask.test.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/tests/email-verifier-with-header-mask.test.ts)
- [`packages/circuits/tests/email-verifier-with-body-mask.test.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/tests/email-verifier-with-body-mask.test.ts)

Important scope distinction:

- `EmailVerifier` itself only includes a regex use for extracting the DKIM `bh=` value
- application-specific body/header regex constraints are expected to be added by a wrapper circuit around `EmailVerifier`

Source: [`docs/zk-email-docs/UsageGuide/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/docs/zk-email-docs/UsageGuide/README.md)

So if we start from `packages/circuits/email-verifier.circom` directly, a “first integration” should be treated as DKIM-signature verification and output commitment plumbing, not yet a full application-specific regex proof.

### Smaller first configuration

Upstream already includes a smaller “no body check” test circuit:

```text
EmailVerifier(640, 768, 121, 17, 1, 0, 0, 0)
```

and a passing test that generates inputs with:

- `maxHeadersLength: 640`
- `maxBodyLength: 768`
- `ignoreBodyHashCheck: true`

Sources:

- [`packages/circuits/tests/test-circuits/email-verifier-no-body-test.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/tests/test-circuits/email-verifier-no-body-test.circom)
- [`packages/circuits/tests/email-verifier-no-body.test.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/tests/email-verifier-no-body.test.ts)

This is the best starting point for our first integration milestone.

## 3. ZK Email vs Semaphore

### What is reusable

The following is directly reusable from the current Semaphore integration:

- the committed fixture layout under `crates/wrapper-tests/fixtures/groth16/<slug>/`
- the `generate.sh` + `README.md` provenance pattern
- the `snarkjs` parser path in [crates/wrapper-backends/src/snarkjs.rs](../crates/wrapper-backends/src/snarkjs.rs)
- named-public-input loading via `parse_snarkjs_groth16_bn254_bundle_with_names(...)` in [crates/wrapper-backends/src/snarkjs.rs](../crates/wrapper-backends/src/snarkjs.rs)
- bundle -> job -> package -> direct outer backend flow in [crates/wrapper-backends/src/groth16.rs](../crates/wrapper-backends/src/groth16.rs) and [crates/wrapper-backends/src/outer/direct/adaptation.rs](../crates/wrapper-backends/src/outer/direct/adaptation.rs)
- end-to-end acceptance / rejection tests in [crates/wrapper-tests/src/lib.rs](../crates/wrapper-tests/src/lib.rs)
- BLS12-381 outer execution through `--backend midnight-bls12381-host` in [crates/wrapper-cli/src/main.rs](../crates/wrapper-cli/src/main.rs)

### What is materially different

ZK Email differs from Semaphore in several important ways.

#### Circuit complexity

Semaphore is a small application-specific circuit with a stable public input shape declared in [circuits/circuits.json](../circuits/circuits.json) and exercised in [circuits/tests/semaphore.test.ts](../circuits/tests/semaphore.test.ts).

ZK Email’s `EmailVerifier` is larger and more generic. Inline comments in the circuit show major blocks of roughly:

- header SHA-256: `506,670` constraints
- RSA verify: `149,251` constraints
- body-hash regex: `617,597` constraints
- body SHA-256: `760,142` constraints for `maxBodyLength = 1536`

Source: [`packages/circuits/email-verifier.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/email-verifier.circom)

So:

- the header-only configuration is already likely far larger than Semaphore
- the full body-check configuration is much larger again

#### Witness generation

Semaphore witness data is small, structured, and locally synthetic in [circuits/tests/semaphore.test.ts](../circuits/tests/semaphore.test.ts).

ZK Email witness generation depends on:

- raw email bytes
- DKIM verification
- RSA public key extraction
- optional sanitization
- optional DNS/archive lookup
- optional SHA-precompute selector logic
- optional quoted-printable cleanup logic

Sources:

- [`packages/helpers/src/dkim/index.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/helpers/src/dkim/index.ts)
- [`packages/helpers/src/input-generators.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/helpers/src/input-generators.ts)

#### Public signal shape

Semaphore’s public-input order is easy to name and is already documented in [crates/wrapper-tests/fixtures/groth16/semaphore/README.md](../crates/wrapper-tests/fixtures/groth16/semaphore/README.md).

ZK Email’s public signals depend on the wrapper `main` circuit we choose:

- exposing `pubkey` publicly creates a large public vector
- enabling masking can expose very large public arrays
- keeping only default outputs public yields a much smaller and cleaner first fixture

Sources:

- [`packages/circuits/email-verifier.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/email-verifier.circom)
- [`packages/circuits/tests/test-circuits/email-verifier-no-body-test.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/tests/test-circuits/email-verifier-no-body-test.circom)

#### Trusted setup and artifact size

Both the current Semaphore lane and the proposed first ZK Email lane would still generate:

- `.r1cs`
- `.wasm`
- `.zkey`
- `verification_key.json`
- `proof.json`
- `public.json`

using Circom + `snarkjs`, because that is the artifact family this repository already ingests. Sources:

- [crates/wrapper-tests/fixtures/groth16/semaphore/generate.sh](../crates/wrapper-tests/fixtures/groth16/semaphore/generate.sh)
- [`docs/zk-email-docs/UsageGuide/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/docs/zk-email-docs/UsageGuide/README.md)

The difference is that ZK Email will need larger setup parameters and likely much slower proving than Semaphore.

#### Test fixture complexity

Semaphore fixture generation only needs a known proof tuple and a known `.zkey` source in [crates/wrapper-tests/fixtures/groth16/semaphore/generate.sh](../crates/wrapper-tests/fixtures/groth16/semaphore/generate.sh).

ZK Email fixture generation needs:

- a pinned upstream repo version
- a pinned `.eml`
- a small wrapper circuit around `EmailVerifier`
- a helper script that emits `input.json`
- local dependency installation for `@zk-email/helpers` and `@zk-email/circuits`

### Current blocker relative to the long-term goal

The biggest architectural blocker is not ZK Email itself. It is this repository’s current ingestion boundary.

Today this repo consumes `snarkjs` Groth16 BN254 inner proofs, then wraps them in the canonical outer circuit. Sources:

- [crates/wrapper-backends/src/snarkjs.rs](../crates/wrapper-backends/src/snarkjs.rs)
- [crates/wrapper-backends/src/groth16.rs](../crates/wrapper-backends/src/groth16.rs)
- [docs/plutus-aiken-integration-plan.md](./plutus-aiken-integration-plan.md)

So a ZK Email Circom integration can answer:

- how a real larger Circom-origin circuit fits the existing wrapper pipeline
- what fixture shape, public inputs, and proving costs look like

But it does not by itself solve:

- the long-term non-Groth16 inner-proof direction

That should stay an explicit later phase, not be hidden inside Phase 1.

## 4. Proposed Integration Plan

### Phase 1: Commit a minimal ZK Email fixture scaffold

#### Goal

Commit one deterministic ZK Email fixture that matches the current repository’s real integration style:

- provenance docs
- one reproduction script
- one committed proof/public/vk tuple
- one clear minimal wrapper circuit

#### Recommended files

Add:

- `crates/wrapper-tests/fixtures/groth16/zk_email_header_only/README.md`
- `crates/wrapper-tests/fixtures/groth16/zk_email_header_only/generate.sh`
- `crates/wrapper-tests/fixtures/groth16/zk_email_header_only/email_verifier_header_only.circom`
- `crates/wrapper-tests/fixtures/groth16/zk_email_header_only/generate_inputs.mjs`
- `crates/wrapper-tests/fixtures/groth16/zk_email_header_only/source_email.eml`
- `crates/wrapper-tests/fixtures/groth16/zk_email_header_only/input.json`
- `crates/wrapper-tests/fixtures/groth16/zk_email_header_only/proof.json`
- `crates/wrapper-tests/fixtures/groth16/zk_email_header_only/public.json`
- `crates/wrapper-tests/fixtures/groth16/zk_email_header_only/verification_key.json`
- `crates/wrapper-tests/fixtures/groth16/zk_email_header_only/metadata.json`

#### Suggested wrapper circuit

The wrapper circuit for Phase 1 should be:

```circom
pragma circom 2.1.6;

include "@zk-email/circuits/email-verifier.circom";

component main = EmailVerifier(640, 768, 121, 17, 1, 0, 0, 0);
```

Why this wrapper:

- it reuses the upstream-tested no-body configuration
- it keeps all inputs private
- it exposes only the default outputs
- it avoids large masked arrays as public signals

Source basis:

- [`packages/circuits/tests/test-circuits/email-verifier-no-body-test.circom`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/tests/test-circuits/email-verifier-no-body-test.circom)
- [`packages/circuits/tests/email-verifier-no-body.test.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/tests/email-verifier-no-body.test.ts)

#### Suggested commands

Inside `generate.sh`, keep the steps reproducible and explicit:

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

git clone --depth 1 https://github.com/zkemail/zk-email-verify "$WORK_DIR/zk-email-verify"
cd "$WORK_DIR/zk-email-verify"
yarn install --immutable

cp "$SCRIPT_DIR/source_email.eml" "$WORK_DIR/source_email.eml"
cp "$SCRIPT_DIR/email_verifier_header_only.circom" "$WORK_DIR/email_verifier_header_only.circom"
cp "$SCRIPT_DIR/generate_inputs.mjs" "$WORK_DIR/generate_inputs.mjs"

node "$WORK_DIR/generate_inputs.mjs" \
  "$WORK_DIR/source_email.eml" \
  "$WORK_DIR/input.json"

circom "$WORK_DIR/email_verifier_header_only.circom" \
  -l "$WORK_DIR/zk-email-verify/node_modules" \
  --r1cs --wasm --sym --O0 \
  -o "$WORK_DIR/build"

snarkjs r1cs info "$WORK_DIR/build/email_verifier_header_only.r1cs"

wget -O "$WORK_DIR/powersOfTau28_hez_final_21.ptau" \
  https://storage.googleapis.com/zkevm/ptau/powersOfTau28_hez_final_21.ptau

snarkjs groth16 setup \
  "$WORK_DIR/build/email_verifier_header_only.r1cs" \
  "$WORK_DIR/powersOfTau28_hez_final_21.ptau" \
  "$WORK_DIR/email_verifier_header_only_0000.zkey"

snarkjs zkey export verificationkey \
  "$WORK_DIR/email_verifier_header_only_0000.zkey" \
  "$WORK_DIR/verification_key.json"

node "$WORK_DIR/build/email_verifier_header_only_js/generate_witness.js" \
  "$WORK_DIR/build/email_verifier_header_only_js/email_verifier_header_only.wasm" \
  "$WORK_DIR/input.json" \
  "$WORK_DIR/witness.wtns"

snarkjs groth16 prove \
  "$WORK_DIR/email_verifier_header_only_0000.zkey" \
  "$WORK_DIR/witness.wtns" \
  "$WORK_DIR/proof.json" \
  "$WORK_DIR/public.json"

snarkjs groth16 verify \
  "$WORK_DIR/verification_key.json" \
  "$WORK_DIR/public.json" \
  "$WORK_DIR/proof.json"
```

Implementation note:

- if `r1cs info` shows the circuit exceeds the `2^21` ceremony size, bump to `powersOfTau28_hez_final_22.ptau` and record that in `README.md`

Source basis for the toolchain:

- [`docs/zk-email-docs/UsageGuide/README.md`](https://github.com/zkemail/zk-email-verify/blob/main/docs/zk-email-docs/UsageGuide/README.md)
- [crates/wrapper-tests/fixtures/groth16/semaphore/generate.sh](../crates/wrapper-tests/fixtures/groth16/semaphore/generate.sh)

#### Expected artifacts

Phase 1 should produce:

- one wrapper circuit file
- one deterministic `input.json`
- one `proof.json`
- one `public.json`
- one `verification_key.json`
- one `metadata.json` containing:
  - upstream repo URL
  - pinned upstream commit hash
  - chosen email file origin
  - chosen `EmailVerifier(...)` parameter tuple
  - whether body hash, masking, or soft-line-break removal were enabled
  - observed public signal count
  - observed public signal order

#### Acceptance tests

Phase 1 is complete when:

1. `generate.sh` reproduces `proof.json`, `public.json`, and `verification_key.json`.
2. `snarkjs groth16 verify` succeeds inside `generate.sh`.
3. `README.md` documents the exact origin of the `.eml` and the upstream repo commit.
4. `metadata.json` records the actual `public.json` ordering instead of guessing it.

### Phase 2: Wire the fixture into the existing generic bundle path

#### Goal

Make the committed ZK Email fixture load through the same Rust bundle path as Semaphore, without introducing a ZK Email-specific code path.

#### Files likely to be modified

Minimal path:

- no Rust code changes at all if we only validate through the existing generic CLI commands against fixture paths

Optional convenience path:

- `crates/wrapper-tests/src/lib.rs`
- `crates/wrapper-backends/src/groth16.rs`

#### Suggested changes

Preferred approach:

- do not add a dedicated `OuterBenchFixture::ZkEmailHeaderOnly` initially
- do not add a dedicated loader branch initially
- validate the fixture through the existing generic CLI/file-path flow in [crates/wrapper-cli/src/main.rs](../crates/wrapper-cli/src/main.rs)

Optional later convenience additions, only if they pay their way:

- add a fixture enum variant for benchmark/profiling reuse
- add a small named-public-input helper if the public signal order is stable and the names materially help debugging
- add narrow generic tests that load the fixture from file bytes, without introducing ZK Email-specific semantics

#### Suggested commands

```bash
cargo run -p wrapper-cli -- inspect-groth16-bundle \
  --id zk-email-header-only \
  --proof crates/wrapper-tests/fixtures/groth16/zk_email_header_only/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/zk_email_header_only/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/zk_email_header_only/verification_key.json

cargo run -p wrapper-cli -- plan-wrapper-job \
  --id zk-email-header-only \
  --proof crates/wrapper-tests/fixtures/groth16/zk_email_header_only/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/zk_email_header_only/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/zk_email_header_only/verification_key.json

cargo run -p wrapper-cli -- export-wrapper-package \
  --id zk-email-header-only \
  --proof crates/wrapper-tests/fixtures/groth16/zk_email_header_only/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/zk_email_header_only/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/zk_email_header_only/verification_key.json
```

#### Expected outputs

- fixture loads into the existing generic `Groth16Bn254ArtifactBundle` path
- `WrapperJob` and `WrapperExecutionPackage` build successfully
- no ZK Email-specific Rust abstraction is required

#### Acceptance tests

Phase 2 is complete when:

1. `inspect-groth16-bundle` succeeds against the committed file paths.
2. `plan-wrapper-job` succeeds against the committed file paths.
3. `export-wrapper-package` succeeds against the committed file paths.
4. We can point to the existing generic loader path as the only Rust path involved.

Optional test coverage can be added later if the fixture becomes a long-lived benchmark/profile case.

### Phase 3: Add end-to-end verification and outer-circuit construction coverage

#### Goal

Prove that the committed ZK Email fixture is accepted by the current host-side verifier and can build the canonical outer wrapper circuit on both host lanes.

#### Files likely to be modified

Minimal path:

- no Rust code changes

Optional later:

- `crates/wrapper-tests/src/lib.rs`
- `crates/wrapper-tests/benches/outer/mod.rs`
- `crates/wrapper-cli/src/main.rs`

#### Suggested commands

```bash
cargo test -p wrapper-tests
cargo run -p wrapper-cli -- inspect-groth16-bundle \
  --id zk-email-header-only \
  --proof crates/wrapper-tests/fixtures/groth16/zk_email_header_only/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/zk_email_header_only/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/zk_email_header_only/verification_key.json
```

If named public inputs are finalized:

```bash
cargo run -p wrapper-cli -- inspect-groth16-bundle \
  --id zk-email-header-only \
  --proof crates/wrapper-tests/fixtures/groth16/zk_email_header_only/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/zk_email_header_only/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/zk_email_header_only/verification_key.json \
  --public-input-name <name-0> \
  --public-input-name <name-1> \
  --public-input-name <name-2>
```

#### Expected outputs

- host-side verification accepts the committed proof tuple
- a one-field mutation in `public_inputs` is rejected
- both BN254-hosted and BLS12-hosted outer circuits can be built from the package

#### Acceptance tests

Phase 3 is complete when the fixture passes the generic end-to-end flow:

- accepted end-to-end host verification
- mutated public input rejected
- placeholder outer bundle materialization works
- direct outer circuit build works on both host lanes

If we later decide the fixture deserves always-run regression coverage comparable to Semaphore, then we can add test helpers in [crates/wrapper-tests/src/lib.rs](../crates/wrapper-tests/src/lib.rs). That should remain a convenience decision, not a semantic requirement.

### Phase 4: Validate the BLS12-381 direct outer lane and Aiken handoff assumptions

#### Goal

Confirm that the new fixture works on the outer lane we actually care about for Aiken generation.

#### Files likely to be modified

Possibly:

- `crates/wrapper-tests/benches/outer/mod.rs`
- `crates/wrapper-cli/src/main.rs`
- `docs/plutus-aiken-integration-plan.md`

#### Suggested commands

```bash
cargo run -p wrapper-cli -- execute-wrapper-direct \
  --id zk-email-header-only \
  --proof crates/wrapper-tests/fixtures/groth16/zk_email_header_only/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/zk_email_header_only/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/zk_email_header_only/verification_key.json \
  --backend midnight-bls12381-host
```

Optional profiling after the fixture is stable:

```bash
cargo run -p wrapper-cli -- profile-layout --family outer
```

#### Expected outputs

- a successful direct setup/prove/verify result on the BLS12-381 host lane
- optionally, a measured outer-profile row if and only if we decide the fixture deserves a first-class profiling slot

#### Acceptance tests

Phase 4 is complete when:

1. the direct BLS12-381 lane succeeds on the committed fixture
2. the produced artifact shape remains compatible with the direct outer artifact contract
3. we know whether the fixture is small enough to include in always-run profiling/bench coverage without turning that profiling path into application-specific infrastructure

Sources:

- [crates/wrapper-backends/src/outer/direct/mod.rs](../crates/wrapper-backends/src/outer/direct/mod.rs)
- [crates/wrapper-cli/src/main.rs](../crates/wrapper-cli/src/main.rs)
- [docs/plutus-aiken-integration-plan.md](./plutus-aiken-integration-plan.md)

### Phase 5: Decide the post-fixture path for the non-Groth16 product goal

#### Goal

Make the architectural decision that the Phase 1 through Phase 4 work deliberately does not make for us:

- do we keep using Circom/Groth16 only as an integration oracle
- or do we need a new inner-proof ingestion path or a Halo2-native reimplementation for the actual product lane

#### Files likely to be modified

- `docs/plutus-aiken-integration-plan.md`
- possibly a new decision doc under `docs/decisions/`

#### Suggested commands

No code command is required for the decision itself. The output should be a written decision backed by the measurements and integration evidence from Phases 1 through 4.

#### Expected outputs

- one explicit architecture decision
- one explicit list of follow-on tasks

#### Acceptance tests

Phase 5 is complete when the team can answer:

1. Is the committed ZK Email fixture only a compatibility study?
2. If yes, what replaces the Groth16 inner proof for production?
3. If no, what product boundary justifies keeping a Groth16 inner proof?

## 5. Recommended First Minimal Target

### Chosen target

The first minimal target should be:

- upstream circuit family: `EmailVerifier`
- wrapper main: `component main = EmailVerifier(640, 768, 121, 17, 1, 0, 0, 0);`
- helper input settings:
  - `maxHeadersLength: 640`
  - `maxBodyLength: 768`
  - `ignoreBodyHashCheck: true`
- no `shaPrecomputeSelector`
- no header masking
- no body masking
- no soft-line-break removal
- fixture email: the upstream `test.eml`-style no-body case already exercised by [`packages/circuits/tests/email-verifier-no-body.test.ts`](https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/tests/email-verifier-no-body.test.ts)

### Why this is the safest first milestone

This configuration is safest because:

- it is already exercised upstream
- it avoids the heaviest body-hash regex and body-SHA logic
- it avoids quoted-printable edge cases
- it avoids giant masked public outputs
- it keeps the witness generator simple
- it still proves the essential ZK Email-specific hard part: DKIM header signature verification over RSA

### Public-input naming recommendation

Do not hardcode public-input names until Phase 1 records the actual `public.json` order.

Expected likely names, if the wrapper keeps only default outputs public, are something close to:

- `pubkey_hash`
- `sha_hi`
- `sha_lo`

But this must be confirmed from the actual compiled proof artifacts, not inferred from source ordering alone.

## 6. Open Questions

The following points are intentionally unresolved and should stay explicit:

1. Exact `public.json` ordering for the Phase 1 wrapper main.
2. Exact constraint count for the chosen header-only wrapper after compilation.
3. Whether `powersOfTau28_hez_final_21.ptau` is sufficient or `22` is required.
4. Whether the upstream `.eml` fixture can be committed directly in this repository or should be copied into a sanitized local variant.
5. Whether the BLS12-381 direct outer lane remains fast enough for always-run CI once the fixture is wired.
6. Whether the long-term product path wants this Circom/Groth16 inner proof only as an integration oracle or as a durable supported input family.

## 7. Recommended Next Task

The next implementation task should be:

> Implement Phase 1 only:
> create `crates/wrapper-tests/fixtures/groth16/zk_email_header_only/`,
> add the minimal wrapper circuit and reproduction script,
> generate and commit the first deterministic proof/public/vk tuple,
> and document the exact observed public signal order in `metadata.json` and `README.md`.
