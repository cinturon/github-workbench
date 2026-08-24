use std::path::Path;

use ulid::Ulid;
use workbench_domain::operations::plan::{OperationPlan, RiskClass, StepStatus};
use workbench_domain::repository::RepositorySnapshot;

use crate::action_tests::{ExpectedRemoteRef, StoredSessionState};
use crate::executor::ExecuteOutcome;
use crate::ports::{
    CleanupItemRecord, Clock, GitClient, GithubClient, IdGenerator, OperationStore,
    TestSessionStore,
};
use crate::redact::{bound_output, redact};
use crate::use_cases::test_sessions::resolve_project;
use crate::AppError;

pub fn list_cleanup<G, S>(
    git: &G,
    store: &S,
    path: &Path,
) -> Result<Vec<CleanupItemRecord>, AppError>
where
    G: GitClient,
    S: OperationStore + TestSessionStore,
{
    let (_, project) = resolve_project(git, store, path)?;
    store.list_cleanup_items(&project.id)
}

pub fn plan_cleanup<G, S>(
    git: &G,
    store: &S,
    path: &Path,
    item_id: &str,
) -> Result<(OperationPlan, RepositorySnapshot, CleanupItemRecord), AppError>
where
    G: GitClient,
    S: OperationStore + TestSessionStore,
{
    let (root, project) = resolve_project(git, store, path)?;
    let item = store
        .get_cleanup_item(&project.id, item_id)?
        .ok_or_else(|| identity_error(item_id, "cleanup item was not found"))?;
    validate_item_shape(&item)?;
    let expected = parse_expected(&item)?;
    let mut snapshot = git.snapshot(&root)?;
    snapshot.selected_remote = Some(expected.identity.remote.clone());
    let plan = cleanup_plan(&item, &expected);
    Ok((plan, snapshot, item))
}

#[allow(clippy::too_many_arguments)]
pub fn execute_cleanup<G, H, S, C, I>(
    git: &G,
    github: &H,
    store: &S,
    clock: &C,
    ids: &I,
    path: &Path,
    item_id: &str,
) -> Result<ExecuteOutcome, AppError>
where
    G: GitClient,
    H: GithubClient,
    S: OperationStore + TestSessionStore,
    C: Clock,
    I: IdGenerator,
{
    let (root, project) = resolve_project(git, store, path)?;
    let item = store
        .get_cleanup_item(&project.id, item_id)?
        .ok_or_else(|| identity_error(item_id, "cleanup item was not found"))?;
    validate_item_shape(&item)?;
    let expected = parse_expected(&item)?;
    let session = store
        .get_test_session(&project.id, &expected.identity.session_id)?
        .ok_or_else(|| identity_error(item_id, "referenced test session was not found"))?;
    let state: StoredSessionState =
        serde_json::from_str(&session.result_json).map_err(|error| {
            identity_error(
                item_id,
                &format!("stored test session state is malformed: {error}"),
            )
        })?;
    validate_expected_identity(&item, &expected, &state)?;

    let owner = project
        .owner
        .as_deref()
        .ok_or(AppError::RepositoryNotMapped)?;
    let repo = project
        .repo
        .as_deref()
        .ok_or(AppError::RepositoryNotMapped)?;
    github.auth_status()?;
    let mut snapshot = git.snapshot(&root)?;
    snapshot.selected_remote = Some(expected.identity.remote.clone());
    let plan = cleanup_plan(&item, &expected);
    let outcome = execute_github_ref_cleanup(
        github,
        store,
        clock,
        ids,
        &project.id,
        &snapshot,
        &plan,
        owner,
        repo,
        &expected,
    )?;
    let now = clock.now_rfc3339();
    store.complete_cleanup_item(&item.id, &now)?;
    Ok(outcome)
}

#[allow(clippy::too_many_arguments)]
fn execute_github_ref_cleanup<H, S, C, I>(
    github: &H,
    store: &S,
    clock: &C,
    ids: &I,
    project_id: &str,
    snapshot: &RepositorySnapshot,
    plan: &OperationPlan,
    owner: &str,
    repo: &str,
    expected: &ExpectedRemoteRef,
) -> Result<ExecuteOutcome, AppError>
where
    H: GithubClient,
    S: OperationStore,
    C: Clock,
    I: IdGenerator,
{
    let operation_id = ids.next().to_string();
    let started_at = clock.now_rfc3339();
    store.create_operation(
        project_id,
        &operation_id,
        &plan.kind,
        "running",
        plan,
        snapshot,
        &started_at,
    )?;

    let step_id = ids.next().to_string();
    let detail_json = serde_json::to_string(&serde_json::json!({
        "type": "delete-github-ref-if-sha-matches",
        "owner": owner,
        "repo": repo,
        "ref_name": expected.identity.ref_name,
        "expected_sha": expected.commit_sha,
    }))
    .map_err(|error| AppError::Storage {
        detail: format!("could not serialize cleanup operation step: {error}"),
    })?;
    store.append_step(
        &operation_id,
        &step_id,
        1,
        "delete-github-ref-if-sha-matches",
        StepStatus::Pending,
        Some(&detail_json),
        &started_at,
    )?;
    store.update_step(&step_id, StepStatus::Running, None, None)?;

    if let Err(error) = github.delete_ref_if_sha_matches(
        owner,
        repo,
        &expected.identity.ref_name,
        &expected.commit_sha,
    ) {
        let completed_at = clock.now_rfc3339();
        let output = bound_output(&redact(&error.to_string()));
        store.update_step(
            &step_id,
            StepStatus::Failed,
            Some(&output),
            Some(&completed_at),
        )?;
        store.update_operation(&operation_id, "failed", Some(&completed_at))?;
        return Err(error);
    }

    let completed_at = clock.now_rfc3339();
    store.update_step(
        &step_id,
        StepStatus::Succeeded,
        Some("Matched the recorded SHA and deleted the ref through the GitHub API."),
        Some(&completed_at),
    )?;
    store.update_operation(&operation_id, "succeeded", Some(&completed_at))?;
    Ok(ExecuteOutcome {
        operation_id,
        status: "succeeded".into(),
        changed: vec![format!(
            "Deleted temporary GitHub ref `{owner}/{repo}:heads/{}` after matching `{}`.",
            expected.identity.ref_name, expected.commit_sha
        )],
    })
}

fn cleanup_plan(item: &CleanupItemRecord, expected: &ExpectedRemoteRef) -> OperationPlan {
    OperationPlan {
        id: Ulid::nil(),
        kind: "cleanup-remote-git-ref".into(),
        risk: RiskClass::Medium,
        summary: format!(
            "Delete temporary remote ref `{}/{}`",
            expected.identity.remote, expected.identity.ref_name
        ),
        rationale: vec![
            format!(
                "Cleanup item `{}` recorded this temporary test ref.",
                item.id
            ),
            "The GitHub API reads the authoritative ref SHA and requests deletion only when it matches the recorded SHA.".into(),
            "GitHub's REST API has a residual race between that read and deletion because it exposes no documented SHA precondition.".into(),
        ],
        commands: vec![],
        preconditions: vec![format!(
            "GitHub `heads/{}` still resolves to `{}` (remote `{}`).",
            expected.identity.ref_name,
            expected.commit_sha,
            expected.identity.remote
        )],
        findings: vec![],
    }
}

fn validate_item_shape(item: &CleanupItemRecord) -> Result<(), AppError> {
    if item.resource_kind != "remote-git-ref" {
        return Err(identity_error(
            &item.id,
            &format!("unsupported resource kind `{}`", item.resource_kind),
        ));
    }
    if item.status != "pending" {
        return Err(identity_error(
            &item.id,
            &format!("cleanup status is `{}`, expected `pending`", item.status),
        ));
    }
    Ok(())
}

fn parse_expected(item: &CleanupItemRecord) -> Result<ExpectedRemoteRef, AppError> {
    serde_json::from_str(&item.expected_identity).map_err(|error| {
        identity_error(
            &item.id,
            &format!("expected remote-ref identity is malformed: {error}"),
        )
    })
}

fn validate_expected_identity(
    item: &CleanupItemRecord,
    expected: &ExpectedRemoteRef,
    state: &StoredSessionState,
) -> Result<(), AppError> {
    if expected.identity != state.plan.cleanup_identity {
        return Err(identity_error(
            &item.id,
            "cleanup identity does not match the stored session plan",
        ));
    }
    if state.pushed_sha.as_deref() != Some(expected.commit_sha.as_str()) {
        return Err(identity_error(
            &item.id,
            "expected commit does not match the stored pushed commit",
        ));
    }
    let resource_id = format!(
        "{}/{}",
        expected.identity.remote, expected.identity.ref_name
    );
    if item.resource_id != resource_id {
        return Err(identity_error(
            &item.id,
            "resource id does not match the expected remote ref",
        ));
    }
    Ok(())
}

fn identity_error(item_id: &str, detail: &str) -> AppError {
    AppError::CleanupIdentityMismatch {
        item_id: item_id.into(),
        detail: detail.into(),
    }
}
