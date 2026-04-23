# Groth16 BN254 Optimization Summary

This note consolidates the narrow Groth16 BN254 verifier optimizations landed so
far, using the repository's profiling workflow plus the implementation history
recorded during development.

Scope:

- narrow Groth16 BN254 verifier slice only
- layout / structural cost only
- no production-performance claims

All numbers below are layout metrics from `wrapper-cli profile-layout` plus the
saved before/after snapshots collected while each optimization phase landed.

## Current Snapshot

Current commands:

```bash
cargo run -p wrapper-cli -- profile-layout --family blocks
cargo run -p wrapper-cli -- profile-layout --family groth16
cargo run -p wrapper-cli -- profile-layout --family pairing-terms
cargo run -p wrapper-cli -- profile-layout --family public-inputs
```

## Current Tables

## `blocks`

| id | rows | k |
| --- | ---: | ---: |
| `bn254_miller_loop_narrow` | 503854 | 19 |
| `bn254_final_exponentiation_easy_part` | 13884 | 14 |
| `bn254_final_exponentiation_hard_part` | 690782 | 20 |
| `bn254_final_exponentiation` | 705596 | 20 |
| `bn254_pairing_check_groth16_style` | 2170266 | 22 |
| `bn254_pairing_check_sample_2_terms` | 1873660 | 21 |

## `groth16`

| id | rows | k |
| --- | ---: | ---: |
| `groth16_fixture_verifier_total` | 2170186 | 22 |
| `groth16_fixture_vk_x_accumulator` | 319 | 9 |
| `groth16_pairing_check_proxy_4_terms` | 2170266 | 22 |

## `pairing-terms`

| term count | rows | k |
| ---: | ---: | ---: |
| 1 | 1415916 | 21 |
| 2 | 1667366 | 21 |
| 3 | 1918816 | 21 |
| 4 | 2170266 | 22 |

## `public-inputs`

| public inputs | rows | k |
| ---: | ---: | ---: |
| 1 | 319 | 9 |
| 2 | 590 | 10 |
| 4 | 1132 | 11 |
| 8 | 2216 | 12 |
| 16 | 4384 | 13 |

## Completed Phases

## 1. Interleaved Multi-Miller Loop

What changed:

- replaced "full Miller loop per term, then multiply" with a single global
  schedule traversal over all terms
- shared the global accumulator square on each `Double`
- preserved exact schedule ordering

Why it was valid:

- same `Bn254MillerSchedule::bn254()`
- same line-consumption order
- same single final exponentiation at the end

Representative movement:

| metric | before | after | delta | k before | k after |
| --- | ---: | ---: | ---: | ---: | ---: |
| 4-term pairing proxy | 4074056 | 3298632 | -775424 | 22 | 22 |
| Groth16 fixture verifier total | 4073976 | 3298552 | -775424 | 22 | 22 |

What remained unchanged:

- final exponentiation algorithm
- verifier relation
- variable vs constant G2 treatment

## 2. Final Exponentiation Audit

What changed:

- no semantic rewrite
- code-level decomposition into easy part / hard part
- profiling split exposed the dominant block precisely

Why it was valid:

- audit-only decomposition
- same overall `final_exponentiation(...)` result

Metrics:

- total block stayed on the pre-optimization baseline during the audit pass
- the important new information was the split:
  - easy part: `13884` rows
  - hard part: `1190996` rows
  - total: `1215080` rows

What remained unchanged:

- final exponentiation result
- pairing-check semantics

## 3. `exp_by_neg_x` Improvement

What changed:

- replaced the generic square-and-multiply lane inside `exp_by_neg_x(...)`
  with a fixed BN254-specific chain
- later unified the host/reference and circuit-side chain metadata into one
  shared source of truth

Why it was valid:

- same exponent `x = 4965661367192848881`
- same conjugation / `unitary_inverse` semantics
- chain shape became explicit and auditable

Measured effect:

- the repository history preserved the operation-count win clearly
- per call, the chain moved from:
  - `62` squares + `27` muls
  to:
  - `63` squares + `16` muls

Standalone row snapshots for this sub-phase were not preserved separately from
the later final-exponentiation work. In the current history, the measurable
final-exponentiation row drop is best read as the combined effect of:

- fixed `exp_by_neg_x(...)` chain work
- subsequent cyclotomic-squaring work

## 4. Cyclotomic Squaring In The Hard Part

What changed:

- the hard part switched its repeated square blocks to cyclotomic squaring
  because those values live in the relevant subgroup domain

Why it was valid:

- the hard part starts from the easy-part output and remains inside the intended
  subgroup structure for those repeated square blocks
- the old and new paths were checked against host/reference decomposition

Representative movement:

| metric | before | after | delta | k before | k after |
| --- | ---: | ---: | ---: | ---: | ---: |
| `bn254_final_exponentiation` | 1215080 | 705596 | -509484 | 21 | 20 |
| `bn254_final_exponentiation_hard_part` | 1190996 | 690782 | -500214 | 21 | 20 |
| `bn254_pairing_check_sample_2_terms` | 2383144 | 1873660 | -509484 | 22 | 21 |

What remained unchanged:

- easy-part cost (`13884` rows)
- Miller loop cost

## 5. Prepared Constant G2 Terms

What changed:

- `beta_g2`, `gamma_g2`, and `delta_g2` now precompute Miller-step line
  coefficients off-circuit
- inside the circuit those three terms consume prepared lines directly
- `proof.b` stays on the variable G2 path

Why it was valid:

- those three G2 points are verifier-key constants
- schedule alignment is validated against `Bn254MillerSchedule::bn254()`
- no silent fallback path exists

Representative movement:

| metric | before | after | delta | k before | k after |
| --- | ---: | ---: | ---: | ---: | ---: |
| 4-term pairing proxy | 2789148 | 2170266 | -618882 | 22 | 22 |
| Groth16 fixture verifier total | 2789068 | 2170186 | -618882 | 22 | 22 |

What remained unchanged:

- Miller loop single-term primitive
- final exponentiation block
- variable `proof.b` handling

Scope note:

- this optimization applies only to constant verifier-key G2 terms
- it does not help the variable proof term

## 6. IC Accumulator Optimization In The Current Repo Shape

What changed:

- replaced the in-circuit `mul_by_scalar_constant + add` per public input with a
  host-parameterized precomputation of `public_input_i * IC_i`
- the circuit now assigns the precomputed scaled G1 point and only adds it

Why it was valid:

- in the current repo shape, `public_inputs` are still supplied as host values
  to circuit construction
- this preserves exact verifier semantics for the current narrow boundary

Representative movement:

| public inputs | before | after | delta | k before | k after |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 680 | 319 | -361 | 10 | 9 |
| 2 | 1312 | 590 | -722 | 11 | 10 |
| 4 | 2576 | 1132 | -1444 | 12 | 11 |
| 8 | 5104 | 2216 | -2888 | 13 | 12 |
| 16 | 10160 | 4384 | -5776 | 14 | 13 |

Per-input slope:

- before: about `+632` rows / input
- after: about `+271` rows / input
- improvement: about `57%` lower slope

What remained unchanged:

- Groth16 fixture verifier total stayed effectively unchanged at current scale
  because the verifier is still dominated by the pairing core

Scope note:

- this is a host-parameterized precomputation in the current repo shape
- it is not yet a true in-circuit variable-scalar fixed-base MSM

## Interpretation

Initial dominant cost:

- pairing core, especially multi-term Miller work and final exponentiation

Current dominant cost:

- still the pairing core
- specifically:
  - Groth16-shaped 4-term pairing check: `2170266` rows
  - final exponentiation hard part alone: `690782` rows
  - Miller loop single-term primitive: `503854` rows

Largest wins so far:

1. interleaved multi-Miller loop
2. final exponentiation improvements, especially in the hard part
3. prepared constant G2 terms for VK terms

What now looks cheap relative to the pairing core:

- the IC accumulator (`319` rows on the canonical 1-input fixture)

## Scope Notes

- Prepared-G2 optimization only applies to constant verifier-key G2 terms.
- The current IC optimization is a host-parameterized precomputation path for
  the repo’s current verifier boundary, not a general in-circuit fixed-base MSM.
- The lower-level `bn254_pairing_check_sample_2_terms` block remains useful as a
  primitive pairing sample, but the Groth16-relevant optimized pairing-core
  block is now `bn254_pairing_check_groth16_style`.

## Ranked Future Work

1. Optimize the remaining variable G2 proof term path.
   - Constant VK terms are already prepared; the proof term is the obvious
     remaining variable-side pairing bottleneck.

2. Further reduce final exponentiation hard-part cost.
   - It is still one of the largest standalone blocks in the verifier.

3. If the verifier boundary changes in the future, replace host-parameterized
   IC precomputation with a true in-circuit fixed-base variable-scalar path.

4. Add more verifier-shaped block snapshots where lower-level primitive samples
   still underrepresent the optimized verifier structure.
