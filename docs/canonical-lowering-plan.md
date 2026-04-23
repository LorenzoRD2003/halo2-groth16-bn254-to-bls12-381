# Design Decision: Canonical Lowering to R1CS for Groth16 Compatibility

## Summary

We adopt **R1CS (Rank-1 Constraint Systems)** as the canonical circuit representation for Groth16 proving, avoiding the need to design a new intermediate representation (IR).

Optionally, we support **zkInterface** as a serialization and interoperability layer on top of R1CS, but **R1CS remains the single source of truth for circuit identity and CRS binding**.

This decision enables a **stable, auditable path** from Halo2/Midnight-style circuits to Groth16 (BLS12-381) proofs.

---

## Motivation

Current constraints:

- Halo2/Midnight uses a **PLONKish arithmetization**
- Groth16 requires **R1CS/QAP**
- No direct, stable translation exists today

We want:

- A **deterministic, auditable lowering path**
- A **canonical circuit identity** (for CRS reuse)
- A **direct mapping to Groth16 outputs** (`pi_a`, `pi_b`, `pi_c`, `IC`)

---

## Decision

### Canonical Representation

We define:

> **The canonical identity of a circuit is its normalized R1CS representation.**

- All CRS generation, VK derivation, and proof compatibility are tied to this R1CS.
- No alternative IR is introduced.

---

### Optional Layer: zkInterface

We optionally support:

- zkInterface as a **serialization format**
- zkInterface as an **interchange layer** for tooling

But:

- zkInterface is **not the canonical identity**
- It is a transport format around R1CS

---

## Architecture

```
Halo2 / Midnight Circuit
        ↓
Canonical Lowering (deterministic)
        ↓
R1CS (canonical form)
        ↓
Groth16 (arkworks / equivalent)
        ↓
Proof (pi_a, pi_b, pi_c)
VK (IC, etc.)
```

---

## Key Requirement: Canonical Lowering

Even though we reuse R1CS, we must define a **canonical lowering specification** from Halo2 to R1CS.

Without this, we cannot guarantee:

- CRS stability
- reproducible builds
- circuit identity consistency

---

## Supported Halo2 Subset (Phase 1)

We intentionally start with a restricted subset:

### Supported

- Witness variables
- Explicit constants inside sparse linear combinations
- Public-input placeholders
- Simple algebraic constraints directly representable as
  `(A ⋅ X) * (B ⋅ X) = (C ⋅ X)`

### Intentionally Unsupported In Phase 1

- Equality / copy constraints
- Permutation arguments
- Lookup arguments
- Claims of full Halo2/Midnight circuit introspection

---

## Canonicalization Rules

To ensure deterministic R1CS:

### Variable Ordering

- Deterministic order:
  - instance variables first
  - then advice variables
  - then intermediate variables

- Stable indexing across runs

### Constraint Ordering

- Ordered by:
  - row index
  - then gate index

- No nondeterministic iteration

### Linear Combination Normalization

- Sort terms by variable index
- Combine duplicates
- Remove zero coefficients
- Normalize constant placement

---

## Lowering Plan (Phased Implementation)

We implement the lowering pipeline incrementally.

Each phase should be independently testable and auditable.

---

### Phase 1 — Basic Algebraic Constraints

**Goal:** Support simple gates of the form:

```
q * (a * b - c) = 0
```

#### Tasks

- Extract:
  - variables from advice columns
  - constants from fixed columns

- Build R1CS constraints:

  ```
  (A ⋅ X) * (B ⋅ X) = (C ⋅ X)
  ```

#### Deliverable

- Minimal canonical R1CS core
- Deterministic builder for the supported algebraic subset
- A clearly named Halo2/Midnight lowering boundary that can attach to real
  circuit introspection later

#### Phase 1 implementation status in this repo

The landed Phase 1 slice is intentionally narrower than a full Halo2 lowering
path:

- the canonical representation is a small internal R1CS data model in
  `crates/wrapper-circuits/src/r1cs.rs`
- linear combinations are canonicalized immediately by sorting terms by
  variable id, combining duplicates, dropping zero coefficients, and keeping
  the constant slot explicit
- constraint order is the explicit insertion order of the builder/lowering
  boundary
- the current lowering entry surface is a small explicit
  `Halo2Phase1R1csLowering` builder, because the repo does not yet expose a
  reusable real-circuit extraction hook for gates/equality/lookups
- supported encodings are:
  - `x * y = z`
  - `x * c = z`
  - `linear = linear` encoded as `(linear_lhs - linear_rhs) * 1 = 0`
  - `linear = constant` encoded as `(linear - constant) * 1 = 0`

Phase 1 still does **not** lower equality/copy constraints, permutation logic,
lookups, or a full Groth16 proving flow.

---

### Phase 2 — Equality Constraints

**Goal:** Support Halo2 equality (copy) constraints

#### Tasks

- Map equality constraints to shared canonical variable indices

#### Design Choice

Prefer:

- **variable unification** only
- no explicit `x - y = 0` constraints in the canonical lowering path

#### Phase 2 implementation status in this repo

The landed Phase 2 slice adds:

- canonical Halo2 cell identities through `Halo2CellRef`
- deterministic equality edges through `EqualityEdge`
- a deterministic union-find whose representative is always the minimum cell in
  the equality class
- canonical class-to-variable assignment ordered by class representative, with
  instance-backed classes before advice-backed classes
- public-input extraction from canonical equality classes
- `Halo2Phase1R1csLowering` now lowering from Halo2 cells via the canonical
  assignment map instead of allocating variables ad hoc

Phase 2 still does **not** implement permutations, lookups, or full real Halo2
constraint-system introspection.

---

### Phase 3 — Public Inputs

**Goal:** Support instance columns

#### Tasks

- Map instance cells to:
  - R1CS public inputs

- Ensure:
  - deterministic ordering
  - correct indexing in IC

#### Phase 3 implementation status in this repo

The landed Phase 3 slice adds:

- an explicit metadata boundary through `Halo2R1csMetadata`
- explicit frontend public-input ordering through `Halo2PublicInputRef`
- validation for public-input cells, indices, and equality-edge endpoints
- metadata-driven construction of `Halo2CellAssignmentMap`
- public input ordering now follows frontend-provided `public_index`

Canonical variable identity still does **not** depend on public input order:

- equality classes still use the minimum `Halo2CellRef` as representative
- class-to-`VariableId` assignment remains canonical and deterministic
- reordering public input indices changes `public_variables`, but does not
  change variable identity

This still does **not** implement full Halo2/Midnight introspection, lookups,
or permutation arguments.

---

### Phase 4 — Canonicalization Layer

**Goal:** Ensure reproducible circuit identity

#### Tasks

- Implement:
  - variable ordering rules
  - constraint ordering rules
  - linear combination normalization

- Add:
  - hashing of R1CS as canonical identity

#### Phase 4 implementation status in this repo

The landed Phase 4 slice adds:

- a canonical byte encoding for `R1csCircuit`
- a versioned domain separator:
  `halo2-groth16-wrapper:r1cs:v1`
- stable circuit hashing through `R1csIdentityHash`
- `CRS_ID = hash(canonical_R1CS)` as the current identity contract

The current identity includes:

- canonical public input order
- canonical variable ids
- canonical linear-combination normalization
- canonical constraint insertion order

Therefore:

- public input order is CRS-binding
- constraint order is CRS-binding
- equality-edge insertion order is **not** CRS-binding except through the final
  canonical variable mapping

This still does **not** implement Groth16 proving, QAP generation, zkInterface
export, lookups, or permutation arguments.

---

### Phase 5 — zkInterface Export (Optional)

**Goal:** Enable interoperability

#### Tasks

- Serialize:
  - R1CS → zkInterface

- Support:
  - witness export
  - constraint system export

#### Phase 5 implementation status in this repo

The landed Phase 5 slice adds:

- an internal zkInterface bridge model for canonical R1CS export
- deterministic export of field modulus, public variables, and constraints
- deterministic witness export ordered by `VariableId`
- export validation against the canonical `R1csCircuit`

This bridge preserves canonical R1CS identity:

- `identity_hash` is carried through the export
- public variable order is preserved exactly
- constraint order is preserved exactly
- R1CS remains the CRS-binding source of truth

External protobuf / zkInterface crate serialization is still future work.

---

### Phase 6 — Arkworks Groth16 Adapter

**Goal:** Delegate QAP generation and Groth16 proving to Arkworks.

#### Phase 6 implementation status in this repo

The landed Phase 6 slice adds:

- an `arkworks.rs` adapter from canonical `R1csCircuit` into
  `ark_relations::r1cs::ConstraintSynthesizer`
- thin helpers for Arkworks Groth16 setup, prove, and verify
- deterministic ordered public-input extraction through
  `R1csCircuit::public_variables()`
- structured assignment validation before synthesis/proving

Important current scope note:

- the first Arkworks adapter targets the current canonical R1CS field, which is
  the BN254 scalar field backing `NativeField`
- QAP generation is delegated entirely to Arkworks
- Groth16 internals are delegated entirely to Arkworks
- canonical R1CS remains the CRS-binding source of truth

This phase still does **not** implement lookups, permutation arguments, or full
Halo2/Midnight introspection.

---

### Phase 6 — Lookup Lowering (Advanced)

**Goal:** Support Halo2 lookups

#### Tasks

- Expand lookups into:
  - algebraic constraints OR
  - table constraints encoded in R1CS

Note:

- This is complex and may significantly increase constraint count

---

## CRS Identity

We define:

```
CRS_ID = hash(canonical_R1CS)
```

This ensures:

- reproducible trusted setup
- consistent verification keys
- no ambiguity across builds

---

## Groth16 Mapping

Once in R1CS:

- Use standard Groth16 pipeline:
  - R1CS → QAP
  - QAP → CRS
  - CRS → Proof

Output maps directly to:

- `pi_a`, `pi_b`, `pi_c`
- `IC` (input coefficients)
- `nPublic`

No lossy translation required.

---

## Tradeoffs

### Pros

- Reuses established standard (R1CS)
- Avoids designing new IR
- Clean Groth16 compatibility
- Strong auditability

### Cons

- Requires explicit lowering spec
- Potential constraint blowup (especially for lookups)
- Halo2 features must be restricted or expanded

---

## Future Work

- Optimize lowering for constraint minimization
- Add lookup compression strategies
- Explore hybrid approaches (e.g., partial PLONK → Groth16 bridges)
- Benchmark against direct R1CS implementations

---

## Conclusion

We do not introduce a new IR.

Instead, we:

- adopt **R1CS as the canonical representation**
- optionally use **zkInterface for interoperability**
- define a **deterministic lowering from Halo2 to R1CS**

This provides a **clear, auditable, and Groth16-compatible path forward**.
