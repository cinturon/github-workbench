use workbench_application::ports::{NewProject, OperationStore};
use workbench_domain::operations::plan::{OperationPlan, RiskClass, StepStatus};
use workbench_domain::repository::RepositorySnapshot;
use workbench_storage::SqliteStore;

#[test]
fn migrations_and_operation_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("workbench.db");
    let store = SqliteStore::open(&db).unwrap();
    // open again to prove migrations are idempotent
    let store = SqliteStore::open(&db).unwrap();

    let project = store
        .upsert_project(NewProject {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            local_path: "/tmp/repo",
            github_host: Some("github.com"),
            owner: Some("acme"),
            repo: Some("widgets"),
            remote_name: Some("github"),
            now: "2026-08-24T00:00:00Z",
        })
        .unwrap();
    assert_eq!(project.remote_name.as_deref(), Some("github"));

    let plan = OperationPlan {
        id: ulid::Ulid::nil(),
        kind: "push".into(),
        risk: RiskClass::Low,
        summary: "test".into(),
        rationale: vec![],
        commands: vec![],
        preconditions: vec![],
        findings: vec![],
    };
    let snapshot = RepositorySnapshot {
        root: "/tmp/repo".into(),
        branch: Some("feature/x".into()),
        detached_head: false,
        head_oid: Some("abc".into()),
        dirty_paths: vec![],
        remotes: vec![],
        selected_remote: Some("github".into()),
        upstream: None,
    };
    let op = store
        .create_operation(
            &project.id,
            "01ARZ3NDEKTSV4RRFFQ69G5FAW",
            "push",
            "running",
            &plan,
            &snapshot,
            "2026-08-24T00:00:01Z",
        )
        .unwrap();
    let step = store
        .append_step(
            &op.id,
            "01ARZ3NDEKTSV4RRFFQ69G5FAX",
            1,
            "push-ref",
            StepStatus::Pending,
            None,
            "2026-08-24T00:00:01Z",
        )
        .unwrap();
    store
        .update_step(
            &step.id,
            StepStatus::Succeeded,
            Some("pushed"),
            Some("2026-08-24T00:00:02Z"),
        )
        .unwrap();
    store
        .update_operation(&op.id, "succeeded", Some("2026-08-24T00:00:02Z"))
        .unwrap();

    let listed = store.list_operations(&project.id, 10).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, "succeeded");
    assert_eq!(listed[0].steps.len(), 1);
    assert_eq!(listed[0].steps[0].status, StepStatus::Succeeded);
    assert_eq!(listed[0].steps[0].output_text.as_deref(), Some("pushed"));
}
