use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Duration as PollDuration;

use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};
use workbench_domain::operations::plan::{GitCommand, OperationPlan, RiskClass};
use workbench_domain::testing::{
    evaluate_assertions, generate_workflow, normalize_test_case, parse_action_definition,
    parse_test_case_yaml, remote_test_branch, workflow_file_path, TestingError,
    RESULT_ARTIFACT_NAME, RESULT_MANIFEST_FILE,
};

use crate::action_tests::{
    CleanupIdentity, ExpectedRemoteRef, RemoteTestResult, RemoteTestSessionPlan,
    StoredSessionState, TestSessionStatus,
};
use crate::executor::execute_plan;
use crate::policy_source::load_policy;
use crate::ports::{
    Clock, GitClient, GithubClient, IdGenerator, NewCleanupItem, NewTestSession, OperationStore,
    PolicySource, Sleeper, TestSessionStore, TestSessionUpdate, WorkflowRunSummary,
};
use crate::redact::redact;
use crate::remote::resolve_remote;
use crate::use_cases::action_discovery::discover_action_tests;
use crate::use_cases::test_sessions::resolve_project;
use crate::AppError;

const CORRELATION_ATTEMPTS: usize = 40;
const POLLING_ATTEMPTS: usize = 600;
const POLL_INTERVAL: PollDuration = PollDuration::from_secs(3);

#[allow(clippy::too_many_arguments)]
pub fn plan_remote_test<G, S, P, I>(
    git: &G,
    store: &S,
    policy_source: &P,
    ids: &I,
    path: &Path,
    test_name: &str,
    remote_flag: Option<&str>,
) -> Result<RemoteTestSessionPlan, AppError>
where
    G: GitClient,
    S: OperationStore,
    P: PolicySource,
    I: IdGenerator,
{
    let root = git.resolve_toplevel(path)?;
    let snapshot = git.snapshot(&root)?;
    if !snapshot.dirty_paths.is_empty() {
        return Err(AppError::DirtyWorkingTree {
            paths: snapshot.dirty_paths,
        });
    }
    if snapshot.detached_head {
        return Err(AppError::Usage {
            message: "detached HEAD cannot run a remote action test".into(),
        });
    }
    let base_sha = snapshot.head_oid.clone().ok_or_else(|| AppError::Usage {
        message: "remote action tests require a repository commit".into(),
    })?;

    let (policy, _) = load_policy(policy_source, &root)?;
    let project = store
        .get_project_by_path(&root)?
        .ok_or(AppError::RepositoryNotMapped)?;
    let mapped_remote = project
        .remote_name
        .as_deref()
        .ok_or(AppError::RepositoryNotMapped)?;
    let remote = resolve_remote(&snapshot.remotes, Some(mapped_remote), remote_flag)?;
    let owner = project.owner.clone().ok_or(AppError::RepositoryNotMapped)?;
    let repo = project.repo.clone().ok_or(AppError::RepositoryNotMapped)?;

    let catalog = discover_action_tests(&root)?;
    let matching_tests: Vec<_> = catalog
        .tests
        .iter()
        .filter(|test| test.name == test_name)
        .collect();
    let test = match matching_tests.as_slice() {
        [test] => *test,
        [] => {
            return Err(AppError::TestCaseInvalid {
                path: test_name.into(),
                detail: format!("test case `{test_name}` was not found"),
            })
        }
        _ => {
            return Err(AppError::TestCaseInvalid {
                path: test_name.into(),
                detail: format!("more than one test case is named `{test_name}`"),
            })
        }
    };
    let test_path = root.join(&test.path);
    let test_path_display = test.path.to_string_lossy().into_owned();
    let test_yaml = read_text(&test_path)?;
    let test_case = parse_test_case_yaml(&test_yaml)
        .map_err(|error| map_testing_error(error, &test_path_display, &test_path_display))?;
    let action_manifest =
        resolve_action_manifest(&root, &test_case.action.path).map_err(|detail| {
            AppError::TestCaseInvalid {
                path: test_path_display.clone(),
                detail,
            }
        })?;
    let action_relative = action_manifest
        .strip_prefix(&root)
        .map_err(|error| AppError::TestCaseInvalid {
            path: test_path_display.clone(),
            detail: error.to_string(),
        })?
        .to_string_lossy()
        .into_owned();
    let action_yaml = read_text(&action_manifest)?;
    let action = parse_action_definition(&action_relative, &action_yaml)
        .map_err(|error| map_testing_error(error, &test_path_display, &action_relative))?;
    let test_plan = normalize_test_case(
        test_case,
        &action,
        policy.remote_testing.default_timeout_minutes,
    )
    .map_err(|error| map_testing_error(error, &test_path_display, &action_relative))?;

    let session_ulid = ids.next();
    let session_id = session_ulid.to_string();
    let branch = remote_test_branch(&policy.remote_testing.branch_prefix, &session_id)
        .map_err(|error| map_testing_error(error, &test_path_display, &action_relative))?;
    let workflow_path = workflow_file_path(&session_id)
        .map_err(|error| map_testing_error(error, &test_path_display, &action_relative))?;
    let workflow_file_name = Path::new(&workflow_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::TestCaseInvalid {
            path: test_path_display.clone(),
            detail: "generated workflow path has no file name".into(),
        })?
        .to_string();
    let workflow_yaml = generate_workflow(&test_plan, &session_id, &branch)
        .map_err(|error| map_testing_error(error, &test_path_display, &action_relative))?;

    let cleanup_identity = CleanupIdentity {
        remote: remote.clone(),
        ref_name: branch.clone(),
        session_id: session_id.clone(),
    };
    let mut rationale = vec![
        format!("Generate `{workflow_path}` for this isolated test run."),
        format!("Push temporary ref `{remote}/{branch}` without force."),
        format!("Download the `{RESULT_ARTIFACT_NAME}` result artifact and run logs."),
        "The temporary remote ref requires manual cleanup if automated cleanup cannot run.".into(),
    ];
    if existing_workflow_has_push_trigger(&root)? {
        rationale.push(
            "Warning: an existing workflow contains `push:` and may also run for the temporary ref."
                .into(),
        );
    }
    let git_plan = OperationPlan {
        id: session_ulid,
        kind: "remote-action-test".into(),
        risk: RiskClass::Medium,
        summary: format!("Run remote action test `{test_name}` on `{branch}`"),
        rationale,
        commands: vec![
            GitCommand::CreateBranch {
                name: branch.clone(),
                start_point: base_sha.clone(),
            },
            GitCommand::CommitPaths {
                message: format!("chore: add GitHub Workbench test {session_id}"),
                paths: vec![workflow_path.clone()],
            },
            GitCommand::PushRef {
                remote: remote.clone(),
                local_ref: branch.clone(),
                remote_ref: branch,
                set_upstream: false,
            },
        ],
        preconditions: vec![
            "The working tree remains clean.".into(),
            format!("HEAD remains at `{base_sha}`."),
        ],
        findings: vec![],
    };

    Ok(RemoteTestSessionPlan {
        project_id: project.id,
        repo_root: root,
        owner,
        repo,
        remote,
        base_sha,
        session_id,
        workflow_file_name,
        workflow_path,
        workflow_yaml,
        assertions: test_plan.assertions.clone(),
        test_plan,
        successful_ref_retention: policy.remote_testing.successful_ref_retention,
        failed_ref_retention: policy.remote_testing.failed_ref_retention,
        cleanup_identity,
        git_plan,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn execute_remote_test<G, H, S, C, I, L>(
    git: &G,
    github: &H,
    store: &S,
    clock: &C,
    ids: &I,
    sleeper: &L,
    plan: &RemoteTestSessionPlan,
    evidence_root: &Path,
) -> Result<RemoteTestResult, AppError>
where
    G: GitClient,
    H: GithubClient,
    S: OperationStore + TestSessionStore,
    C: Clock,
    I: IdGenerator,
    L: Sleeper,
{
    github.auth_status()?;

    let snapshot = git.snapshot(&plan.repo_root)?;
    if !snapshot.dirty_paths.is_empty() {
        return Err(AppError::DirtyWorkingTree {
            paths: snapshot.dirty_paths,
        });
    }
    if snapshot.head_oid.as_deref() != Some(plan.base_sha.as_str()) {
        return Err(AppError::OperationFailed {
            message: "HEAD changed after remote-test planning".into(),
            changed: Vec::new(),
            unchanged: vec![
                "The generated workflow was not written.".into(),
                "The temporary branch was not pushed.".into(),
            ],
            retry_safe: true,
            remediation: "Create a new remote-test plan from the current HEAD.".into(),
        });
    }

    let workflow_path = plan.repo_root.join(&plan.workflow_path);
    fs::create_dir_all(workflow_path.parent().unwrap())
        .map_err(|error| io_error(&workflow_path, error))?;
    fs::write(&workflow_path, &plan.workflow_yaml)
        .map_err(|error| io_error(&workflow_path, error))?;

    execute_plan(
        git,
        store,
        clock,
        ids,
        &plan.project_id,
        &snapshot,
        &plan.git_plan,
    )?;

    let pushed_sha =
        git.rev_parse(&plan.repo_root, "HEAD")?
            .ok_or_else(|| AppError::OperationFailed {
                message: "could not resolve the pushed test commit".into(),
                changed: vec![format!(
                    "Temporary branch `{}` was created.",
                    plan.cleanup_identity.ref_name
                )],
                unchanged: vec!["Run correlation was not started.".into()],
                retry_safe: false,
                remediation: "Inspect the operation journal before retrying.".into(),
            })?;

    let state = StoredSessionState {
        plan: plan.clone(),
        pushed_sha: Some(pushed_sha.clone()),
        result: None,
    };
    let state_json = serialize_state(&state)?;
    let row_id = ids.next().to_string();
    let now = clock.now_rfc3339();
    store.create_test_session(NewTestSession {
        id: &row_id,
        project_id: &plan.project_id,
        session_id: &plan.session_id,
        commit_sha: &pushed_sha,
        remote_ref: &plan.cleanup_identity.ref_name,
        workflow_name: &plan.workflow_file_name,
        status: TestSessionStatus::Pushed,
        result_json: &state_json,
        now: &now,
    })?;

    watch_session(
        git,
        github,
        store,
        clock,
        ids,
        sleeper,
        &plan.repo_root,
        &plan.session_id,
        evidence_root,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn watch_session<G, H, S, C, I, L>(
    git: &G,
    github: &H,
    store: &S,
    clock: &C,
    ids: &I,
    sleeper: &L,
    path: &Path,
    session_id: &str,
    evidence_root: &Path,
    wait: bool,
) -> Result<RemoteTestResult, AppError>
where
    G: GitClient,
    H: GithubClient,
    S: OperationStore + TestSessionStore,
    C: Clock,
    I: IdGenerator,
    L: Sleeper,
{
    github.auth_status()?;
    let (_, project) = resolve_project(git, store, path)?;
    let session = store
        .get_test_session(&project.id, session_id)?
        .ok_or_else(|| AppError::Usage {
            message: format!("remote test session `{session_id}` was not found"),
        })?;
    let mut state: StoredSessionState =
        serde_json::from_str(&session.result_json).map_err(|error| AppError::Storage {
            detail: format!("could not deserialize remote test session `{session_id}`: {error}"),
        })?;
    if let Some(result) = &state.result {
        return if result.passed {
            Ok(result.clone())
        } else {
            Err(assertion_error(result))
        };
    }
    let pushed_sha = state.pushed_sha.clone().ok_or_else(|| AppError::Storage {
        detail: format!("remote test session `{session_id}` has no pushed commit"),
    })?;

    let run_id = match session.run_id {
        Some(run_id) => run_id,
        None => correlate_run(
            github,
            store,
            clock,
            sleeper,
            &project.id,
            &mut state,
            &pushed_sha,
            wait,
        )?,
    };

    let detail = poll_run(
        github,
        store,
        clock,
        sleeper,
        &project.id,
        &mut state,
        run_id,
        wait,
    )?;
    let conclusion = detail
        .conclusion
        .clone()
        .ok_or_else(|| AppError::OperationFailed {
            message: format!("completed workflow run `{run_id}` has no conclusion"),
            changed: vec!["The temporary remote test ref was pushed.".into()],
            unchanged: vec!["Remote test assertions were not evaluated.".into()],
            retry_safe: true,
            remediation: format!("Inspect the workflow run at {}.", detail.html_url),
        })?;

    let evidence_dir = evidence_root.join(session_id);
    fs::create_dir_all(&evidence_dir).map_err(|error| io_error(&evidence_dir, error))?;
    match github.download_artifact_zip(
        &state.plan.owner,
        &state.plan.repo,
        run_id,
        RESULT_ARTIFACT_NAME,
        &evidence_dir,
    ) {
        Ok(_) | Err(AppError::ArtifactNotFound { .. }) => {}
        Err(error) => return Err(error),
    }
    let requested_logs_path = evidence_dir.join("run.log");
    let logs_path = github.download_run_logs(
        &state.plan.owner,
        &state.plan.repo,
        run_id,
        &requested_logs_path,
    )?;
    let manifest_path = evidence_dir.join(RESULT_MANIFEST_FILE);
    let manifest_json = if manifest_path.is_file() {
        Some(read_text(&manifest_path)?)
    } else {
        None
    };
    let logs = read_text(&logs_path)?;
    let assertion_report = evaluate_assertions(
        &state.plan.assertions,
        &conclusion,
        manifest_json.as_deref(),
        &logs,
        &detail.html_url,
    );
    fs::write(&logs_path, redact(&logs)).map_err(|error| io_error(&logs_path, error))?;

    let result = RemoteTestResult {
        session_id: session_id.into(),
        run_id,
        run_url: detail.html_url,
        conclusion,
        passed: assertion_report.passed,
        assertion_report,
        manifest_path: manifest_json.map(|_| manifest_path),
        logs_path,
    };
    state.result = Some(result.clone());
    persist_session(
        store,
        clock,
        &project.id,
        &state,
        Some(run_id),
        if result.passed {
            TestSessionStatus::Passed
        } else {
            TestSessionStatus::Failed
        },
        Some(&evidence_dir),
    )?;
    enqueue_cleanup(store, clock, ids, &project.id, &state, result.passed)?;

    if result.passed {
        Ok(result)
    } else {
        Err(assertion_error(&result))
    }
}

#[allow(clippy::too_many_arguments)]
fn correlate_run<H, S, C, L>(
    github: &H,
    store: &S,
    clock: &C,
    sleeper: &L,
    project_id: &str,
    state: &mut StoredSessionState,
    pushed_sha: &str,
    wait: bool,
) -> Result<u64, AppError>
where
    H: GithubClient,
    S: TestSessionStore,
    C: Clock,
    L: Sleeper,
{
    for attempt in 0..CORRELATION_ATTEMPTS {
        let summaries = github.list_workflow_runs(
            &state.plan.owner,
            &state.plan.repo,
            pushed_sha,
            &state.plan.workflow_file_name,
        )?;
        let matches: Vec<_> = summaries
            .into_iter()
            .filter(|run| exact_run_match(run, pushed_sha, &state.plan.workflow_file_name))
            .collect();
        match matches.as_slice() {
            [run] => {
                let status = session_status(&run.status);
                persist_session(store, clock, project_id, state, Some(run.id), status, None)?;
                return Ok(run.id);
            }
            [] if !wait => {
                return Err(AppError::RemotePending {
                    session_id: state.plan.session_id.clone(),
                })
            }
            [] if attempt + 1 < CORRELATION_ATTEMPTS => sleeper.sleep(POLL_INTERVAL),
            [] => {
                return Err(AppError::RunNotCorrelated {
                    session_id: state.plan.session_id.clone(),
                    head_sha: pushed_sha.into(),
                })
            }
            _ => {
                return Err(AppError::OperationFailed {
                    message: format!(
                        "more than one workflow run matched session `{}`",
                        state.plan.session_id
                    ),
                    changed: vec!["The temporary remote test ref was pushed.".into()],
                    unchanged: vec!["No workflow run was selected.".into()],
                    retry_safe: false,
                    remediation: "Inspect the matching workflow runs before resuming.".into(),
                })
            }
        }
    }
    unreachable!("correlation loop always returns")
}

#[allow(clippy::too_many_arguments)]
fn poll_run<H, S, C, L>(
    github: &H,
    store: &S,
    clock: &C,
    sleeper: &L,
    project_id: &str,
    state: &mut StoredSessionState,
    run_id: u64,
    wait: bool,
) -> Result<crate::ports::WorkflowRunDetail, AppError>
where
    H: GithubClient,
    S: TestSessionStore,
    C: Clock,
    L: Sleeper,
{
    for attempt in 0..POLLING_ATTEMPTS {
        let detail = github.get_workflow_run(&state.plan.owner, &state.plan.repo, run_id)?;
        if detail.status == "completed" {
            return Ok(detail);
        }

        persist_session(
            store,
            clock,
            project_id,
            state,
            Some(run_id),
            session_status(&detail.status),
            None,
        )?;
        if !wait || attempt + 1 == POLLING_ATTEMPTS {
            return Err(AppError::RemotePending {
                session_id: state.plan.session_id.clone(),
            });
        }
        sleeper.sleep(POLL_INTERVAL);
    }
    unreachable!("polling loop always returns")
}

fn exact_run_match(run: &WorkflowRunSummary, pushed_sha: &str, workflow_file_name: &str) -> bool {
    run.head_sha == pushed_sha
        && Path::new(&run.path)
            .file_name()
            .and_then(|name| name.to_str())
            == Some(workflow_file_name)
}

fn session_status(status: &str) -> TestSessionStatus {
    match status {
        "queued" => TestSessionStatus::Queued,
        "in_progress" => TestSessionStatus::InProgress,
        _ => TestSessionStatus::Pushed,
    }
}

fn persist_session<S, C>(
    store: &S,
    clock: &C,
    project_id: &str,
    state: &StoredSessionState,
    run_id: Option<u64>,
    status: TestSessionStatus,
    evidence_dir: Option<&Path>,
) -> Result<(), AppError>
where
    S: TestSessionStore,
    C: Clock,
{
    let state_json = serialize_state(state)?;
    let evidence_dir = evidence_dir.map(|path| path.to_string_lossy().into_owned());
    let now = clock.now_rfc3339();
    store.update_test_session(TestSessionUpdate {
        project_id,
        session_id: &state.plan.session_id,
        run_id,
        status,
        result_json: &state_json,
        evidence_dir: evidence_dir.as_deref(),
        now: &now,
    })
}

fn enqueue_cleanup<S, C, I>(
    store: &S,
    clock: &C,
    ids: &I,
    project_id: &str,
    state: &StoredSessionState,
    passed: bool,
) -> Result<(), AppError>
where
    S: TestSessionStore,
    C: Clock,
    I: IdGenerator,
{
    let pushed_sha = state.pushed_sha.as_ref().ok_or_else(|| AppError::Storage {
        detail: format!(
            "remote test session `{}` has no pushed commit",
            state.plan.session_id
        ),
    })?;
    let expected = ExpectedRemoteRef {
        identity: state.plan.cleanup_identity.clone(),
        commit_sha: pushed_sha.clone(),
    };
    let expected_identity =
        serde_json::to_string(&expected).map_err(|error| AppError::Storage {
            detail: format!("could not serialize cleanup identity: {error}"),
        })?;
    let now = clock.now_rfc3339();
    let retention = if passed {
        state.plan.successful_ref_retention.0
    } else {
        state.plan.failed_ref_retention.0
    };
    let retention = i64::try_from(retention).map_err(|_| AppError::Storage {
        detail: "remote test retention is too large".into(),
    })?;
    let due_at = OffsetDateTime::parse(&now, &Rfc3339)
        .map_err(|error| AppError::Storage {
            detail: format!("invalid clock timestamp `{now}`: {error}"),
        })?
        .checked_add(Duration::hours(retention))
        .ok_or_else(|| AppError::Storage {
            detail: "cleanup due time is outside the supported range".into(),
        })?
        .format(&Rfc3339)
        .map_err(|error| AppError::Storage {
            detail: format!("could not format cleanup due time: {error}"),
        })?;
    let item_id = ids.next().to_string();
    let resource_id = format!(
        "{}/{}",
        expected.identity.remote, expected.identity.ref_name
    );
    store.enqueue_cleanup(NewCleanupItem {
        id: &item_id,
        project_id,
        resource_kind: "remote-git-ref",
        resource_id: &resource_id,
        expected_identity: &expected_identity,
        due_at: &due_at,
        now: &now,
    })?;
    Ok(())
}

fn assertion_error(result: &RemoteTestResult) -> AppError {
    AppError::AssertionFailed {
        session_id: result.session_id.clone(),
        failures: result
            .assertion_report
            .failures
            .iter()
            .map(|failure| {
                format!(
                    "{}: expected {}, actual {}",
                    failure.rule, failure.expected, failure.actual
                )
            })
            .collect(),
    }
}

fn resolve_action_manifest(repo_root: &Path, action_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(action_path);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("action.path must stay inside the repository".into());
    }
    let candidate = repo_root.join(relative);
    if candidate.is_file()
        && matches!(
            candidate.file_name().and_then(|name| name.to_str()),
            Some("action.yml" | "action.yaml")
        )
    {
        return Ok(candidate);
    }
    for file_name in ["action.yml", "action.yaml"] {
        let manifest = candidate.join(file_name);
        if manifest.is_file() {
            return Ok(manifest);
        }
    }
    Err(format!(
        "action path `{action_path}` does not contain action.yml or action.yaml"
    ))
}

fn existing_workflow_has_push_trigger(repo_root: &Path) -> Result<bool, AppError> {
    let workflows = repo_root.join(".github/workflows");
    let entries = match fs::read_dir(&workflows) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(io_error(&workflows, error)),
    };
    for entry in entries {
        let entry = entry.map_err(|error| io_error(&workflows, error))?;
        let path = entry.path();
        if !entry
            .file_type()
            .map_err(|error| io_error(&path, error))?
            .is_file()
        {
            continue;
        }
        if read_text(&path)?.contains("push:") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn map_testing_error(error: TestingError, test_path: &str, action_path: &str) -> AppError {
    match error {
        TestingError::ActionNotComposite { using } => AppError::ActionNotComposite {
            path: action_path.into(),
            using,
        },
        other => AppError::TestCaseInvalid {
            path: test_path.into(),
            detail: other.to_string(),
        },
    }
}

fn serialize_state(state: &StoredSessionState) -> Result<String, AppError> {
    serde_json::to_string(state).map_err(|error| AppError::Storage {
        detail: format!("could not serialize remote test session: {error}"),
    })
}

fn read_text(path: &Path) -> Result<String, AppError> {
    fs::read_to_string(path).map_err(|error| io_error(path, error))
}

fn io_error(path: &Path, error: std::io::Error) -> AppError {
    AppError::Io {
        path: path.display().to_string(),
        detail: error.to_string(),
    }
}
