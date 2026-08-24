use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

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
    #[serde(default)]
    pub remote_testing: RemoteTestingConfig,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionHours(pub u64);

impl Serialize for RetentionHours {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}h", self.0))
    }
}

impl<'de> Deserialize<'de> for RetentionHours {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let hours = value
            .strip_suffix('h')
            .ok_or_else(|| de::Error::custom("retention must end in h"))?
            .parse::<u64>()
            .map_err(de::Error::custom)?;
        Ok(Self(hours))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RemoteTestingConfig {
    #[serde(default = "default_remote_isolation")]
    pub isolation: String,
    #[serde(default = "default_remote_branch_prefix")]
    pub branch_prefix: String,
    #[serde(default = "default_max_matrix_jobs")]
    pub max_matrix_jobs: u16,
    #[serde(default = "default_remote_timeout")]
    pub default_timeout_minutes: u16,
    #[serde(default = "default_successful_retention")]
    pub successful_ref_retention: RetentionHours,
    #[serde(default = "default_failed_retention")]
    pub failed_ref_retention: RetentionHours,
}

impl Default for RemoteTestingConfig {
    fn default() -> Self {
        Self {
            isolation: default_remote_isolation(),
            branch_prefix: default_remote_branch_prefix(),
            max_matrix_jobs: default_max_matrix_jobs(),
            default_timeout_minutes: default_remote_timeout(),
            successful_ref_retention: default_successful_retention(),
            failed_ref_retention: default_failed_retention(),
        }
    }
}

fn default_remote_isolation() -> String {
    "ephemeral-branch".into()
}

fn default_remote_branch_prefix() -> String {
    "github-workbench/test".into()
}

fn default_max_matrix_jobs() -> u16 {
    6
}

fn default_remote_timeout() -> u16 {
    15
}

fn default_successful_retention() -> RetentionHours {
    RetentionHours(0)
}

fn default_failed_retention() -> RetentionHours {
    RetentionHours(72)
}
