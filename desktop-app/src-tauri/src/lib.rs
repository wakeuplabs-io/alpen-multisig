//! Desktop application library — exposes layered modules for external consumers.
//!
//! Layers follow ADR-005: `domain` (pure types), `application` (business logic + traits),
//! `infrastructure` (concrete implementations, including crypto signing).

pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;
