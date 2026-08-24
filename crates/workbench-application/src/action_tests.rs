use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use workbench_domain::operations::plan::OperationPlan;
use workbench_domain::policy::RetentionHours;
use workbench_domain::testing::{AssertionReport, TestAssertions, TestPlan};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupIdentity {
    pub remote: String,
    pub ref_name: String,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedRemoteRef {
    pub identity: CleanupIdentity,
    pub commit_sha: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteTestSessionPlan {
    pub project_id: String,
    pub repo_root: PathBuf,
    pub owner: String,
    pub repo: String,
    pub remote: String,
    pub base_sha: String,
    pub session_id: String,
    pub workflow_file_name: String,
    pub workflow_path: String,
    pub workflow_yaml: String,
    pub test_plan: TestPlan,
    pub assertions: TestAssertions,
    pub successful_ref_retention: RetentionHours,
    pub failed_ref_retention: RetentionHours,
    pub cleanup_identity: CleanupIdentity,
    pub git_plan: OperationPlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteTestResult {
    pub session_id: String,
    pub run_id: u64,
    pub run_url: String,
    pub conclusion: String,
    pub passed: bool,
    pub assertion_report: AssertionReport,
    pub manifest_path: Option<PathBuf>,
    pub logs_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredSessionState {
    pub plan: RemoteTestSessionPlan,
    pub pushed_sha: Option<String>,
    pub result: Option<RemoteTestResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TestSessionStatus {
    Planned,
    Pushed,
    Queued,
    InProgress,
    Passed,
    Failed,
}
