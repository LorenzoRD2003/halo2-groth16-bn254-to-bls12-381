# RISC Zero STARK Verify PoC Fixture

This fixture captures the real Groth16 BN254 verification key exported from the
official RISC Zero `stark_verify_final.zkey` artifact for the Circom
`stark_verify` circuit, and includes a local reproduction script for
materializing a matching `input.json` / `proof.json` / `public.json` tuple from
the sibling `risc0` repository.

Scope:

- Upstream repo: `risc0/risc0`
- Upstream circuit: `compact_proof/groth16/stark_verify.circom`
- Local artifact source:
  `/tmp/risc0-stark2snark-run/stark_verify_final.zkey`
- Export command:
  `snarkjs zkey export verificationkey stark_verify_final.zkey stark_verification_key.json`

Committed artifacts:

- `verification_key.json`
- `public_input_names.json`

Generated artifacts after running `./generate.sh`:

- `proof.json`
- `public.json`

Public input order:

1. `control_root_0`
2. `control_root_1`
3. `claim_digest_0`
4. `claim_digest_1`
5. `bn254_control_id`

Reproduction:

```bash
./generate.sh
```

Prerequisites:

- a sibling clone of `risc0` at `../risc0`, or
  `RISC0_REPO=/absolute/path/to/risc0`
- `cargo`
- `curl`
- `docker`
- `circom`
- network access, because the script may need to fetch upstream RISC Zero
  build artifacts and toolchains
- the RISC Zero guest Rust toolchain installed through `rzup`
- a compatible `r0vm` available either on `PATH` or through
  `RISC0_SERVER_PATH`

Recommended one-time setup from the sibling `risc0` checkout:

```bash
cargo run --manifest-path ../risc0/rzup/Cargo.toml -- install rust
```

Compatibility note:

- the `r0vm` binary used here should come from the same `risc0` code line as
  the sibling checkout
- if it is not on `PATH`, set
  `RISC0_SERVER_PATH=/absolute/path/to/r0vm` before running `./generate.sh`

The script:

1. locates the sibling `risc0` repository, defaulting to `../risc0`
2. builds a temporary Rust harness against `risc0-zkvm` and
   `hello-world-methods`
3. proves a tiny zkVM execution directly to a `Groth16Receipt` through `r0vm`
4. decodes the returned Groth16 seal into Circom/snarkjs-style `proof.json`
5. derives the ordered 5-element `public.json`
6. verifies the resulting tuple with `snarkjs groth16 verify`

Notes:

- The committed `verification_key.json` is normalized to `curve = "bn254"` for
  readability, but the current loader already accepts both `bn128` and
  `bn254`.
- The official verification key is committed now because it is stable and comes
  directly from the published `.zkey`.
- The matching `proof.json` / `public.json` pair is generated locally because
  it depends on running the upstream zkVM proving and shrink-wrap pipeline.
- The real circuit interface is still confirmed:
  - private witness input: `iop[25749]`
  - public outputs: `5`
  - Groth16 public-input count: `5`
- The public-input semantics come from
  `risc0/groth16/src/verifier.rs`, where the verifier prepares the scalar
  vector as:
  `[control_root halves, claim_digest halves, bn254_control_id]`.
