# Finalize Checkpoint Profiling Plan

## Purpose

This document turns the current `prove-finalize` debugging need into a concrete
implementation plan.

The immediate goal is not to optimize rows, change prover semantics, or solve
the memory issue directly.
The immediate goal is:

- to identify the last successful checkpoint reached by
  `execute-wrapper-direct-prove-finalize`
- to distinguish whether the failure occurs:
  - before `compute_h_poly(...)`
  - inside `compute_h_poly(...)` / `evaluate_h(...)`
  - after `h_poly` during openings / evaluations / `multi_open`
- to keep high-frequency progress logs so we also learn which subphases are
  slow even when they succeed
- to produce logs that are structured enough to compare runs across chunk sizes,
  host lanes, and code revisions

This is an implementation plan, not an ADR.
It should guide the next instrumentation pass on the direct finalize lane.

## Current Baseline

The repository already has useful but incomplete logging on the finalize path.

### Existing wrapper-side checkpoints

`crates/wrapper-backends/src/outer/direct/proving.rs` already logs:

- `prove-finalize: validating setup verification key`
- `prove-finalize: using circuit_k=...`
- `prove-finalize: deserializing BaseProvingKey`
- `prove-finalize: deserializing persisted prover trace`
- `prove-finalize: entering finalise_proof_from_base_trace`
- `prove-finalize: finalise_proof_from_base_trace complete`
- `prove-finalize: proof serialization and validation complete`

### Existing prover-side checkpoints

`patches/midnight-proofs/src/plonk/prover.rs` already logs major subphases of
`finalise_proof_from_base_trace(...)`, including:

- building the `h_poly` key
- computing `h_poly`
- building the opening key
- constructing vanishing commitments
- deserializing opening polynomials
- writing evals to the transcript
- evaluating vanishing / permutation / lookup / trash commitments
- computing opening queries
- entering `multi_open`

### Existing `h_poly` key checkpoints

`patches/midnight-proofs/src/plonk/mod.rs` already logs:

- `before compute_lagrange_polys`
- `after compute_lagrange_polys`
- `before sparse fixed cosets`
- `after sparse fixed cosets`
- `before permutation h key`
- `after permutation h key`

### Existing fine-grained `evaluate_h(...)` checkpoints

`patches/midnight-proofs/src/plonk/evaluation.rs` already logs progress inside
`evaluate_h(...)`, especially around:

- custom gates
- permutation constraints
- permutation sets
- sigma-column cosets
- lookups
- trash constraints

This means the next pass should deepen and standardize the existing logging,
not replace it.

## Problems With The Current Logging

Even though logging exists, it is still too weak for reliable diagnosis.

Current issues:

- log lines are free-form strings rather than one stable structured schema
- we do not capture resident memory at each checkpoint
- we do not capture elapsed time per subphase
- not every heavy subphase has explicit `start` and `end` markers
- some logs are progress-oriented but not directly comparable across runs
- there is no simple post-processing tool that reports:
  - last successful checkpoint
  - next checkpoint not reached
  - peak memory observed before failure
  - slowest successful subphases

## Design Goals

The instrumentation pass should satisfy all of the following:

1. Every heavy finalize subphase has explicit `start` and `end` logs.
2. Every checkpoint log carries enough metadata to identify the run context.
3. Every checkpoint log records memory at that point.
4. Every `end` checkpoint records elapsed time for the subphase.
5. There is also a lightweight heartbeat log at each meaningful iteration
   boundary in the hottest loops.
6. The log file is readable in real time with one operator command while the
   process is still running.
7. The log format is machine-parsable with simple tools.
8. The logging overhead is acceptable for long-running debug experiments.
9. The same instrumentation supports both:
   - OOM localization
   - time profiling of slow-but-successful runs

## Logging Contract

### Log transport

Keep the current transport based on `WRAPPER_DIRECT_LOG_FILE`.
Do not introduce a second logging sink for this work.

Reason:

- the repo already uses this path
- the current direct-lane workflow is already built around file-backed long-run
  diagnostics

### Stable log shape

Switch the checkpoint logs to one line per event with stable key-value fields.

Recommended shape:

```text
ts=... level=INFO run_id=... lane=prove-finalize host=... fixture=... k=... phase=... step=... event=start|end|point elapsed_ms=... rss_kb=... hwm_kb=... extra=...
```

Minimum required fields:

- `ts`
- `run_id`
- `lane`
- `host`
- `k`
- `phase`
- `step`
- `event`
- `rss_kb`
- `hwm_kb`

Required when applicable:

- `elapsed_ms`
- `chunk_size`
- `iteration`
- `proof_index`
- `set_idx`
- `column_idx`
- `column_count`
- `extra`

### Real-time readability requirement

The log file must remain line-buffered and append-only so it can be inspected
while the prover is still running.

Minimum supported operator workflow:

```bash
tail -f "$WRAPPER_DIRECT_LOG_FILE"
```

Recommended filtered workflow for long runs:

```bash
tail -f "$WRAPPER_DIRECT_LOG_FILE" | rg --line-buffered "prove-finalize|midnight finalize|midnight h_poly"
```

This requirement means:

- every log event must end with a newline immediately
- no batch flushing at the end of a phase
- no binary or multi-line logging format for this pass

### Memory fields

Read memory from `/proc/self/status` at each checkpoint and log:

- `VmRSS`
- `VmHWM`
- optionally `VmSize`

The plan should prefer one small helper that parses this file once per event.

### Timing fields

Every heavy subphase should emit:

- one `start` event
- one `end` event with `elapsed_ms`

For nested operations, use scoped timers rather than wall-clock subtraction in
callers only.

### Iteration-level heartbeat logs

In addition to `start` / `end` checkpoints, the hottest iterative regions
should emit lightweight progress logs at every meaningful iteration.

This does not mean logging every field element or row.
It means logging every iteration boundary that materially advances the loop
structure we are trying to profile.

Required iteration-style logs:

- every permutation set iteration
- every sigma-column iteration within a permutation set
- every lookup iteration
- every trash iteration
- every row-chunk iteration when chunked paths are active

Recommended event shape:

```text
ts=... run_id=... phase=h_poly step=permutation_sigma event=iter iteration=... proof_index=... set_idx=... column_idx=... chunk_size=... rss_kb=... hwm_kb=...
```

Guiding rule:

- if the process dies, the log should tell us the exact last completed loop
  iteration, not merely the last coarse phase

## Instrumentation Phases

## Phase 1. Standardize wrapper-side finalize logs

Goal:

- make the outermost direct finalize lane self-describing

Files:

- `crates/wrapper-backends/src/outer/direct/proving.rs`

Changes:

- add one small helper that writes structured checkpoint lines
- log `host lane`, `backend`, `circuit_k`, and a generated `run_id`
- split current coarse logs into explicit `start` / `end` pairs for:
  - `deserialize_base_pk`
  - `deserialize_prepared_trace`
  - `init_transcript`
  - `finalise_proof_from_base_trace`
  - `serialize_and_validate_proof`

Why this matters:

- it immediately rules out failures that are not in the prover internals
- it gives one top-level elapsed time split even before touching the patch

Acceptance criteria:

- one failed finalize run clearly shows the last completed outer wrapper step
- one successful finalize run shows elapsed time for each outer wrapper step

## Phase 2. Standardize prover-side finalize logs

Goal:

- turn `finalise_proof_from_base_trace(...)` into a sequence of measurable and
  comparable subphases

Files:

- `patches/midnight-proofs/src/plonk/prover.rs`

Changes:

- replace free-form progress logs with structured logs that preserve the same
  step names
- ensure `start` / `end` pairs for:
  - `build_h_poly_key`
  - `compute_h_poly`
  - `build_opening_key`
  - `construct_vanishing_commitments`
  - `deserialize_opening_polys`
  - `write_evals_to_transcript`
  - `evaluate_vanishing`
  - `evaluate_shared_permutation_data`
  - `evaluate_permutation_commitments`
  - `evaluate_lookup_commitments`
  - `evaluate_trash_commitments`
  - `compute_opening_queries`
  - `multi_open`

Also add:

- `proof_index`
- serialized-trace section sizes when known
- memory snapshots before and after each heavy step

Acceptance criteria:

- the last successful internal finalize step is always visible in one failed
  run
- the slowest successful internal step is obvious in one successful run

## Phase 3. Deepen `finalize_for_h_poly()` promotion logs

Goal:

- determine whether the memory spike comes from promoting the base key into
  `HPolyKey` rather than from evaluating `h_poly` itself

Files:

- `patches/midnight-proofs/src/plonk/mod.rs`
- `patches/midnight-proofs/src/plonk/permutation.rs`

Changes:

- keep existing checkpoints
- add structured metadata to them:
  - domain size
  - number of fixed columns marked as used
  - number of advice columns marked as used
  - number of instance columns marked as used
  - number of sparse fixed cosets actually materialized
  - permutation set count
- if cheap to compute, also log estimated byte counts for:
  - materialized fixed cosets
  - lagrange polys
  - permutation h-key promotion data

Acceptance criteria:

- a failed run can distinguish:
  - failure before sparse fixed cosets
  - failure during sparse fixed cosets
  - failure during permutation h-key promotion

## Phase 4. Add ultra-fine timing and progress inside `compute_h_poly(...)`

Goal:

- localize OOM or time hotspots inside `compute_h_poly_from_prepared_parts(...)`
  and `Evaluator::evaluate_h(...)`

Files:

- `patches/midnight-proofs/src/plonk/prover.rs`
- `patches/midnight-proofs/src/plonk/evaluation.rs`

Changes:

- preserve existing `evaluate_h(...)` logs, but normalize them into the same
  schema
- add `start` / `end` timing around:
  - custom gates block
  - permutation constraints block
  - each permutation set
  - each permutation set column-view materialization
  - each sigma-coset materialization
  - final accumulation per set
  - lookup blocks
  - trash blocks
- log cardinalities:
  - `proof_index`
  - `set_idx`
  - `set_count`
  - `column_idx`
  - `column_count`
  - `row_chunk_size`
  - domain size
- emit one lightweight `event=iter` log for every:
  - permutation set iteration
  - sigma-column iteration
  - lookup iteration
  - trash iteration
  - chunk loop iteration where the row-chunk path is active

Important rule:

- do not log every single row or element
- log one event per meaningful chunk boundary or per set/column subphase

Acceptance criteria:

- one failed run can identify the last successful permutation set / sigma
  column / chunk-sensitive block reached
- one successful run can identify which major `h_poly` subphase dominates wall
  clock time

## Phase 5. Add a run-summary extractor

Goal:

- avoid manual log reading for every experiment

Suggested location:

- `scripts/` if a new helper script is appropriate, or
- a small CLI subcommand if the team wants it inside `wrapper-cli`

Recommended outputs:

- `last_successful_checkpoint`
- `next_missing_checkpoint`
- `max_rss_kb_seen`
- `max_hwm_kb_seen`
- `slowest_completed_step`
- `elapsed_ms_by_phase`
- `chunk_size_used`
- `fixture`
- `host`
- `k`

This helper can stay intentionally simple.
It only needs to parse the structured key-value log format introduced above.

Acceptance criteria:

- one command or script can summarize a finalize run in a few lines
- the output is sufficient to compare two runs with different chunk sizes

## Experimental Workflow

Once instrumentation is in place, use this workflow.

### 1. Freeze the input artifacts

For one profiling series, keep constant:

- fixture
- setup artifact
- proving-key sidecar
- persisted prover trace
- host lane
- thread count

This avoids mixing instrumentation results with artifact-shape drift.

### 2. Run one baseline failure reproduction

Use the smallest real reproduction path first:

- fixture: `circom_multiplier2`
- host: BN254
- one known failing `prove-finalize` path

Expected output:

- one structured log with a clear last successful checkpoint
- one live stream that can be watched with `tail -f`

### 3. Sweep `--h-poly-row-chunk-size`

Run the same finalize input with a small matrix:

- default
- `18`
- `17`
- `16`
- `15`

Goal:

- check whether the last successful checkpoint changes with chunk size
- check whether elapsed time shifts from one subphase to another
- check whether the last successful iteration-level heartbeat changes with
  chunk size

Interpretation:

- if the last successful checkpoint moves within permutation-set logging, the
  issue is likely inside chunk-sensitive `evaluate_h(...)`
- if it never gets past `build_h_poly_key`, chunking is not the first target
- if `h_poly` completes and later phases dominate, the next optimization should
  target opening-side work instead

## Decision Rules For Interpreting The Logs

Use the following rules when triaging results.

### Case 1. Failure before `build_h_poly_key`

Interpretation:

- the issue is not yet in `compute_h_poly(...)`
- inspect deserialization, trace loading, or transcript setup first

### Case 2. Failure inside `finalize_for_h_poly()`

Interpretation:

- the issue is in key promotion
- first suspects:
  - lagrange polynomial materialization
  - sparse fixed cosets
  - permutation h-key promotion

### Case 3. Failure between `compute_h_poly start` and `h_poly complete`

Interpretation:

- the issue is inside `evaluate_h(...)`
- first suspects:
  - permutation set cosets
  - sigma-coset materialization
  - chunk-size-sensitive accumulation

### Case 4. `h_poly` completes and failure happens later

Interpretation:

- the issue is in opening-side finalization, not in `h_poly`
- inspect:
  - opening-key promotion
  - vanishing commitments
  - opening polynomial deserialization
  - query construction
  - `multi_open`

### Case 5. Successful run but extreme latency in one block

Interpretation:

- even without OOM, the same instrumentation doubles as a time profiler
- prioritize the slowest stable subphase after memory stability is understood

## Non-Goals

This plan does not yet propose:

- changing the arithmetic or semantics of `compute_h_poly(...)`
- changing the external proof format
- introducing a full tracing framework dependency
- solving the OOM in the same patch as the logging work

The goal is observability first.

## Expected Result

After this plan lands, the team should be able to answer questions like:

- “what is the exact last successful checkpoint before OOM?”
- “is the failure before or after `compute_h_poly(...)`?”
- “which permutation set or sigma-column subphase was active at failure?”
- “which exact loop iteration was the last one completed?”
- “what was the resident memory at that moment?”
- “which successful finalize subphase is the slowest?”
- “can we watch that progress live while the run is still executing?”

That level of visibility is enough to convert the current debugging state from:

- “the failure seems to be somewhere near `compute_h_poly(...)`”

into:

- “the last successful checkpoint is `...`, RSS/HWM were `...`, and the next
  optimization target is clearly `...`”
