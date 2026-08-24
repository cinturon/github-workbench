use crate::policy::PolicyFinding;
use crate::workflow::state::WorkflowState;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkbenchError {
    #[error("invalid policy: {0} finding(s)", findings.len())]
    InvalidPolicy { findings: Vec<PolicyFinding> },

    #[error("policy blocked operation: {0} finding(s)", findings.len())]
    PolicyBlocked { findings: Vec<PolicyFinding> },

    #[error("invalid branch name: {reason}")]
    InvalidBranchName { reason: String },

    #[error("refusing to use protected branch `{branch}` for feature work")]
    ProtectedBranchMisuse { branch: String },

    #[error("illegal workflow transition from {from:?} to {to:?}")]
    IllegalTransition {
        from: WorkflowState,
        to: WorkflowState,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{PolicyFinding, Severity};

    #[test]
    fn display_includes_finding_count() {
        let err = WorkbenchError::InvalidPolicy {
            findings: vec![PolicyFinding {
                rule_id: "schema-version".into(),
                severity: Severity::Blocker,
                expected: "1".into(),
                actual: "2".into(),
                message: "unsupported".into(),
                remediation: "use schema-version 1".into(),
            }],
        };
        assert!(err.to_string().contains("1 finding"));
    }
}
