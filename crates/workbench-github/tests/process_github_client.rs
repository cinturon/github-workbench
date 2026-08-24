use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pretty_assertions::assert_eq;
use workbench_application::ports::{CommandOutput, CommandSpec, GithubClient, ProcessRunner};
use workbench_application::AppError;
use workbench_github::ProcessGithubClient;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct RecordingRunner {
    calls: RefCell<Vec<CommandSpec>>,
    outputs: RefCell<VecDeque<Result<CommandOutput, AppError>>>,
}

impl RecordingRunner {
    fn with_outputs(outputs: impl IntoIterator<Item = CommandOutput>) -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            outputs: RefCell::new(outputs.into_iter().map(Ok).collect()),
        }
    }

    fn with_error(error: AppError) -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            outputs: RefCell::new(VecDeque::from([Err(error)])),
        }
    }
}

impl ProcessRunner for RecordingRunner {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput, AppError> {
        self.calls.borrow_mut().push(spec.clone());
        self.outputs
            .borrow_mut()
            .pop_front()
            .expect("a recorded output for every process call")
    }
}

fn output(exit_code: i32, stdout: &str, stderr: &str) -> CommandOutput {
    CommandOutput {
        exit_code,
        stdout: stdout.into(),
        stderr: stderr.into(),
    }
}

fn fixture_client(
    outputs: impl IntoIterator<Item = CommandOutput>,
) -> ProcessGithubClient<RecordingRunner> {
    ProcessGithubClient::with_program(
        RecordingRunner::with_outputs(outputs),
        PathBuf::from("/repo"),
        "gh-fixture",
    )
}

fn only_call(client: &ProcessGithubClient<RecordingRunner>) -> CommandSpec {
    let calls = client.runner().calls.borrow();
    assert_eq!(calls.len(), 1);
    calls[0].clone()
}

#[test]
fn lists_and_filters_runs_with_argv_only() {
    let client = fixture_client([output(0, include_str!("fixtures/workflow_runs.json"), "")]);

    let runs = client
        .list_workflow_runs(
            "acme",
            "widgets",
            "abc123",
            "github-workbench-test-01JABC.yml",
        )
        .unwrap();

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, 42);
    let call = only_call(&client);
    assert_eq!(call.program, "gh-fixture");
    assert_eq!(call.cwd, Path::new("/repo"));
    assert_eq!(
        call.args,
        vec![
            "api",
            "--method",
            "GET",
            "repos/acme/widgets/actions/runs",
            "-f",
            "head_sha=abc123",
            "-f",
            "per_page=100",
        ]
    );
}

#[test]
fn parses_a_completed_workflow_run() {
    let client = fixture_client([output(
        0,
        include_str!("fixtures/workflow_run_completed.json"),
        "",
    )]);

    let run = client.get_workflow_run("acme", "widgets", 42).unwrap();

    assert_eq!(run.id, 42);
    assert_eq!(run.status, "completed");
    assert_eq!(run.conclusion.as_deref(), Some("success"));
    assert_eq!(
        only_call(&client).args,
        vec!["api", "repos/acme/widgets/actions/runs/42"]
    );
}

#[test]
fn auth_status_failure_requires_authentication() {
    let client = fixture_client([output(1, "", "not logged in")]);

    let error = client.auth_status().unwrap_err();

    assert!(matches!(
        error,
        AppError::AuthRequired { ref detail } if detail == "not logged in"
    ));
    assert_eq!(only_call(&client).args, vec!["auth", "status"]);
}

#[test]
fn api_auth_and_scope_failures_require_authentication() {
    for stderr in [
        "HTTP 401: Bad credentials",
        "HTTP 403: Forbidden",
        "insufficient scope for this endpoint",
        "Resource not accessible by personal access token",
    ] {
        let client = fixture_client([output(1, "", stderr)]);

        let error = client.get_workflow_run("acme", "widgets", 42).unwrap_err();

        assert!(
            matches!(error, AppError::AuthRequired { .. }),
            "unexpected mapping for {stderr:?}: {error:?}"
        );
    }
}

#[test]
fn non_auth_api_failure_is_redacted_and_bounded() {
    let secret = "ghp_abcdefghijklmnopqrstuvwxyz012345";
    let stderr = format!("server rejected token {secret}");
    let client = fixture_client([output(2, "", &stderr)]);

    let error = client.get_workflow_run("acme", "widgets", 42).unwrap_err();

    match error {
        AppError::GithubFailed {
            program,
            args_summary,
            status,
            stderr_redacted,
        } => {
            assert_eq!(program, "gh-fixture");
            assert_eq!(args_summary, "api repos/acme/widgets/actions/runs/42");
            assert_eq!(status, 2);
            assert!(stderr_redacted.contains("[redacted]"));
            assert!(!stderr_redacted.contains(secret));
        }
        other => panic!("expected GithubFailed, got {other:?}"),
    }
}

#[test]
fn process_launch_failure_is_reported_as_github_failure() {
    let runner = RecordingRunner::with_error(AppError::GitUnavailable {
        detail: "ghp_abcdefghijklmnopqrstuvwxyz012345 was not found".into(),
    });
    let client = ProcessGithubClient::with_program(runner, PathBuf::from("/repo"), "missing-gh");

    let error = client.auth_status().unwrap_err();

    assert!(matches!(
        error,
        AppError::GithubFailed {
            ref program,
            status: -1,
            ref stderr_redacted,
            ..
        } if program == "missing-gh"
            && stderr_redacted.contains("[redacted]")
            && !stderr_redacted.contains("ghp_")
    ));
}

#[test]
fn downloads_named_artifact_with_argv_only() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("artifacts");
    let client = fixture_client([output(0, "", "")]);

    let returned = client
        .download_artifact_zip("acme", "widgets", 42, "gww-result", &dest)
        .unwrap();

    assert_eq!(returned, dest);
    assert!(dest.is_dir());
    assert_eq!(
        only_call(&client).args,
        vec![
            "run",
            "download",
            "42",
            "--repo",
            "acme/widgets",
            "--name",
            "gww-result",
            "--dir",
            dest.to_str().unwrap(),
        ]
    );
}

#[test]
fn missing_artifact_has_a_specific_error() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("artifacts");
    let client = fixture_client([output(1, "", "no valid artifacts found to download")]);

    let error = client
        .download_artifact_zip("acme", "widgets", 42, "gww-result", &dest)
        .unwrap_err();

    assert_eq!(
        error,
        AppError::ArtifactNotFound {
            run_id: 42,
            artifact_name: "gww-result".into(),
        }
    );
}

#[test]
fn downloads_run_logs_to_the_requested_path() {
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("run.log");
    let logs = include_str!("fixtures/run_logs.txt");
    let client = fixture_client([output(0, logs, "")]);

    let returned = client
        .download_run_logs("acme", "widgets", 42, &dest)
        .unwrap();

    assert_eq!(returned, dest);
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), logs);
    assert_eq!(
        only_call(&client).args,
        vec!["run", "view", "42", "--repo", "acme/widgets", "--log"]
    );
}

#[test]
fn new_uses_gww_gh_program_without_forwarding_the_control_variable() {
    let _guard = ENV_LOCK.lock().unwrap();
    let previous = std::env::var_os("GWW_GH_PROGRAM");
    std::env::set_var("GWW_GH_PROGRAM", "gh-from-env");
    let client = ProcessGithubClient::new(
        RecordingRunner::with_outputs([output(0, "", "")]),
        PathBuf::from("/repo"),
    );
    match previous {
        Some(value) => std::env::set_var("GWW_GH_PROGRAM", value),
        None => std::env::remove_var("GWW_GH_PROGRAM"),
    }

    client.auth_status().unwrap();

    let call = only_call(&client);
    assert_eq!(call.program, "gh-from-env");
    assert!(!call.env.iter().any(|(key, _)| key == "GWW_GH_PROGRAM"));
}
