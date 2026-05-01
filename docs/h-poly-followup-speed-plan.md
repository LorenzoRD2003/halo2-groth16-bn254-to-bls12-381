# H Poly Follow-up Speed Plan

This note tracks speed-oriented follow-up work for the retained chunked
`h_poly` path after the current memory blocker is solved.

It is intentionally scoped to "what should we optimize next once the pipeline
finishes reliably" rather than the current OOM fire-fighting work.

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

## How To Use This Note

Reach for this file only after:

- `execute-wrapper-direct-prove-finalize` finishes reliably on the current
  machine
- the active blocker is no longer OOM in `h_poly`

Until then, the current source of truth for memory debugging remains:

- `docs/decisions/0003-direct-outer-setup-cost-reduction.md`
- `docs/decisions/0004-local-midnight-proofs-patch.md`
