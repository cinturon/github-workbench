mod support;

use support::RemoteTestHarness;
use workbench_application::action_tests::{StoredSessionState, TestSessionStatus};
use workbench_application::ports::TestSessionStore;
use workbench_application::use_cases::remote_test::execute_remote_test;
use workbench_domain::operations::plan::GitCommand;

#[test]
fn execution_authenticates_pushes_downloads_and_persists_result() {
    let harness = RemoteTestHarness::completed_success();
    let plan = harness.plan();

    let result = execute_remote_test(
        &harness.git,
        &harness.github,
        &harness.store,
        &harness.clock,
        &harness.ids,
        &harness.sleeper,
        &plan,
        harness.evidence.path(),
    )
    .unwrap();

    assert!(result.passed);
    assert_eq!(result.run_id, 42);
    assert!(harness.github.calls().first().unwrap().starts_with("auth"));
    assert_eq!(
        harness
            .git
            .executed
            .borrow()
            .iter()
            .filter(|command| { matches!(command, GitCommand::PushRef { .. }) })
            .count(),
        1
    );
    assert_eq!(
        harness
            .store
            .list_cleanup_items(&plan.project_id)
            .unwrap()
            .len(),
        1
    );

    let stored = harness
        .store
        .get_test_session(&plan.project_id, &plan.session_id)
        .unwrap()
        .unwrap();
    let state: StoredSessionState = serde_json::from_str(&stored.result_json).unwrap();
    assert_eq!(stored.status, TestSessionStatus::Passed);
    assert_eq!(stored.run_id, Some(42));
    assert_eq!(state.result.as_ref(), Some(&result));
    assert!(result.manifest_path.as_ref().unwrap().exists());
    assert!(result.logs_path.exists());
}

#[test]
fn head_change_is_rejected_before_workflow_or_git_mutation() {
    let harness = RemoteTestHarness::completed_success();
    let plan = harness.plan();
    harness.git.snapshot.borrow_mut().head_oid = Some("new-head".into());

    let error = execute_remote_test(
        &harness.git,
        &harness.github,
        &harness.store,
        &harness.clock,
        &harness.ids,
        &harness.sleeper,
        &plan,
        harness.evidence.path(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        workbench_application::AppError::OperationFailed {
            retry_safe: true,
            ..
        }
    ));
    assert!(!harness.repo.path().join(&plan.workflow_path).exists());
    assert!(harness.git.executed.borrow().is_empty());
    assert!(harness.store.sessions.lock().unwrap().is_empty());
}
