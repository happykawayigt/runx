//! Pure Rust parity kernel for runx decisions.
//!
//! TypeScript remains authoritative until every kernel fixture passes in both
//! languages and a later cutover spec changes ownership.

pub mod kernel_eval;
pub mod policy;
pub mod serde_conventions;
pub mod state_machine;
