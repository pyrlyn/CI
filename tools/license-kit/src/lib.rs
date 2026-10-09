//! Sync and check pyrlyn/ci's canonical license files in GPL repositories.
//!
//! `licenses/targets.yml` (see [`config`]) maps each target repository to the paths of its
//! copies; `licenses/<file>` holds the single source of truth. GitHub access goes through the
//! [`github::GitHub`] trait so every decision is testable against an in-memory mock.

pub mod automerge;
pub mod config;
pub mod drift;
pub mod github;
pub mod headers;
pub mod license;
pub mod readme;
pub mod sync;
