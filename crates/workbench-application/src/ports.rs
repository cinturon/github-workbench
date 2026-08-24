use std::path::{Path, PathBuf};

use crate::error::AppError;
use workbench_domain::operations::plan::{OperationPlan, StepStatus};
use workbench_domain::repository::{BranchState, Remote, RepositorySnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait ProcessRunner {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput, AppError>;
}

pub trait GitClient {
    fn resolve_toplevel(&self, path: &Path) -> Result<PathBuf, AppError>;
    fn snapshot(&self, repo_root: &Path) -> Result<RepositorySnapshot, AppError>;
    fn branch_state(&self, repo_root: &Path, comparison_ref: &str)
        -> Result<BranchState, AppError>;
    fn list_remotes(&self, repo_root: &Path) -> Result<Vec<Remote>, AppError>;
    fn fetch(&self, repo_root: &Path, remote: &str) -> Result<CommandOutput, AppError>;
    fn create_branch(
        &self,
        repo_root: &Path,
        name: &str,
        start_point: &str,
    ) -> Result<CommandOutput, AppError>;
    fn push_ref(
        &self,
        repo_root: &Path,
        remote: &str,
        local_ref: &str,
        remote_ref: &str,
        set_upstream: bool,
    ) -> Result<CommandOutput, AppError>;
}

pub trait Clock {
    fn now_rfc3339(&self) -> String;
}

pub trait IdGenerator {
    fn next(&self) -> ulid::Ulid;
}

pub trait PolicySource {
    fn read_yaml(&self, repo_root: &Path) -> Result<Option<String>, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    pub id: String,
    pub local_path: String,
    pub github_host: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub remote_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRecord {
    pub id: String,
    pub operation_id: String,
    pub sequence: i32,
    pub kind: String,
    pub status: StepStatus,
    pub detail_json: Option<String>,
    pub output_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRecord {
    pub id: String,
    pub project_id: String,
    pub kind: String,
    pub status: String,
    pub plan_json: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub snapshot_json: Option<String>,
    pub steps: Vec<StepRecord>,
}

pub struct NewProject<'a> {
    pub id: &'a str,
    pub local_path: &'a str,
    pub github_host: Option<&'a str>,
    pub owner: Option<&'a str>,
    pub repo: Option<&'a str>,
    pub remote_name: Option<&'a str>,
    pub now: &'a str,
}

pub trait OperationStore {
    fn upsert_project(&self, project: NewProject<'_>) -> Result<ProjectRecord, AppError>;
    fn get_project_by_path(&self, path: &Path) -> Result<Option<ProjectRecord>, AppError>;
    #[allow(clippy::too_many_arguments)]
    fn create_operation(
        &self,
        project_id: &str,
        id: &str,
        kind: &str,
        status: &str,
        plan: &OperationPlan,
        snapshot: &RepositorySnapshot,
        started_at: &str,
    ) -> Result<OperationRecord, AppError>;
    fn update_operation(
        &self,
        id: &str,
        status: &str,
        completed_at: Option<&str>,
    ) -> Result<(), AppError>;
    #[allow(clippy::too_many_arguments)]
    fn append_step(
        &self,
        operation_id: &str,
        id: &str,
        sequence: i32,
        kind: &str,
        status: StepStatus,
        detail_json: Option<&str>,
        now: &str,
    ) -> Result<StepRecord, AppError>;
    fn update_step(
        &self,
        id: &str,
        status: StepStatus,
        output_text: Option<&str>,
        completed_at: Option<&str>,
    ) -> Result<(), AppError>;
    fn list_operations(
        &self,
        project_id: &str,
        limit: u32,
    ) -> Result<Vec<OperationRecord>, AppError>;
}
