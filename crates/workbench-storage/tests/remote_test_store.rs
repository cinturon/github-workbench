use tempfile::tempdir;
use workbench_application::action_tests::TestSessionStatus;
use workbench_application::ports::{
    NewCleanupItem, NewProject, NewTestSession, OperationStore, TestSessionStore,
};
use workbench_storage::SqliteStore;

#[test]
fn migration_two_round_trips_sessions_and_cleanup() {
    let temp = tempdir().unwrap();
    let store = SqliteStore::open(&temp.path().join("workbench.db")).unwrap();

    store
        .upsert_project(NewProject {
            id: "project-1",
            local_path: "/repo",
            github_host: Some("github.com"),
            owner: Some("acme"),
            repo: Some("widgets"),
            remote_name: Some("origin"),
            now: "2026-08-24T00:00:00Z",
        })
        .unwrap();

    store
        .create_test_session(NewTestSession {
            id: "row-1",
            project_id: "project-1",
            session_id: "01JABC",
            commit_sha: "abc123",
            remote_ref: "github-workbench/test/01JABC",
            workflow_name: "github-workbench-test-01JABC.yml",
            status: TestSessionStatus::Pushed,
            result_json: r#"{"plan":{},"pushed_sha":"abc123","result":null}"#,
            now: "2026-08-24T00:00:00Z",
        })
        .unwrap();

    let session = store
        .get_test_session("project-1", "01JABC")
        .unwrap()
        .unwrap();
    assert_eq!(session.commit_sha, "abc123");
    assert_eq!(session.status, TestSessionStatus::Pushed);

    store
        .enqueue_cleanup(NewCleanupItem {
            id: "cleanup-1",
            project_id: "project-1",
            resource_kind: "remote-git-ref",
            resource_id: "origin/github-workbench/test/01JABC",
            expected_identity: r#"{"commit_sha":"abc123"}"#,
            due_at: "2026-08-24T00:00:00Z",
            now: "2026-08-24T00:00:00Z",
        })
        .unwrap();

    assert_eq!(store.list_cleanup_items("project-1").unwrap().len(), 1);
}

#[test]
fn migrations_are_idempotent() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("workbench.db");

    SqliteStore::open(&path).unwrap();
    SqliteStore::open(&path).unwrap();
}
