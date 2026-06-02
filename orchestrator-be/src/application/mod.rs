//! Application layer — business logic for auth, proposals, and signatures.
//!
//! Handlers delegate here. Traits defined here are implemented in `crate::infrastructure`.
//!
//! **P-028:** Do not import `strata_*` crates here. SSZ/ASM types stay in
//! `crate::infrastructure::action_codec` and `asm_role_membership`.

pub(crate) mod proposals;
pub(crate) mod traits;
