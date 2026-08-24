//! External capability ports. Implementations arrive in later phases.

#![allow(dead_code)]

/// Placeholder until Phase 2 defines Git operations.
pub trait GitClient {}

/// Placeholder until Phase 2/4 defines GitHub operations.
pub trait GitHubClient {}

/// Placeholder until Phase 2 defines operation journaling.
pub trait OperationStore {}
