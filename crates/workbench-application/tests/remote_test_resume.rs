mod support;

use std::time::Duration;

use support::RemoteTestHarness;
use workbench_application::action_tests::TestSessionStatus;
use workbench_application::ports::{TestSessionStore, WorkflowRunDetail};
use workbench_application::use_cases::remote_test::watch_session;
use workbench_application::use_cases::test_sessions::{get_session_result, list_sessions};
use workbench_application::AppError;
use workbench_domain::operations::plan::GitCommand;

#[test]
fn watch_resumes_a_stored_push_without_repush() {
    let harness = RemoteTestHarness::stored_pending_then_success();

    let result = watch_session(
        &harness.git,
        &harness.github,
        &harness.store,
        &harness.clock,
        &harness.ids,
        &harness.sleeper,
        harness.repo.path(),
        "01JABC",
        harness.evidence.path(),
        true,
    )
    .unwrap();

    assert!(result.passed);
    assert!(harness.git.executed.borrow().iter().all(|command| {
        !matches!(
            command,
            GitCommand::CreateBranch { .. }
                | GitCommand::CommitPaths { .. }
                | GitCommand::PushRef { .. }
        )
    }));
    assert_eq!(
        get_session_result(&harness.git, &harness.store, harness.repo.path(), "01JABC").unwrap(),
        Some(result)
    );
    assert_eq!(
        list_sessions(&harness.git, &harness.store, harness.repo.path())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn non_waiting_watch_persists_queued_state_and_returns_pending() {
    let harness = RemoteTestHarness::stored_pending_then_success();
    let mut details = harness.github.run_detail_responses.lock().unwrap();
    details.clear();
    details.push_back(Ok(WorkflowRunDetail {
        id: 42,
        head_sha: "abc123".into(),
        path: ".github/workflows/github-workbench-test-01JABC.yml".into(),
        status: "queued".into(),
        conclusion: None,
        html_url: "https://github.com/acme/widgets/actions/runs/42".into(),
    }));
    drop(details);

    let error = watch_session(
        &harness.git,
        &harness.github,
        &harness.store,
        &harness.clock,
        &harness.ids,
        &harness.sleeper,
        harness.repo.path(),
        "01JABC",
        harness.evidence.path(),
        false,
    )
    .unwrap_err();

    assert_eq!(
        error,
        AppError::RemotePending {
            session_id: "01JABC".into()
        }
    );
    let stored = harness
        .store
        .get_test_session("project-1", "01JABC")
        .unwrap()
        .unwrap();
    assert_eq!(stored.run_id, Some(42));
    assert_eq!(stored.status, TestSessionStatus::Queued);
    assert!(harness.sleeper.durations.lock().unwrap().is_empty());
}

#[test]
fn waiting_watch_sleeps_between_non_terminal_polls() {
    let harness = RemoteTestHarness::stored_pending_then_success();
    harness
        .github
        .run_detail_responses
        .lock()
        .unwrap()
        .push_front(Ok(WorkflowRunDetail {
            id: 42,
            head_sha: "abc123".into(),
            path: ".github/workflows/github-workbench-test-01JABC.yml".into(),
            status: "in_progress".into(),
            conclusion: None,
            html_url: "https://github.com/acme/widgets/actions/runs/42".into(),
        }));

    watch_session(
        &harness.git,
        &harness.github,
        &harness.store,
        &harness.clock,
        &harness.ids,
        &harness.sleeper,
        harness.repo.path(),
        "01JABC",
        harness.evidence.path(),
        true,
    )
    .unwrap();

    assert_eq!(
        harness.sleeper.durations.lock().unwrap().as_slice(),
        &[Duration::from_secs(3)]
    );
}
