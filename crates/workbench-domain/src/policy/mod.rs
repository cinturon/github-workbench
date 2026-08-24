pub mod finding;
pub mod load;
pub mod preset;
pub mod schema;

pub use finding::{PolicyFinding, Severity};
pub use load::{merge_policy, parse_policy_yaml};
pub use preset::github_flow_defaults;
pub use schema::{
    BranchTypeConfig, BranchesConfig, CommitsConfig, Enforcement, PolicyConfig, PullRequestsConfig,
    StrategyConfig,
};
