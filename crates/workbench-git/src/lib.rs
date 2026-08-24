//! Process-based Git adapter. Phase 2.

pub mod argv;
pub mod env;
pub mod process;

pub use argv::{assert_no_force, command_argv, describe_command};
pub use env::sanitized_env;
pub use process::StdProcessRunner;
