use super::schema::{
    BranchesConfig, CommitsConfig, PolicyConfig, PullRequestsConfig, RemoteTestingConfig,
    StrategyConfig,
};

pub fn github_flow_defaults() -> PolicyConfig {
    PolicyConfig {
        schema_version: 1,
        strategy: StrategyConfig {
            preset: "github-flow".into(),
            default_branch: "main".into(),
        },
        branches: BranchesConfig::default(),
        commits: CommitsConfig::default(),
        pull_requests: PullRequestsConfig::default(),
        remote_testing: RemoteTestingConfig::default(),
    }
}
