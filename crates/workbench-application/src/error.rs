use std::fmt::Write as _;

use thiserror::Error;
use workbench_domain::WorkbenchError;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] WorkbenchError),

    #[error("git is not available: {detail}")]
    GitUnavailable { detail: String },

    #[error("git command failed: {program} {args_summary} (exit {status}): {stderr_redacted}")]
    GitFailed {
        program: String,
        args_summary: String,
        status: i32,
        stderr_redacted: String,
    },

    #[error("working tree is dirty ({0} path(s))", paths.len())]
    DirtyWorkingTree { paths: Vec<String> },

    #[error("could not resolve a unique remote")]
    RemoteNotResolved { candidates: Vec<String> },

    #[error("repository is not mapped to a Git remote")]
    RepositoryNotMapped,

    #[error("not a git repository: {path}")]
    NotAGitRepository { path: String },

    #[error("storage error: {detail}")]
    Storage { detail: String },

    #[error("I/O error at {path}: {detail}")]
    Io { path: String, detail: String },

    #[error("{message}")]
    Usage { message: String },

    #[error("GitHub authentication is required: {detail}")]
    AuthRequired { detail: String },

    #[error("remote test session `{session_id}` is still pending")]
    RemotePending { session_id: String },

    #[error("gh command failed: {program} {args_summary} (exit {status}): {stderr_redacted}")]
    GithubFailed {
        program: String,
        args_summary: String,
        status: i32,
        stderr_redacted: String,
    },

    #[error("artifact `{artifact_name}` was not found for workflow run `{run_id}`")]
    ArtifactNotFound { run_id: u64, artifact_name: String },

    #[error("action `{path}` uses unsupported runtime `{using}`")]
    ActionNotComposite { path: String, using: String },

    #[error("invalid test case `{path}`: {detail}")]
    TestCaseInvalid { path: String, detail: String },

    #[error("no workflow run matched session `{session_id}` at `{head_sha}`")]
    RunNotCorrelated {
        session_id: String,
        head_sha: String,
    },

    #[error("cleanup ref `{ref_name}` moved from `{expected}` to `{actual}`")]
    CleanupRefMoved {
        ref_name: String,
        expected: String,
        actual: String,
    },

    #[error("cleanup item `{item_id}` has invalid identity: {detail}")]
    CleanupIdentityMismatch { item_id: String, detail: String },

    #[error("remote test assertions failed for `{session_id}`: {failures:?}")]
    AssertionFailed {
        session_id: String,
        failures: Vec<String>,
    },

    #[error("{message}")]
    OperationFailed {
        message: String,
        changed: Vec<String>,
        unchanged: Vec<String>,
        retry_safe: bool,
        remediation: String,
    },
}

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Domain(
                WorkbenchError::PolicyBlocked { .. } | WorkbenchError::ProtectedBranchMisuse { .. },
            ) => 3,
            AppError::AuthRequired { .. } => 4,
            AppError::RemotePending { .. } => 5,
            AppError::Domain(WorkbenchError::InvalidPolicy { .. })
            | AppError::ActionNotComposite { .. }
            | AppError::TestCaseInvalid { .. }
            | AppError::Usage { .. }
            | AppError::Io { .. } => 2,
            _ => 1,
        }
    }

    pub fn user_report(&self) -> String {
        let (failed, changed, unchanged, retry_safe, remediation) = match self {
            AppError::OperationFailed {
                message,
                changed,
                unchanged,
                retry_safe,
                remediation,
            } => (
                message.clone(),
                changed.clone(),
                unchanged.clone(),
                *retry_safe,
                remediation.clone(),
            ),
            AppError::DirtyWorkingTree { paths } => (
                "Push is blocked because the working tree is dirty.".into(),
                Vec::new(),
                vec!["No Git refs were updated.".into()],
                true,
                format!(
                    "Commit or stash these paths, then retry: {}",
                    paths.join(", ")
                ),
            ),
            AppError::GitFailed {
                program,
                args_summary,
                status,
                stderr_redacted,
            } => (
                format!("{program} {args_summary} exited {status}: {stderr_redacted}"),
                Vec::new(),
                vec!["Later plan steps were not started.".into()],
                false,
                "Inspect the Git error, fix the repository state, then retry the command.".into(),
            ),
            AppError::GitUnavailable { detail } => (
                format!("The git executable is not available ({detail})."),
                Vec::new(),
                vec!["No repository changes were made.".into()],
                true,
                "Install Git and ensure it is on PATH, or set GWW_GIT_PROGRAM.".into(),
            ),
            AppError::AuthRequired { detail } => (
                format!("GitHub authentication is required ({detail})."),
                Vec::new(),
                vec!["No remote test or cleanup operation was started.".into()],
                true,
                "Run `gh auth login`, verify `gh auth status`, then retry.".into(),
            ),
            AppError::GithubFailed {
                program,
                args_summary,
                status,
                stderr_redacted,
            } => (
                format!("{program} {args_summary} exited {status}: {stderr_redacted}"),
                Vec::new(),
                vec!["Later GitHub operation steps were not started.".into()],
                false,
                "Inspect the redacted GitHub CLI error, fix authentication or repository access, then retry.".into(),
            ),
            AppError::RemotePending { session_id } => (
                format!("Remote test session `{session_id}` is still pending."),
                vec!["The remote test session was preserved for resumption.".into()],
                vec!["The workflow run has not completed.".into()],
                true,
                format!("Resume monitoring with `gww runs watch {session_id}`."),
            ),
            AppError::RunNotCorrelated {
                session_id,
                head_sha,
            } => (
                format!(
                    "No workflow run matched session `{session_id}` at commit `{head_sha}`."
                ),
                vec!["The remote test ref was pushed.".into()],
                vec!["A matching workflow run was not correlated.".into()],
                true,
                format!(
                    "Confirm Actions is enabled, then resume with `gww runs watch {session_id}`."
                ),
            ),
            AppError::CleanupRefMoved {
                ref_name,
                expected,
                actual,
            } => (
                format!(
                    "Cleanup ref `{ref_name}` moved from `{expected}` to `{actual}`."
                ),
                Vec::new(),
                vec!["The remote ref was not deleted.".into()],
                false,
                "Inspect the remote ref and enqueue cleanup again only after verifying its identity."
                    .into(),
            ),
            AppError::CleanupIdentityMismatch { item_id, detail } => (
                format!("Cleanup item `{item_id}` has invalid identity ({detail})."),
                Vec::new(),
                vec!["The remote ref was not deleted.".into()],
                false,
                "Inspect the stored session and cleanup identity before attempting deletion.".into(),
            ),
            AppError::AssertionFailed {
                session_id,
                failures,
            } => (
                format!(
                    "Remote test assertions failed for `{session_id}`: {}",
                    failures.join("; ")
                ),
                vec!["The workflow run and downloaded evidence were preserved.".into()],
                vec!["The remote test was not marked as passed.".into()],
                false,
                format!(
                    "Inspect the session evidence and workflow logs with `gww runs show {session_id}`."
                ),
            ),
            AppError::RemoteNotResolved { candidates } => (
                "Could not choose a unique Git remote.".into(),
                Vec::new(),
                vec!["No repository changes were made.".into()],
                true,
                format!(
                    "Pass --remote <name>. Candidates: {}",
                    candidates.join(", ")
                ),
            ),
            AppError::RepositoryNotMapped => (
                "This repository is not mapped to a Git remote / local project.".into(),
                Vec::new(),
                vec!["No repository changes were made.".into()],
                true,
                "Run `gww open <path>` (with --remote if several remotes exist).".into(),
            ),
            AppError::Domain(inner) => (
                inner.to_string(),
                Vec::new(),
                vec!["No repository changes were made.".into()],
                true,
                "Fix the policy or branch name and retry.".into(),
            ),
            other => (
                other.to_string(),
                Vec::new(),
                vec!["No repository changes were made.".into()],
                true,
                "See the error message above.".into(),
            ),
        };

        let mut out = String::new();
        let _ = writeln!(out, "What failed: {failed}");
        let _ = writeln!(
            out,
            "What already changed: {}",
            if changed.is_empty() {
                "nothing".into()
            } else {
                changed.join("; ")
            }
        );
        let _ = writeln!(
            out,
            "What did not happen: {}",
            if unchanged.is_empty() {
                "n/a".into()
            } else {
                unchanged.join("; ")
            }
        );
        let _ = writeln!(
            out,
            "Retry is safe: {}",
            if retry_safe {
                "yes"
            } else {
                "no, inspect journal first"
            }
        );
        let _ = writeln!(out, "Remediation: {remediation}");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use workbench_domain::WorkbenchError;

    #[test]
    fn policy_blocked_uses_exit_code_3() {
        let err = AppError::Domain(WorkbenchError::PolicyBlocked { findings: vec![] });
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn protected_branch_misuse_uses_exit_code_3() {
        let err = AppError::Domain(WorkbenchError::ProtectedBranchMisuse {
            branch: "main".into(),
        });
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn invalid_policy_uses_exit_code_2() {
        let err = AppError::Domain(WorkbenchError::InvalidPolicy { findings: vec![] });
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn git_failed_uses_exit_code_1() {
        let err = AppError::GitFailed {
            program: "git".into(),
            args_summary: "push github feature/x:feature/x".into(),
            status: 1,
            stderr_redacted: "rejected".into(),
        };
        assert_eq!(err.exit_code(), 1);
        let report = err.user_report();
        assert!(report.contains("What failed"));
        assert!(report.contains("retry"));
    }

    #[test]
    fn phase_three_exit_codes_are_stable() {
        assert_eq!(
            AppError::AuthRequired {
                detail: "run gh auth login".into()
            }
            .exit_code(),
            4
        );
        assert_eq!(
            AppError::RemotePending {
                session_id: "01JABC".into()
            }
            .exit_code(),
            5
        );
        assert_eq!(
            AppError::TestCaseInvalid {
                path: "test.yml".into(),
                detail: "bad runner".into(),
            }
            .exit_code(),
            2
        );
    }
}
