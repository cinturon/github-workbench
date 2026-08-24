use std::path::Path;

use ulid::Ulid;
use workbench_domain::operations::plan::{GitCommand, OperationPlan, RiskClass};
use workbench_domain::repository::RepositorySnapshot;

use crate::action_tests::{ExpectedRemoteRef, StoredSessionState};
use crate::executor::{execute_plan, ExecuteOutcome};
use crate::ports::{
    CleanupItemRecord, Clock, GitClient, GithubClient, IdGenerator, OperationStore,
    TestSessionStore,
};
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

    github.auth_status()?;
    git.fetch(&root, &expected.identity.remote)?;
    let remote_tracking_ref = format!(
        "refs/remotes/{}/{}",
        expected.identity.remote, expected.identity.ref_name
    );
    let actual = git.rev_parse(&root, &remote_tracking_ref)?;
    if actual.as_deref() != Some(expected.commit_sha.as_str()) {
        return Err(AppError::CleanupRefMoved {
            ref_name: expected.identity.ref_name.clone(),
            expected: expected.commit_sha.clone(),
            actual: actual.unwrap_or_else(|| "<missing>".into()),
        });
    }

    let mut snapshot = git.snapshot(&root)?;
    snapshot.selected_remote = Some(expected.identity.remote.clone());
    let plan = cleanup_plan(&item, &expected);
    let outcome = execute_plan(git, store, clock, ids, &project.id, &snapshot, &plan)?;
    let now = clock.now_rfc3339();
    store.complete_cleanup_item(&item.id, &now)?;
    Ok(outcome)
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
            "The ref will be deleted only if its commit still matches the recorded SHA.".into(),
        ],
        commands: vec![GitCommand::DeleteRemoteRef {
            remote: expected.identity.remote.clone(),
            ref_name: expected.identity.ref_name.clone(),
        }],
        preconditions: vec![format!(
            "`{}/{}` still resolves to `{}`.",
            expected.identity.remote, expected.identity.ref_name, expected.commit_sha
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
