//! Application layer — business logic for auth, proposals, and signatures.
//!
//! Handlers delegate here. Traits defined here are implemented in `crate::infrastructure`.

pub(crate) mod proposals;
pub(crate) mod traits;
