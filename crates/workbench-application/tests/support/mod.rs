#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use ulid::Ulid;
use workbench_application::action_tests::{
    CleanupIdentity, ExpectedRemoteRef, RemoteTestSessionPlan, StoredSessionState,
    TestSessionStatus,
};
use workbench_application::fakes::{
    FakeClock, FakeGit, FakeGithub, FakeIds, FakePolicy, FakeSleeper, FakeStore,
};
use workbench_application::ports::{
    NewCleanupItem, NewTestSession, ProjectRecord, TestSessionStore, WorkflowRunDetail,
    WorkflowRunSummary,
};
use workbench_application::use_cases::remote_test::plan_remote_test;
use workbench_domain::repository::{BranchState, Remote, RepositorySnapshot};

const NOW: &str = "2026-08-24T12:00:00Z";
const ACTION_YAML: &str = r#"
name: Upload report
description: Uploads a generated report
inputs:
  report-path:
    description: Report path
    required: false
runs:
  using: composite
  steps:
    - shell: bash
      run: echo "Upload completed"
"#;
const TEST_YAML: &str = r#"
schema-version: 1
name: smoke-composite
description: Exercises the root composite action.
action:
  path: .
runner:
  os:
    - ubuntu-latest
  timeout-minutes: 10
permissions:
  contents: read
inputs: {}
environment: {}
expect:
  conclusion: success
  logs:
    - contains: "Upload completed"
    - not-contains: "secret="
"#;
const LOG_FIXTURE: &str = "Run action under test\nUpload completed\n";

pub struct RemoteTestHarness {
    pub repo: tempfile::TempDir,
    pub evidence: tempfile::TempDir,
    pub git: FakeGit,
    pub github: FakeGithub,
    pub store: FakeStore,
    pub policy: FakePolicy,
    pub clock: FakeClock,
    pub ids: FakeIds,
    pub sleeper: FakeSleeper,
}

impl RemoteTestHarness {
    pub fn new() -> Self {
        let repo = tempfile::tempdir().unwrap();
        let evidence = tempfile::tempdir().unwrap();
        fs::write(repo.path().join("action.yml"), ACTION_YAML).unwrap();
        fs::create_dir_all(repo.path().join(".github-workbench/tests")).unwrap();
        fs::write(
            repo.path()
                .join(".github-workbench/tests/smoke-composite.yml"),
            TEST_YAML,
        )
        .unwrap();

        let root = repo.path().to_string_lossy().into_owned();
        let snapshot = RepositorySnapshot {
            root: root.clone(),
            branch: Some("main".into()),
            detached_head: false,
            head_oid: Some("abc123".into()),
            dirty_paths: vec![],
            remotes: vec![Remote {
                name: "github".into(),
                url: "git@github.com:acme/widgets.git".into(),
            }],
            selected_remote: None,
            upstream: None,
        };
        let git = FakeGit {
            toplevel: PathBuf::from(&root),
            snapshot: RefCell::new(snapshot),
            branch: RefCell::new(BranchState {
                name: "main".into(),
                head_oid: Some("abc123".into()),
                upstream: None,
                base_branch: Some("main".into()),
                ahead: 0,
                behind: 0,
                dirty_paths: vec![],
                is_protected: true,
            }),
            executed: RefCell::new(vec![]),
            fail_kind: RefCell::new(None),
            refs: RefCell::new(BTreeMap::from([("HEAD".into(), "abc123".into())])),
        };
        let store = FakeStore::new();
        store.projects.lock().unwrap().push(ProjectRecord {
            id: "project-1".into(),
            local_path: root,
            github_host: Some("github.com".into()),
            owner: Some("acme".into()),
            repo: Some("widgets".into()),
            remote_name: Some("github".into()),
            created_at: NOW.into(),
            updated_at: NOW.into(),
        });

        Self {
            repo,
            evidence,
            git,
            github: FakeGithub::new(),
            store,
            policy: FakePolicy { yaml: None },
            clock: FakeClock(NOW.into()),
            ids: FakeIds::new(),
            sleeper: FakeSleeper::default(),
        }
    }

    pub fn completed_success() -> Self {
        let harness = Self::new();
        let session_id = Ulid::from_parts(1, 1).to_string();
        let workflow_file_name = format!("github-workbench-test-{session_id}.yml");
        harness
            .github
            .run_list_responses
            .lock()
            .unwrap()
            .push_back(Ok(vec![run_summary(
                42,
                "fake-commit-1",
                &workflow_file_name,
                "queued",
                None,
            )]));
        harness
            .github
            .run_detail_responses
            .lock()
            .unwrap()
            .push_back(Ok(run_detail(
                42,
                "fake-commit-1",
                &workflow_file_name,
                "completed",
                Some("success"),
            )));
        *harness.github.artifact_fixture.lock().unwrap() =
            manifest_fixture(&session_id).into_bytes();
        *harness.github.logs_fixture.lock().unwrap() = LOG_FIXTURE.as_bytes().to_vec();
        harness
    }

    pub fn stored_pending_then_success() -> Self {
        let harness = Self::new();
        let plan = fixed_plan(&harness);
        insert_pushed_session(&harness, &plan, "abc123");
        harness
            .github
            .run_list_responses
            .lock()
            .unwrap()
            .push_back(Ok(vec![run_summary(
                42,
                "abc123",
                &plan.workflow_file_name,
                "queued",
                None,
            )]));
        harness
            .github
            .run_detail_responses
            .lock()
            .unwrap()
            .push_back(Ok(run_detail(
                42,
                "abc123",
                &plan.workflow_file_name,
                "completed",
                Some("success"),
            )));
        *harness.github.artifact_fixture.lock().unwrap() = manifest_fixture("01JABC").into_bytes();
        *harness.github.logs_fixture.lock().unwrap() = LOG_FIXTURE.as_bytes().to_vec();
        harness
    }

    pub fn cleanup_with_remote_sha(actual_sha: &str) -> Self {
        let harness = Self::new();
        let plan = fixed_plan(&harness);
        insert_pushed_session(&harness, &plan, "abc123");
        let expected = ExpectedRemoteRef {
            identity: plan.cleanup_identity.clone(),
            commit_sha: "abc123".into(),
        };
        let expected_identity = serde_json::to_string(&expected).unwrap();
        harness
            .store
            .enqueue_cleanup(NewCleanupItem {
                id: "cleanup-1",
                project_id: &plan.project_id,
                resource_kind: "remote-git-ref",
                resource_id: "github/github-workbench/test/01JABC",
                expected_identity: &expected_identity,
                due_at: NOW,
                now: NOW,
            })
            .unwrap();
        harness.git.refs.borrow_mut().insert(
            "refs/remotes/github/github-workbench/test/01JABC".into(),
            actual_sha.into(),
        );
        harness
    }

    pub fn plan(&self) -> RemoteTestSessionPlan {
        plan_remote_test(
            &self.git,
            &self.store,
            &self.policy,
            &self.ids,
            self.repo.path(),
            "smoke-composite",
            None,
        )
        .unwrap()
    }
}

fn fixed_plan(harness: &RemoteTestHarness) -> RemoteTestSessionPlan {
    let mut plan = harness.plan();
    let branch = "github-workbench/test/01JABC".to_string();
    plan.session_id = "01JABC".into();
    plan.workflow_file_name = "github-workbench-test-01JABC.yml".into();
    plan.workflow_path = ".github/workflows/github-workbench-test-01JABC.yml".into();
    plan.cleanup_identity = CleanupIdentity {
        remote: "github".into(),
        ref_name: branch.clone(),
        session_id: "01JABC".into(),
    };
    for command in &mut plan.git_plan.commands {
        match command {
            workbench_domain::operations::plan::GitCommand::CreateBranch { name, .. } => {
                *name = branch.clone();
            }
            workbench_domain::operations::plan::GitCommand::CommitPaths { paths, .. } => {
                *paths = vec![plan.workflow_path.clone()];
            }
            workbench_domain::operations::plan::GitCommand::PushRef {
                local_ref,
                remote_ref,
                ..
            } => {
                *local_ref = branch.clone();
                *remote_ref = branch.clone();
            }
            _ => {}
        }
    }
    plan
}

fn insert_pushed_session(
    harness: &RemoteTestHarness,
    plan: &RemoteTestSessionPlan,
    pushed_sha: &str,
) {
    let state = StoredSessionState {
        plan: plan.clone(),
        pushed_sha: Some(pushed_sha.into()),
        result: None,
    };
    let state_json = serde_json::to_string(&state).unwrap();
    harness
        .store
        .create_test_session(NewTestSession {
            id: "session-row-1",
            project_id: &plan.project_id,
            session_id: &plan.session_id,
            commit_sha: pushed_sha,
            remote_ref: &plan.cleanup_identity.ref_name,
            workflow_name: &plan.workflow_file_name,
            status: TestSessionStatus::Pushed,
            result_json: &state_json,
            now: NOW,
        })
        .unwrap();
}

fn run_summary(
    id: u64,
    head_sha: &str,
    workflow_file_name: &str,
    status: &str,
    conclusion: Option<&str>,
) -> WorkflowRunSummary {
    WorkflowRunSummary {
        id,
        head_sha: head_sha.into(),
        path: format!(".github/workflows/{workflow_file_name}"),
        status: status.into(),
        conclusion: conclusion.map(str::to_string),
        html_url: format!("https://github.com/acme/widgets/actions/runs/{id}"),
    }
}

fn run_detail(
    id: u64,
    head_sha: &str,
    workflow_file_name: &str,
    status: &str,
    conclusion: Option<&str>,
) -> WorkflowRunDetail {
    WorkflowRunDetail {
        id,
        head_sha: head_sha.into(),
        path: format!(".github/workflows/{workflow_file_name}"),
        status: status.into(),
        conclusion: conclusion.map(str::to_string),
        html_url: format!("https://github.com/acme/widgets/actions/runs/{id}"),
    }
}

fn manifest_fixture(session_id: &str) -> String {
    format!(
        r#"{{"schema_version":1,"session_id":"{session_id}","case":"smoke-composite","runner":"ubuntu-latest","action_outcome":"success","outputs":{{}}}}"#
    )
}
