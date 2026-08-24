use std::path::{Path, PathBuf};

use serde::Deserialize;
use workbench_application::ports::{
    CommandOutput, CommandSpec, GithubClient, ProcessRunner, WorkflowRunDetail, WorkflowRunSummary,
};
use workbench_application::redact::{bound_output, redact};
use workbench_application::AppError;

use crate::env::sanitized_env;
use crate::parser::{parse_run, parse_runs};

#[derive(Deserialize)]
struct RefResponse {
    object: RefObject,
}

#[derive(Deserialize)]
struct RefObject {
    sha: String,
}

pub struct ProcessGithubClient<R> {
    runner: R,
    cwd: PathBuf,
    gh_program: String,
}

impl<R> ProcessGithubClient<R> {
    pub fn new(runner: R, cwd: PathBuf) -> Self {
        Self {
            runner,
            cwd,
            gh_program: std::env::var("GWW_GH_PROGRAM").unwrap_or_else(|_| "gh".into()),
        }
    }

    pub fn with_program(runner: R, cwd: PathBuf, program: impl Into<String>) -> Self {
        Self {
            runner,
            cwd,
            gh_program: program.into(),
        }
    }

    pub fn runner(&self) -> &R {
        &self.runner
    }
}

impl<R: ProcessRunner> ProcessGithubClient<R> {
    fn run(&self, args: Vec<String>) -> Result<CommandOutput, AppError> {
        self.runner
            .run(&CommandSpec {
                program: self.gh_program.clone(),
                args: args.clone(),
                cwd: self.cwd.clone(),
                env: sanitized_env(),
            })
            .map_err(|error| match error {
                AppError::GitUnavailable { detail } => AppError::GithubFailed {
                    program: self.gh_program.clone(),
                    args_summary: args.join(" "),
                    status: -1,
                    stderr_redacted: bound_output(&redact(&detail)),
                },
                other => other,
            })
    }

    fn checked(&self, args: Vec<String>) -> Result<CommandOutput, AppError> {
        let output = self.run(args.clone())?;
        if output.exit_code == 0 {
            return Ok(output);
        }

        let stderr_redacted = bound_output(&redact(&output.stderr));
        if is_auth_failure(&output.stderr) {
            return Err(AppError::AuthRequired {
                detail: stderr_redacted,
            });
        }
        Err(AppError::GithubFailed {
            program: self.gh_program.clone(),
            args_summary: args.join(" "),
            status: output.exit_code,
            stderr_redacted,
        })
    }
}

fn is_auth_failure(stderr: &str) -> bool {
    let message = stderr.to_ascii_lowercase();
    [
        "http 401",
        "http 403",
        "not logged in",
        "authentication",
        "insufficient scope",
        "resource not accessible by personal access token",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

impl<R: ProcessRunner> GithubClient for ProcessGithubClient<R> {
    fn auth_status(&self) -> Result<(), AppError> {
        let output = self.run(vec!["auth".into(), "status".into()])?;
        if output.exit_code == 0 {
            Ok(())
        } else {
            Err(AppError::AuthRequired {
                detail: bound_output(&redact(&output.stderr)),
            })
        }
    }

    fn delete_ref_if_sha_matches(
        &self,
        owner: &str,
        repo: &str,
        ref_name: &str,
        expected_sha: &str,
    ) -> Result<(), AppError> {
        let get_args = vec![
            "api".into(),
            "--method".into(),
            "GET".into(),
            format!("repos/{owner}/{repo}/git/ref/heads/{ref_name}"),
        ];
        let output = self.checked(get_args)?;
        let response: RefResponse =
            serde_json::from_str(&output.stdout).map_err(|error| AppError::GithubFailed {
                program: self.gh_program.clone(),
                args_summary: "parse GitHub ref response".into(),
                status: 0,
                stderr_redacted: bound_output(&redact(&error.to_string())),
            })?;
        if response.object.sha != expected_sha {
            return Err(AppError::CleanupRefMoved {
                ref_name: ref_name.into(),
                expected: expected_sha.into(),
                actual: response.object.sha,
            });
        }

        // GitHub's REST delete-ref endpoint has no documented SHA precondition.
        // This authoritative GET check substantially narrows the race compared
        // with an unconditional Git push deletion, though a residual GET/DELETE
        // race remains.
        self.checked(vec![
            "api".into(),
            "--method".into(),
            "DELETE".into(),
            format!("repos/{owner}/{repo}/git/refs/heads/{ref_name}"),
        ])?;
        Ok(())
    }

    fn list_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        head_sha: &str,
        workflow_file_name: &str,
    ) -> Result<Vec<WorkflowRunSummary>, AppError> {
        let output = self.checked(vec![
            "api".into(),
            "--method".into(),
            "GET".into(),
            format!("repos/{owner}/{repo}/actions/runs"),
            "-f".into(),
            format!("head_sha={head_sha}"),
            "-f".into(),
            "per_page=100".into(),
        ])?;
        parse_runs(&output.stdout, head_sha, workflow_file_name)
    }

    fn get_workflow_run(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<WorkflowRunDetail, AppError> {
        let output = self.checked(vec![
            "api".into(),
            format!("repos/{owner}/{repo}/actions/runs/{run_id}"),
        ])?;
        parse_run(&output.stdout)
    }

    fn download_artifact_zip(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        artifact_name: &str,
        dest_dir: &Path,
    ) -> Result<PathBuf, AppError> {
        std::fs::create_dir_all(dest_dir).map_err(|error| AppError::Io {
            path: dest_dir.display().to_string(),
            detail: error.to_string(),
        })?;
        let download = self.checked(vec![
            "run".into(),
            "download".into(),
            run_id.to_string(),
            "--repo".into(),
            format!("{owner}/{repo}"),
            "--name".into(),
            artifact_name.into(),
            "--dir".into(),
            dest_dir.display().to_string(),
        ]);
        if let Err(error) = download {
            let missing = matches!(
                &error,
                AppError::GithubFailed {
                    stderr_redacted,
                    ..
                } if {
                    let message = stderr_redacted.to_ascii_lowercase();
                    message.contains("no artifacts found")
                        || message.contains("no valid artifacts found")
                }
            );
            if missing {
                return Err(AppError::ArtifactNotFound {
                    run_id,
                    artifact_name: artifact_name.into(),
                });
            }
            return Err(error);
        }
        Ok(dest_dir.to_path_buf())
    }

    fn download_run_logs(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        dest_path: &Path,
    ) -> Result<PathBuf, AppError> {
        let output = self.checked(vec![
            "run".into(),
            "view".into(),
            run_id.to_string(),
            "--repo".into(),
            format!("{owner}/{repo}"),
            "--log".into(),
        ])?;
        std::fs::write(dest_path, output.stdout).map_err(|error| AppError::Io {
            path: dest_path.display().to_string(),
            detail: error.to_string(),
        })?;
        Ok(dest_path.to_path_buf())
    }
}
