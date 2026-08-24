mod support;

use support::RemoteTestHarness;
use workbench_application::ports::TestSessionStore;
use workbench_application::use_cases::cleanup::{execute_cleanup, list_cleanup, plan_cleanup};
use workbench_application::AppError;
use workbench_domain::operations::plan::{GitCommand, RiskClass};

#[test]
fn cleanup_ref_move_is_refused_without_delete() {
    let harness = RemoteTestHarness::cleanup_with_remote_sha("abc123");
    *harness.github.delete_ref_actual_sha.lock().unwrap() = Some("moved-sha".into());

    let error = execute_cleanup(
        &harness.git,
        &harness.github,
        &harness.store,
        &harness.clock,
        &harness.ids,
        harness.repo.path(),
        "cleanup-1",
    )
    .unwrap_err();

    assert!(matches!(
        error,
        AppError::CleanupRefMoved { actual, .. } if actual == "moved-sha"
    ));
    assert_eq!(
        harness.github.calls(),
        vec![
            "auth",
            "delete-ref acme/widgets github-workbench/test/01JABC abc123"
        ]
    );
    assert!(harness.git.executed.borrow().is_empty());
    assert_eq!(
        harness
            .store
            .get_cleanup_item("project-1", "cleanup-1")
            .unwrap()
            .unwrap()
            .status,
        "pending"
    );
}

#[test]
fn matching_cleanup_deletes_through_github_journals_and_completes() {
    let harness = RemoteTestHarness::cleanup_with_remote_sha("abc123");

    let outcome = execute_cleanup(
        &harness.git,
        &harness.github,
        &harness.store,
        &harness.clock,
        &harness.ids,
        harness.repo.path(),
        "cleanup-1",
    )
    .unwrap();

    assert_eq!(outcome.status, "succeeded");
    assert_eq!(
        harness.github.calls(),
        vec![
            "auth",
            "delete-ref acme/widgets github-workbench/test/01JABC abc123"
        ]
    );
    assert!(harness
        .git
        .executed
        .borrow()
        .iter()
        .all(|command| !matches!(command, GitCommand::DeleteRemoteRef { .. })));
    let operations = harness.store.operations.lock().unwrap();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].steps.len(), 1);
    assert_eq!(
        operations[0].steps[0].kind,
        "delete-github-ref-if-sha-matches"
    );
    assert_eq!(
        harness
            .store
            .get_cleanup_item("project-1", "cleanup-1")
            .unwrap()
            .unwrap()
            .status,
        "completed"
    );
}

#[test]
fn malformed_cleanup_identity_is_rejected_before_auth_or_git() {
    let harness = RemoteTestHarness::cleanup_with_remote_sha("abc123");
    harness.store.cleanup.lock().unwrap()[0].resource_id = "github/wrong-ref".into();

    let error = execute_cleanup(
        &harness.git,
        &harness.github,
        &harness.store,
        &harness.clock,
        &harness.ids,
        harness.repo.path(),
        "cleanup-1",
    )
    .unwrap_err();

    assert!(matches!(error, AppError::CleanupIdentityMismatch { .. }));
    assert!(harness.github.calls().is_empty());
    assert!(harness.git.executed.borrow().is_empty());
}

#[test]
fn cleanup_listing_and_plan_are_project_scoped_and_delete_only() {
    let harness = RemoteTestHarness::cleanup_with_remote_sha("abc123");

    let items = list_cleanup(&harness.git, &harness.store, harness.repo.path()).unwrap();
    let (plan, snapshot, item) = plan_cleanup(
        &harness.git,
        &harness.store,
        harness.repo.path(),
        "cleanup-1",
    )
    .unwrap();

    assert_eq!(items, vec![item]);
    assert_eq!(plan.risk, RiskClass::Medium);
    assert!(plan.commands.is_empty());
    assert!(plan
        .rationale
        .iter()
        .any(|line| line.contains("GitHub API")));
    assert_eq!(snapshot.root, harness.repo.path().to_string_lossy());
    assert!(harness.git.executed.borrow().is_empty());
}
