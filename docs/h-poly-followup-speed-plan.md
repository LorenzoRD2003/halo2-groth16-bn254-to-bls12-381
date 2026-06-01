# H Poly Follow-up Speed Plan

This note tracks speed-oriented follow-up work for the retained chunked
`h_poly` path after the current memory blocker is solved.

It is intentionally scoped to "what should we optimize next once the pipeline
finishes reliably" rather than the current OOM fire-fighting work.

The first successful chunked finalize baseline is now recorded in:

- `docs/finalize-successful-run-metrics.md`

## Current Observation

The current chunked permutation path inside `evaluate_h(...)` reduces peak
memory, but it can become very slow.

The main reason is that the earlier "chunked" helper was not truly chunked: it
still built a full extended-domain polynomial before slicing out one chunk.

That issue led to the current hybrid design:

- keep per-set permutation sigma cosets materialized once
- walk rows in chunks
- expose the row-chunk exponent from `execute-wrapper-direct-prove-finalize`

This is a good memory tradeoff, but it still leaves several speed
opportunities.

Current operational note from later successful runs:

- on the current machine and `circom_multiplier2` fixture, raising
  `--h-poly-row-chunk-size` from `13` to `14` did not prove worthwhile
- `13` is the current recommended measurement setting because it keeps memory
  safer without giving up a clear wall-clock win
- future chunk-size experiments should therefore start from `13`, not from the
  assumption that "larger must be faster"

Current BLS12-hosted memory note from the first direct `prove-finalize`
attempt on `circom_multiplier2`:

- the run did not first fail inside `h_poly`; it reached `multi_open`
- however, the largest visible memory jumps happened earlier in:
  - `deserialize_prepared_trace`
  - `custom_fixed_cosets`
- the largest `h_poly`-local pressure point remained `custom_fixed_cosets`,
  which pushed the run to roughly `22.4 GiB` HWM before later work crossed the
  final limit
- the current mitigation therefore targets peak memory, not throughput:
  split `custom_gates` into smaller gate-local evaluator batches so fixed
  columns are materialized and dropped batch-by-batch instead of all at once

## Prioritized Follow-ups

1. Avoid copying chunk windows when no wrap occurs.
   Today helper logic still allocates `Vec<F>` chunk windows such as
   `current_chunk`, `current_next_chunk`, `previous_last_chunk`, `first_chunk`,
   and `last_chunk`.
   A better design would expose one borrowed slice when the chunk does not wrap,
   and only fall back to an owned buffer when wrapping is actually required.

2. Reuse scratch buffers across chunks.
   The permutation path still allocates fresh `Vec<F>` chunk buffers for the
   current set and sigma columns.
   Reusing per-set scratch storage should lower allocator overhead and improve
   throughput once the shape is stable.

3. Consider reducing logging overhead in non-diagnostic runs.
   The current fine-grained `h_poly` logging is useful while debugging, but it
   adds overhead.
   Once the memory issue is solved, keep:
   - fine-grained logs for debugging mode
   - coarser progress logs for normal long-running measurements

4. Revisit lookup/trash paths with the same hybrid pattern.
   Even though recent failures stop in permutation constraints, the next likely
   bottlenecks are:
   - lookup cosets
   - trash cosets
   Those paths still deserve the same "materialize once per argument, then walk
   rows in chunks" treatment if they become dominant later.

5. Evaluate a true partial extended-domain evaluation helper.
   The strongest long-term direction would be one domain helper that computes
   only the requested chunk of one coefficient-form polynomial in the extended
   coset domain.
   This is more invasive than the current hybrid approach, so it is explicitly
   deferred until correctness and memory stability are established.

6. Keep custom-gate fixed materialization batch-local.
   The current retained memory-oriented change is:
   - build one custom-gate evaluator batch per gate
   - materialize only that batch's fixed columns
   - evaluate the batch
   - drop the batch-local fixed materialization before continuing
   This should reduce peak memory during `custom_fixed_cosets` at the cost of
   some extra passes over `values`. Measure it first as a memory tradeoff, not
   assume it is a throughput win.

## How To Use This Note

Reach for this file only after:

- `execute-wrapper-direct-prove-finalize` finishes reliably on the current
  machine
- the active blocker is no longer OOM in `h_poly`

Until then, the current source of truth for memory debugging remains:

- `docs/decisions/0003-direct-outer-setup-cost-reduction.md`
- `docs/decisions/0004-local-midnight-proofs-patch.md`
