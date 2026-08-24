use crate::policy::PolicyFinding;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskClass {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
    CompensationNeeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum GitCommand {
    Fetch {
        remote: String,
    },
    CreateBranch {
        name: String,
        start_point: String,
    },
    PushRef {
        remote: String,
        local_ref: String,
        remote_ref: String,
        set_upstream: bool,
    },
    CommitPaths {
        message: String,
        paths: Vec<String>,
    },
    DeleteRemoteRef {
        remote: String,
        ref_name: String,
    },
}

impl GitCommand {
    pub fn step_kind(&self) -> &'static str {
        match self {
            GitCommand::Fetch { .. } => "fetch",
            GitCommand::CreateBranch { .. } => "create-branch",
            GitCommand::PushRef { .. } => "push-ref",
            GitCommand::CommitPaths { .. } => "commit-paths",
            GitCommand::DeleteRemoteRef { .. } => "delete-remote-ref",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationPlan {
    pub id: Ulid,
    pub kind: String,
    pub risk: RiskClass,
    pub summary: String,
    pub rationale: Vec<String>,
    pub commands: Vec<GitCommand>,
    pub preconditions: Vec<String>,
    pub findings: Vec<PolicyFinding>,
}
