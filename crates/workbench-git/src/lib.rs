//! Process-based Git adapter. Phase 2.

pub mod argv;
pub mod client;
pub mod env;
pub mod parser;
pub mod process;

pub use argv::{assert_no_force, command_argv, describe_command};
pub use client::ProcessGitClient;
pub use env::sanitized_env;
pub use parser::{parse_ahead_behind, parse_porcelain_z, parse_remotes_verbose};
pub use process::StdProcessRunner;
