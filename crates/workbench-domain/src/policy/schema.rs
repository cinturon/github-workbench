use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PolicyConfig {
    pub schema_version: u32,
    pub strategy: StrategyConfig,
    #[serde(default)]
    pub branches: BranchesConfig,
    #[serde(default)]
    pub commits: CommitsConfig,
    #[serde(default)]
    pub pull_requests: PullRequestsConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct StrategyConfig {
    pub preset: String,
    pub default_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BranchesConfig {
    #[serde(default = "default_feature")]
    pub feature: BranchTypeConfig,
    #[serde(default = "default_fix")]
    pub fix: BranchTypeConfig,
    #[serde(default = "default_prefixes")]
    pub allowed_prefixes: Vec<String>,
}

impl Default for BranchesConfig {
    fn default() -> Self {
        Self {
            feature: default_feature(),
            fix: default_fix(),
            allowed_prefixes: default_prefixes(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BranchTypeConfig {
    pub pattern: String,
    pub start_from: String,
    pub require_issue: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CommitsConfig {
    #[serde(default)]
    pub require_signing: bool,
    #[serde(default = "default_conventional_commits")]
    pub conventional_commits: Enforcement,
}

impl Default for CommitsConfig {
    fn default() -> Self {
        Self {
            require_signing: false,
            conventional_commits: default_conventional_commits(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Enforcement {
    Off,
    Warning,
    Blocker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PullRequestsConfig {
    #[serde(default = "default_true")]
    pub draft_by_default: bool,
    #[serde(default = "default_main")]
    pub required_base: String,
    #[serde(default = "default_merge_method")]
    pub merge_method: String,
    #[serde(default = "default_true")]
    pub require_linked_issue: bool,
}

impl Default for PullRequestsConfig {
    fn default() -> Self {
        Self {
            draft_by_default: true,
            required_base: default_main(),
            merge_method: default_merge_method(),
            require_linked_issue: true,
        }
    }
}

pub(crate) fn default_feature() -> BranchTypeConfig {
    BranchTypeConfig {
        pattern: "feature/{issue}-{slug}".into(),
        start_from: "main".into(),
        require_issue: true,
    }
}

pub(crate) fn default_fix() -> BranchTypeConfig {
    BranchTypeConfig {
        pattern: "fix/{issue}-{slug}".into(),
        start_from: "main".into(),
        require_issue: true,
    }
}

pub(crate) fn default_prefixes() -> Vec<String> {
    ["feature", "fix", "docs", "chore"]
        .into_iter()
        .map(String::from)
        .collect()
}

fn default_conventional_commits() -> Enforcement {
    Enforcement::Warning
}

fn default_true() -> bool {
    true
}

fn default_main() -> String {
    "main".into()
}

fn default_merge_method() -> String {
    "squash".into()
}
