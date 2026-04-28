# ZK Email Header-Only Groth16 BN254 Fixture

This fixture captures a real Circom/snarkjs Groth16 BN254 proof for a minimal
ZK Email integration target.

Scope:

- Upstream repo: `zkemail/zk-email-verify`
- Upstream circuit family: `EmailVerifier`
- Local wrapper circuit: `email_verifier_header_only.circom`
- Goal: verify DKIM header signature only
- Body hash check: disabled
- Header masking: disabled
- Body masking: disabled
- Soft line break removal: disabled

Chosen template parameters:

- `maxHeadersLength = 576`
- `maxBodyLength = 64`
- `n = 121`
- `k = 17`
- `ignoreBodyHashCheck = 1`
- `enableHeaderMasking = 0`
- `enableBodyMasking = 0`
- `removeSoftLineBreaks = 0`

Committed artifacts:

- `source_email.eml`
- `input.json`
- `proof.json`
- `public.json`
- `verification_key.json`
- `metadata.json`

Reproduction:

```bash
./generate.sh
```

The script:

1. creates a temporary npm workspace
2. installs pinned published ZK Email packages
3. generates circuit inputs from `source_email.eml`
4. compiles the local wrapper circuit with `circom --O0`
5. reuses or creates the cached `~/.cache/snarkjs/powersOfTau28_hez_final_21.ptau`
6. runs Groth16 setup / witness / prove / verify with pinned `snarkjs` `0.7.6`

Notes:

- The input-generation step depends on DKIM public-key resolution through the
  upstream helper path, with fallback to the ZK Email DNS archive enabled.
- The script uses pinned published packages `@zk-email/helpers` and
  `@zk-email/circuits` instead of the full upstream monorepo install, because
  that path is lighter and more reproducible in this repository.
- The script pins `snarkjs` `0.7.6` explicitly so it does not accidentally pick
  up the browser-oriented `snarkjs` variant pulled in through ZK Email
  dependencies.
- The cached `ptau` used by this fixture is a local development artifact. It
  is acceptable for this repository's integration fixture work, but it is not a
  production ceremony artifact.
- The public-signal order is recorded in `metadata.json` from the actual
  produced `public.json`; no Rust-side semantic naming is required for this
  fixture.
