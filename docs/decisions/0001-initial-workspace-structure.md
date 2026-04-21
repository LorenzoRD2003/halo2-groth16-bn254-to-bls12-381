# ADR 0001: Initial Workspace Structure

## Status

Accepted.

## Context

The project starts as a research-oriented repository but is expected to grow into a multi-week engineering effort spanning domain modeling, backend integration, Halo2 circuit work, and developer tooling. A single crate would be fast to start but would make later separation more painful, especially once cryptographic dependencies arrive.

## Decision

The repository is split into the following crates from the beginning:

- `wrapper-core`
- `wrapper-circuits`
- `wrapper-backends`
- `wrapper-cli`
- `wrapper-tests`

Each crate owns a distinct responsibility boundary, with `wrapper-core` serving as the stable domain layer shared by the others.

## Consequences

Positive:

- cleaner ownership boundaries
- lower risk of Halo2 or backend concerns infecting core APIs
- easier contributor onboarding
- more deliberate dependency management
- better staging for future testing and documentation

Tradeoffs:

- slightly more up-front boilerplate
- more manifests to maintain
- some placeholder types exist before concrete implementations

## Alternatives Considered

Single crate:

- rejected because it would encourage mixing domain, backend, and circuit concerns during the earliest implementation phase

Core plus monolithic implementation crate:

- rejected because it still obscures the backend versus circuit boundary that the project is specifically trying to preserve

