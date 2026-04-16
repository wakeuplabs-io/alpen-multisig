//! Desktop application library — exposes layered modules for external consumers.
//!
//! Layers follow ADR-005: `domain` (pure types), `application` (business logic + traits),
//! `infrastructure` (concrete implementations), and `signing` (standalone crypto).

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod signing;
