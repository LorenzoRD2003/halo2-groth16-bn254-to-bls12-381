# Semaphore Groth16 BN254 Fixture

This fixture captures a real Semaphore proof in the standard `snarkjs`
artifact shape used by the current narrow BN254 verifier path.

Source shape:

- Circuit family: Semaphore
- Circuit parameter: Merkle tree depth `10`
- Curve label emitted by `snarkjs`: `bn128` (the BN254 family used by this repo)
- Proof origin: the known-good Semaphore proof embedded in
  `semaphore-rs/src/proof.rs:test_semaphore_js_proof`
- Verification key origin: exported from the official Semaphore depth-10
  `.zkey` artifact published at
  `https://snark-artifacts.pse.dev/semaphore/4.13.0/semaphore-10.zkey`

Committed artifacts:

- `verification_key.json`
- `proof.json`
- `public.json`

Public input order:

1. Merkle root
2. Nullifier
3. Hashed message
4. Hashed scope

Raw statement values behind this fixture:

- Merkle root:
  `4990292586352433503726012711155167179034286198473030768981544541070532815155`
- Nullifier:
  `17540473064543782218297133630279824063352907908315494138425986188962403570231`
- Raw message:
  `32745724963520510550185023804391900974863477733501474067656557556163468591104`
- Raw scope:
  `37717653415819232215590989865455204849443869931268328771929128739472152723456`

The public `message` and `scope` values committed in `public.json` are the
Semaphore hashes used by the verifier path, not the raw 32-byte values above.

Reproduction:

```bash
./generate.sh
```

The script downloads the official depth-10 `.zkey`, rewrites the known-good
proof/public artifacts, exports the verification key, and verifies the tuple
with `snarkjs groth16 verify`.
