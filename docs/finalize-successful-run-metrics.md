# Successful `prove-finalize` Run Metrics

This note records the first successful `execute-wrapper-direct-prove-finalize`
run after the recent memory-pressure reductions in `h_poly`,
`build_opening_key`, and the params-cache path.

It is intended to answer three concrete questions:

1. What exactly finished successfully?
2. Where did the time go?
3. What should we optimize next if the goal is lower wall-clock time without
   reintroducing OOM regressions?

## Run Identity

- Date: `2026-05-29`
- Fixture: `circom-multiplier2`
- Command family: `execute-wrapper-direct-prove-finalize`
- Host/backend: `midnight-direct-halo2-outer-backend-bn254-host`
- Run id:
  `pid121289-execute-wrapper-direct-prove-finalize-1780102735754`
- Chunk setting: `--h-poly-row-chunk-size 13`
  - effective row chunk size: `2^13 = 8192`
- Log mode used for this measured success run: `detailed`
- Params cache: `hit`
  - log path:
    `$HOME/.cache/halo2-wrapper/direct-params/midnight-direct-halo2-outer-backend-bn254-host-k21.params`

## Output Artifacts

Produced bundle:

- `artifacts/direct-profile/circom-multiplier2-proof-bundle-chunk13.json`

Supporting persisted artifacts:

- `artifacts/direct-profile/circom-multiplier2-setup.json`
- `artifacts/direct-profile/circom-multiplier2-setup.json.pk`
- `artifacts/direct-profile/circom-multiplier2-trace.bin`

Artifact sizes observed at the time of this run:

- proof bundle JSON: about `17 KiB`
- setup manifest JSON: about `5.4 KiB`
- setup proving-key sidecar: about `3.9 GiB`
- persisted trace: about `9.4 GiB`

## Final Result Snapshot

The finalized proof bundle reports:

- `job_id = circom-multiplier2`
- `backend = midnight-direct-halo2-outer-backend-bn254-host`
- `outer_host = midnight-bn254-host`
- `protocol = halo2-plonkish`
- `curve = bn254`
- `circuit_k = 21`
- `public_input_count = 1`
- `prove_elapsed_ms = 1769025`

Converted wall-clock:

- total `prove-finalize` wall-clock: about `29.48 min`

## Peak Memory

Peak memory reported by the run log:

- `VmHWM = 22941844 kB`
- approx. `21.88 GiB`

This matters for planning because it means the run now completes under the
current `24 GiB` limit with only a modest remaining margin.

## High-Level Timing Breakdown

Percentages below are relative to the total `prove-finalize` wall-clock
(`1769025 ms`).

| Phase | Elapsed ms | Minutes | % of total |
| --- | ---: | ---: | ---: |
| `finalise_proof_from_base_trace` | `1525143` | `25.42` | `86.2%` |
| `compute_h_poly` | `1195429` | `19.92` | `67.6%` |
| `build_h_poly_key` | `90597` | `1.51` | `5.1%` |
| `build_opening_key` | `88718` | `1.48` | `5.0%` |
| `params_cache.read` | `77257` | `1.29` | `4.4%` |
| `construct_vanishing_commitments` | `63245` | `1.05` | `3.6%` |
| `multi_open` | `62405` | `1.04` | `3.5%` |
| `deserialize_prepared_trace` | `58901` | `0.98` | `3.3%` |
| `deserialize_base_pk` | `29134` | `0.49` | `1.6%` |
| `deserialize_opening_polys` | `12181` | `0.20` | `0.7%` |

Immediate takeaway:

- `compute_h_poly` still dominates the run.
- Post-`h_poly` phases are now real but secondary.
- the params cache already removed the earlier `~17 min` params-generation tax;
  the read path is now only `~1.3 min`.

## `h_poly` Breakdown

Percentages in this section are relative to `compute_h_poly`
(`1195429 ms`) unless stated otherwise.

### Coarse `h_poly` hotspots

| Step | Elapsed ms | % of `h_poly` | % of total |
| --- | ---: | ---: | ---: |
| `permutation_constraints` | `641216` | `53.6%` | `36.2%` |
| `custom_fixed_cosets` | `194467` | `16.3%` | `11.0%` |
| total `lookup_cosets` | `183937` | `15.4%` | `10.4%` |
| total `lookup_constraints` | `45415` | `3.8%` | `2.6%` |
| `custom_gates` | `56882` | `4.8%` | `3.2%` |
| `lookup_fixed_cosets` | `56042` | `4.7%` | `3.2%` |
| `permutation_fixed_cosets` | `16711` | `1.4%` | `0.9%` |
| `values_allocation` | `191` | effectively `0%` | effectively `0%` |

### Internal permutation breakdown

Percentages in this table are relative to `permutation_constraints`
(`641216 ms`).

| Permutation substep | Elapsed ms | % of permutation block | % of total |
| --- | ---: | ---: | ---: |
| total `permutation_sigma` | `409951` | `63.9%` | `23.2%` |
| total `permutation_final_accumulation` | `168733` | `26.3%` | `9.5%` |
| total `permutation_set_cosets` | `62110` | `9.7%` | `3.5%` |

Observed shape:

- `permutation_sigma` remains the single biggest hot lane inside `h_poly`
  even after the memory fixes.
- `permutation_final_accumulation` is the second largest permutation cost.
- `permutation_set_cosets` is comparatively small and does not look like the
  first timing target.

### Lookup breakdown

The lookup path is no longer the crash site, but it is not cheap.

Aggregate lookup costs:

- total `lookup_fixed_cosets`: `56042 ms`
- total `lookup_cosets` across `lookup_idx = 0..7`: `183937 ms`
- total `lookup_constraints` across `lookup_idx = 0..7`: `45415 ms`
- aggregate lookup-related time: `285394 ms`

That is:

- about `23.9%` of `h_poly`
- about `16.1%` of total `prove-finalize`

So the lookup lane is now a real second-tier optimization target after
permutation throughput.

## What Changed Relative to The Earlier Failing Runs

This successful run confirms several earlier hypotheses:

1. The run no longer dies inside the first `permutation_sigma` of the first
   permutation set.
2. `build_opening_key` now completes in about `88.7 s`; the previous
   post-`h_poly` OOM moved out of the way once the opening path stopped
   materializing unnecessary permutation cosets.
3. `sparse_fixed_coeffs` is materially cheaper than the earlier
   `sparse_fixed_cosets` approach.
4. The params cache is paying off: the expensive params-generation miss path is
   no longer present in the successful steady-state run.

## Recommended Next Optimizations

The next speed work should be prioritized in this order.

### 1. `permutation_sigma` throughput

Reason:

- it is the largest remaining hot lane inside `h_poly`
- it alone costs about `23.2%` of the total finalize wall-clock

Recommended direction:

- switch the permutation walk from the current effective
  `set -> column -> chunk` shape toward a more cache-friendly
  `set -> chunk -> all columns` shape
- reuse scratch buffers more aggressively within that reordered loop
- reduce repeated per-column per-chunk setup work

Expected benefit:

- lowers the dominant term inside `permutation_constraints`
- should be mostly a time win rather than a memory regression if implemented as
  a true replacement rather than an additive buffer layer

### 2. `custom_fixed_cosets`

Reason:

- about `11.0%` of the total run
- the largest non-permutation substep inside `h_poly`

Recommended direction:

- avoid unnecessary buffer copies when chunk windows do not wrap
- reuse fixed-column scratch storage instead of rebuilding short-lived vectors
  repeatedly
- prefer borrowed views where the current row window is contiguous

Current retained memory-oriented follow-up:

- for the BLS12-hosted direct lane, the next custom-fixed experiment does not
  try to make `custom_fixed_cosets` faster first
- instead, it splits `custom_gates` into smaller gate-local evaluator batches
  so fixed columns are materialized and dropped batch-by-batch
- the goal is to reduce peak memory during the `custom_fixed_cosets` region,
  because the first BLS12 `prove-finalize` run showed that this phase was one
  of the largest visible allocation jumps before the later failure in
  `multi_open`

### 3. lookup coset throughput

Reason:

- aggregate lookup path still costs about `16.1%` of the total run
- the individual `lookup_cosets` steps are consistently `~22-24 s` each across
  eight lookups

Recommended direction:

- apply the same “materialize once, reuse scratch, avoid avoidable copies”
  discipline already used on the permutation path
- only escalate to more invasive domain-helper work if the simpler locality and
  reuse wins prove insufficient

### 4. use `--log-mode efficient` for future timing runs

This successful run used `detailed` mode, which still emits `event=iter`
heartbeats and chunk-level progress noise.

Future throughput measurements should use:

- `--log-mode efficient`

so the timing baselines are not inflated by diagnostic logging overhead.

## Recommended Near-Term Experiment Order

1. Re-run the successful command with `--log-mode efficient` and the same
   chunk size `13`.
2. If it stays stable, compare `13` versus `14`.
3. Only after that, start reworking `permutation_sigma` for chunk-outer
   execution and better scratch reuse.

This order keeps the measurement story clean:

- first separate real prover cost from logging overhead
- then measure chunk-size sensitivity
- only then change the internal hot loop again

## Source Files Most Relevant To The Next Speed Pass

- `patches/midnight-proofs/src/plonk/evaluation.rs`
- `patches/midnight-proofs/src/plonk/mod.rs`
- `patches/midnight-proofs/src/plonk/prover.rs`
- `docs/h-poly-followup-speed-plan.md`
- `docs/plans/0006-finalize-checkpoint-profiling-plan.md`

## 2026-05-30 Follow-up Run After `permutation_sigma` Throughput Work

After changing the permutation lane from the earlier effective
`set -> column -> chunk` scratch-file pattern to an in-memory
`set -> chunk -> all columns` shape, the same direct `prove-finalize` flow was
rerun on the same fixture:

- Date: `2026-05-30`
- Run id:
  `pid271826-execute-wrapper-direct-prove-finalize-1780144528056`
- Host/backend: `midnight-direct-halo2-outer-backend-bn254-host`
- Chunk setting: `--h-poly-row-chunk-size 13`
- effective row chunk size: `8192`
- log mode: `efficient`
- output bundle:
  `artifacts/direct-profile/circom-multiplier2-proof-bundle-chunk13.json`

That bundle now embeds the finalize metrics directly under
`finalize_metrics`.

### Follow-up Run Snapshot

- `prove_elapsed_ms = 1553322`
- `finalise_proof_from_base_trace_ms = 1331145`
- `compute_h_poly_ms = 1011070`
- `build_h_poly_key_ms = 82441`
- `build_opening_key_ms = 88652`
- `construct_vanishing_commitments_ms = 61070`
- `deserialize_opening_polys_ms = 12165`
- `multi_open_ms = 63455`
- `VmHWM = 23331520 kB`

Converted wall-clock:

- total `prove-finalize` wall-clock: about `25.89 min`

### Before / After Comparison

This comparison uses:

- baseline successful run on `2026-05-29`
- follow-up successful run on `2026-05-30`

| Metric | Baseline | Follow-up | Delta | Relative change |
| --- | ---: | ---: | ---: | ---: |
| total `prove-finalize` | `1769025 ms` | `1553322 ms` | `-215703 ms` | `-12.2%` |
| `finalise_proof_from_base_trace` | `1525143 ms` | `1331145 ms` | `-193998 ms` | `-12.7%` |
| `compute_h_poly` | `1195429 ms` | `1011070 ms` | `-184359 ms` | `-15.4%` |
| `permutation_constraints` | `641216 ms` | `479854 ms` | `-161362 ms` | `-25.2%` |
| `build_h_poly_key` | `90597 ms` | `82441 ms` | `-8156 ms` | `-9.0%` |
| `params_cache.read` | `77257 ms` | `69454 ms` | `-7803 ms` | `-10.1%` |
| `deserialize_prepared_trace` | `58901 ms` | `51831 ms` | `-7070 ms` | `-12.0%` |
| `custom_gates` | `56882 ms` | `47944 ms` | `-8938 ms` | `-15.7%` |
| `custom_fixed_cosets` | `194467 ms` | `183833 ms` | `-10634 ms` | `-5.5%` |
| `build_opening_key` | `88718 ms` | `88652 ms` | `-66 ms` | effectively `0%` |
| `deserialize_opening_polys` | `12181 ms` | `12165 ms` | `-16 ms` | effectively `0%` |
| `multi_open` | `62405 ms` | `63455 ms` | `+1050 ms` | `+1.7%` |
| peak `VmHWM` | `22941844 kB` | `23331520 kB` | `+389676 kB` | `+1.7%` |

### Interpretation

This follow-up run confirms that the `permutation_sigma` throughput work was
useful in the real wrapper flow, not just in an isolated micro-benchmark.

Strongest signal:

- `permutation_constraints` improved by about `25.2%`
- total `compute_h_poly` improved by about `15.4%`
- total `prove-finalize` improved by about `12.2%`

That is a meaningful wall-clock win:

- about `3.60 min` saved on the full `prove-finalize`

Tradeoff:

- memory peak increased slightly by about `381 MiB`
- the run still stayed under the current `24 GiB` limit

So the throughput win appears worthwhile under the current memory budget.

### Fast Feedback Benchmark For This Optimization

To avoid rerunning the entire wrapper flow for every permutation-lane change, a
dedicated Criterion benchmark was added for this phase alone.

Command:

```bash
cargo bench --manifest-path patches/midnight-proofs/Cargo.toml --features bench-internal --bench plonk -- "Permutation constraints only"
```

Current measured output after the optimization:

```text
plonk-permutation-phase/k16/Permutation constraints only
time: [566.34 ms 583.70 ms 597.12 ms]
```

This benchmark is intentionally narrow:

- it prepares one real prover trace
- it then measures only the permutation-constraint portion of `compute_h_poly`
- it is intended as fast relative feedback for future local throughput changes

It should not be treated as a replacement for full `prove-finalize` timing, but
it is now the fastest reproducible check for whether a permutation-lane edit is
moving in the right direction.

## 2026-05-31 Follow-up Run After `lookup coset throughput` Work

After adding scratch-buffer reuse for lookup coset materialization, the same
direct `prove-finalize` flow was rerun on the same fixture:

- Date: `2026-05-31`
- Run id:
  `pid450442-execute-wrapper-direct-prove-finalize-1780152815433`
- Host/backend: `midnight-direct-halo2-outer-backend-bn254-host`
- Chunk setting: `--h-poly-row-chunk-size 13`
- effective row chunk size: `8192`
- log mode: `efficient`
- output bundle:
  `artifacts/direct-profile/circom-multiplier2-proof-bundle-chunk13.json`

### Follow-up Run Snapshot

- `prove_elapsed_ms = 1570809`
- `finalise_proof_from_base_trace_ms = 1338474`
- `compute_h_poly_ms = 1010248`
- `build_h_poly_key_ms = 86405`
- `build_opening_key_ms = 92099`
- `construct_vanishing_commitments_ms = 61946`
- `deserialize_opening_polys_ms = 11679`
- `multi_open_ms = 63451`
- `VmHWM = 22970260 kB`

Converted wall-clock:

- total `prove-finalize` wall-clock: about `26.18 min`

### Before / After Comparison

This comparison uses:

- the successful `permutation_sigma` follow-up run on `2026-05-30`
- the `lookup coset throughput` follow-up run on `2026-05-31`

| Metric | Prior run | Lookup follow-up | Delta | Relative change |
| --- | ---: | ---: | ---: | ---: |
| total `prove-finalize` | `1553322 ms` | `1570809 ms` | `+17487 ms` | `+1.1%` |
| `finalise_proof_from_base_trace` | `1331145 ms` | `1338474 ms` | `+7329 ms` | `+0.6%` |
| `compute_h_poly` | `1011070 ms` | `1010248 ms` | `-822 ms` | `-0.08%` |
| `lookup_fixed_cosets` | `56042 ms` | `55346 ms` | `-696 ms` | `-1.2%` |
| total `lookup_cosets` | `183937 ms` | `177971 ms` | `-5966 ms` | `-3.2%` |
| total `lookup_constraints` | `45415 ms` | `44996 ms` | `-419 ms` | `-0.9%` |
| aggregate lookup path | `285394 ms` | `278313 ms` | `-7081 ms` | `-2.5%` |
| `permutation_constraints` | `479854 ms` | `473073 ms` | `-6781 ms` | `-1.4%` |
| `custom_fixed_cosets` | `183833 ms` | `190020 ms` | `+6187 ms` | `+3.4%` |
| `build_h_poly_key` | `82441 ms` | `86405 ms` | `+3964 ms` | `+4.8%` |
| `build_opening_key` | `88652 ms` | `92099 ms` | `+3447 ms` | `+3.9%` |
| `construct_vanishing_commitments` | `61070 ms` | `61946 ms` | `+876 ms` | `+1.4%` |
| `deserialize_opening_polys` | `12165 ms` | `11679 ms` | `-486 ms` | `-4.0%` |
| `multi_open` | `63455 ms` | `63451 ms` | `-4 ms` | effectively `0%` |
| peak `VmHWM` | `23331520 kB` | `22970260 kB` | `-361260 kB` | `-1.5%` |

### Interpretation

This follow-up confirms that the lookup-lane work was directionally useful, but
it was a modest local win rather than a strong end-to-end win.

Strongest signal:

- total `lookup_cosets` improved by about `3.2%`
- aggregate lookup-related work improved by about `2.5%`
- peak memory also improved by about `353 MiB`

But the total run did not improve:

- total `prove-finalize` regressed by about `1.1%`
- `compute_h_poly` stayed effectively flat

So the current reading is:

- the optimization seems real inside the lookup lane
- the gain is relatively small compared with run-to-run noise and secondary
  costs elsewhere in the pipeline
- this was not a breakthrough optimization on the same level as the retained
  `permutation_sigma` throughput work

That makes this a reasonable local cleanup to keep, but not a new dominant
speed lever by itself.

### Chunk-size conclusion

An additional experiment with `--h-poly-row-chunk-size 14` showed that raising
the chunk size above `13` is not currently worthwhile on this machine for the
`circom_multiplier2` direct lane.

Operational conclusion:

- keep `--h-poly-row-chunk-size 13` as the recommended setting for real timing
  runs on this fixture
- do not assume that a larger chunk size buys meaningful wall-clock wins
- treat larger chunk sizes primarily as higher-risk memory experiments unless a
  future code change materially shifts the cost structure
