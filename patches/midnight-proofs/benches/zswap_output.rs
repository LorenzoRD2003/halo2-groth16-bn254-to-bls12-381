//! Local placeholder for the upstream zswap-output benchmark.
//!
//! The original benchmark depends on a downstream Midnight application stack
//! (`midnight-circuits` + `midnight-zk-stdlib`) that brings in a second
//! `midnight-proofs` crate instance. Within this repository we patch
//! `midnight-proofs` locally, so compiling that benchmark here would produce
//! incompatible duplicate proof/circuit types instead of a meaningful signal.
//!
//! We therefore keep the target name reserved but make the local benchmark a
//! no-op until the downstream stack can be pointed at the same patched
//! `midnight-proofs` instance.

fn main() {}
