use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use ulid::Ulid;

use crate::error::AppError;
use crate::ports::{
    CleanupItemRecord, Clock, CommandOutput, GitClient, GithubClient, IdGenerator, NewCleanupItem,
    NewProject, NewTestSession, OperationRecord, OperationStore, PolicySource, ProjectRecord,
    Sleeper, StepRecord, TestSessionRecord, TestSessionStore, TestSessionUpdate, WorkflowRunDetail,
    WorkflowRunSummary,
};
use workbench_domain::operations::plan::{GitCommand, OperationPlan, StepStatus};
use workbench_domain::repository::{BranchState, Remote, RepositorySnapshot};

pub struct FakeClock(pub String);
impl Clock for FakeClock {
    fn now_rfc3339(&self) -> String {
        self.0.clone()
    }
}

pub struct FakeIds {
    pub next: Mutex<u64>,
}
impl FakeIds {
    pub fn new() -> Self {
        Self {
            next: Mutex::new(1),
        }
    }
}
impl Default for FakeIds {
    fn default() -> Self {
        Self::new()
    }
}
impl IdGenerator for FakeIds {
    fn next(&self) -> Ulid {
        let mut n = self.next.lock().unwrap();
        let value = *n;
        *n += 1;
        Ulid::from_parts(value, value as u128)
    }
}

pub struct FakePolicy {
    pub yaml: Option<String>,
}
impl PolicySource for FakePolicy {
    fn read_yaml(&self, _repo_root: &Path) -> Result<Option<String>, AppError> {
        Ok(self.yaml.clone())
    }
}

pub struct FakeGit {
    pub toplevel: PathBuf,
    pub snapshot: RefCell<RepositorySnapshot>,
    pub branch: RefCell<BranchState>,
    pub executed: RefCell<Vec<GitCommand>>,
    pub fail_kind: RefCell<Option<String>>,
    pub refs: RefCell<BTreeMap<String, String>>,
    pub rev_parse_responses: RefCell<VecDeque<Option<String>>>,
}

impl FakeGit {
    fn maybe_fail(&self, kind: &str) -> Result<CommandOutput, AppError> {
        if self.fail_kind.borrow().as_deref() == Some(kind) {
            return Err(AppError::GitFailed {
                program: "git".into(),
                args_summary: kind.into(),
                status: 1,
                stderr_redacted: "injected failure".into(),
            });
        }
        Ok(CommandOutput {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}

impl GitClient for FakeGit {
    fn resolve_toplevel(&self, _path: &Path) -> Result<PathBuf, AppError> {
        Ok(self.toplevel.clone())
    }

    fn snapshot(&self, _repo_root: &Path) -> Result<RepositorySnapshot, AppError> {
        Ok(self.snapshot.borrow().clone())
    }

    fn branch_state(
        &self,
        _repo_root: &Path,
        _comparison_ref: &str,
    ) -> Result<BranchState, AppError> {
        Ok(self.branch.borrow().clone())
    }

    fn list_remotes(&self, _repo_root: &Path) -> Result<Vec<Remote>, AppError> {
        Ok(self.snapshot.borrow().remotes.clone())
    }

    fn fetch(&self, _repo_root: &Path, remote: &str) -> Result<CommandOutput, AppError> {
        self.executed.borrow_mut().push(GitCommand::Fetch {
            remote: remote.into(),
        });
        self.maybe_fail("fetch")
    }

    fn create_branch(
        &self,
        _repo_root: &Path,
        name: &str,
        start_point: &str,
    ) -> Result<CommandOutput, AppError> {
        self.executed.borrow_mut().push(GitCommand::CreateBranch {
            name: name.into(),
            start_point: start_point.into(),
        });
        let out = self.maybe_fail("create-branch")?;
        self.snapshot.borrow_mut().branch = Some(name.into());
        self.branch.borrow_mut().name = name.into();
        Ok(out)
    }

    fn push_ref(
        &self,
        _repo_root: &Path,
        remote: &str,
        local_ref: &str,
        remote_ref: &str,
        set_upstream: bool,
    ) -> Result<CommandOutput, AppError> {
        self.executed.borrow_mut().push(GitCommand::PushRef {
            remote: remote.into(),
            local_ref: local_ref.into(),
            remote_ref: remote_ref.into(),
            set_upstream,
        });
        self.maybe_fail("push-ref")
    }

    fn commit_paths(
        &self,
        _repo_root: &Path,
        message: &str,
        paths: &[String],
    ) -> Result<CommandOutput, AppError> {
        self.executed.borrow_mut().push(GitCommand::CommitPaths {
            message: message.into(),
            paths: paths.to_vec(),
        });
        let out = self.maybe_fail("commit-paths")?;
        let commit_number = self
            .executed
            .borrow()
            .iter()
            .filter(|command| matches!(command, GitCommand::CommitPaths { .. }))
            .count();
        let sha = format!("fake-commit-{commit_number}");
        self.refs.borrow_mut().insert("HEAD".into(), sha.clone());
        self.snapshot.borrow_mut().head_oid = Some(sha.clone());
        self.branch.borrow_mut().head_oid = Some(sha);
        Ok(out)
    }

    fn delete_remote_ref(
        &self,
        _repo_root: &Path,
        remote: &str,
        ref_name: &str,
    ) -> Result<CommandOutput, AppError> {
        self.executed
            .borrow_mut()
            .push(GitCommand::DeleteRemoteRef {
                remote: remote.into(),
                ref_name: ref_name.into(),
            });
        let out = self.maybe_fail("delete-remote-ref")?;
        self.refs
            .borrow_mut()
            .remove(&format!("refs/remotes/{remote}/{ref_name}"));
        Ok(out)
    }

    fn rev_parse(&self, _repo_root: &Path, reference: &str) -> Result<Option<String>, AppError> {
        if let Some(response) = self.rev_parse_responses.borrow_mut().pop_front() {
            return Ok(response);
        }
        Ok(self.refs.borrow().get(reference).cloned())
    }
}

pub struct FakeGithub {
    pub auth_error: Mutex<Option<AppError>>,
    pub delete_ref_actual_sha: Mutex<Option<String>>,
    pub run_list_responses: Mutex<VecDeque<Result<Vec<WorkflowRunSummary>, AppError>>>,
    pub run_detail_responses: Mutex<VecDeque<Result<WorkflowRunDetail, AppError>>>,
    pub artifact_fixture: Mutex<Vec<u8>>,
    pub logs_fixture: Mutex<Vec<u8>>,
    calls: Mutex<Vec<String>>,
}

impl FakeGithub {
    pub fn new() -> Self {
        Self {
            auth_error: Mutex::new(None),
            delete_ref_actual_sha: Mutex::new(None),
            run_list_responses: Mutex::new(VecDeque::new()),
            run_detail_responses: Mutex::new(VecDeque::new()),
            artifact_fixture: Mutex::new(Vec::new()),
            logs_fixture: Mutex::new(Vec::new()),
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn record(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }
}

impl Default for FakeGithub {
    fn default() -> Self {
        Self::new()
    }
}

impl GithubClient for FakeGithub {
    fn auth_status(&self) -> Result<(), AppError> {
        self.record("auth".into());
        match self.auth_error.lock().unwrap().clone() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn delete_ref_if_sha_matches(
        &self,
        owner: &str,
        repo: &str,
        ref_name: &str,
        expected_sha: &str,
    ) -> Result<(), AppError> {
        self.record(format!(
            "delete-ref {owner}/{repo} {ref_name} {expected_sha}"
        ));
        let actual = self
            .delete_ref_actual_sha
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| expected_sha.into());
        if actual == expected_sha {
            Ok(())
        } else {
            Err(AppError::CleanupRefMoved {
                ref_name: ref_name.into(),
                expected: expected_sha.into(),
                actual,
            })
        }
    }

    fn list_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        head_sha: &str,
        workflow_file_name: &str,
    ) -> Result<Vec<WorkflowRunSummary>, AppError> {
        self.record(format!(
            "list-runs {owner}/{repo} {head_sha} {workflow_file_name}"
        ));
        self.run_list_responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    fn get_workflow_run(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<WorkflowRunDetail, AppError> {
        self.record(format!("get-run {owner}/{repo} {run_id}"));
        self.run_detail_responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(AppError::Storage {
                    detail: format!("FakeGithub has no queued detail for workflow run {run_id}"),
                })
            })
    }

    fn download_artifact_zip(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        artifact_name: &str,
        dest_dir: &Path,
    ) -> Result<PathBuf, AppError> {
        self.record(format!(
            "download-artifact {owner}/{repo} {run_id} {artifact_name}"
        ));
        fs::create_dir_all(dest_dir).map_err(|error| io_error(dest_dir, error))?;
        let path = dest_dir.join(format!("{artifact_name}.json"));
        fs::write(&path, self.artifact_fixture.lock().unwrap().as_slice())
            .map_err(|error| io_error(&path, error))?;
        Ok(dest_dir.to_path_buf())
    }

    fn download_run_logs(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        dest_path: &Path,
    ) -> Result<PathBuf, AppError> {
        self.record(format!("download-logs {owner}/{repo} {run_id}"));
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
        }
        fs::write(dest_path, self.logs_fixture.lock().unwrap().as_slice())
            .map_err(|error| io_error(dest_path, error))?;
        Ok(dest_path.to_path_buf())
    }
}

fn io_error(path: &Path, error: std::io::Error) -> AppError {
    AppError::Io {
        path: path.display().to_string(),
        detail: error.to_string(),
    }
}

#[derive(Default)]
pub struct FakeSleeper {
    pub durations: Mutex<Vec<Duration>>,
}

impl Sleeper for FakeSleeper {
    fn sleep(&self, duration: Duration) {
        self.durations.lock().unwrap().push(duration);
    }
}

pub struct FakeStore {
    pub projects: Mutex<Vec<ProjectRecord>>,
    pub operations: Mutex<Vec<OperationRecord>>,
    pub sessions: Mutex<Vec<TestSessionRecord>>,
    pub cleanup: Mutex<Vec<CleanupItemRecord>>,
}

impl FakeStore {
    pub fn new() -> Self {
        Self {
            projects: Mutex::new(Vec::new()),
            operations: Mutex::new(Vec::new()),
            sessions: Mutex::new(Vec::new()),
            cleanup: Mutex::new(Vec::new()),
        }
    }
}
impl Default for FakeStore {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationStore for FakeStore {
    fn upsert_project(&self, project: NewProject<'_>) -> Result<ProjectRecord, AppError> {
        let mut projects = self.projects.lock().unwrap();
        if let Some(existing) = projects
            .iter_mut()
            .find(|p| p.local_path == project.local_path)
        {
            existing.github_host = project.github_host.map(str::to_string);
            existing.owner = project.owner.map(str::to_string);
            existing.repo = project.repo.map(str::to_string);
            existing.remote_name = project.remote_name.map(str::to_string);
            existing.updated_at = project.now.to_string();
            return Ok(existing.clone());
        }
        let record = ProjectRecord {
            id: project.id.to_string(),
            local_path: project.local_path.to_string(),
            github_host: project.github_host.map(str::to_string),
            owner: project.owner.map(str::to_string),
            repo: project.repo.map(str::to_string),
            remote_name: project.remote_name.map(str::to_string),
            created_at: project.now.to_string(),
            updated_at: project.now.to_string(),
        };
        projects.push(record.clone());
        Ok(record)
    }

    fn get_project_by_path(&self, path: &Path) -> Result<Option<ProjectRecord>, AppError> {
        let key = path.to_string_lossy().into_owned();
        Ok(self
            .projects
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.local_path == key)
            .cloned())
    }

    fn create_operation(
        &self,
        project_id: &str,
        id: &str,
        kind: &str,
        status: &str,
        plan: &OperationPlan,
        snapshot: &RepositorySnapshot,
        started_at: &str,
    ) -> Result<OperationRecord, AppError> {
        let record = OperationRecord {
            id: id.into(),
            project_id: project_id.into(),
            kind: kind.into(),
            status: status.into(),
            plan_json: serde_json::to_string(plan).unwrap(),
            started_at: Some(started_at.into()),
            completed_at: None,
            snapshot_json: Some(serde_json::to_string(snapshot).unwrap()),
            steps: vec![],
        };
        self.operations.lock().unwrap().push(record.clone());
        Ok(record)
    }

    fn update_operation(
        &self,
        id: &str,
        status: &str,
        completed_at: Option<&str>,
    ) -> Result<(), AppError> {
        let mut ops = self.operations.lock().unwrap();
        let op = ops.iter_mut().find(|o| o.id == id).unwrap();
        op.status = status.into();
        op.completed_at = completed_at.map(str::to_string);
        Ok(())
    }

    fn append_step(
        &self,
        operation_id: &str,
        id: &str,
        sequence: i32,
        kind: &str,
        status: StepStatus,
        detail_json: Option<&str>,
        _now: &str,
    ) -> Result<StepRecord, AppError> {
        let step = StepRecord {
            id: id.into(),
            operation_id: operation_id.into(),
            sequence,
            kind: kind.into(),
            status,
            detail_json: detail_json.map(str::to_string),
            output_text: None,
        };
        let mut ops = self.operations.lock().unwrap();
        let op = ops.iter_mut().find(|o| o.id == operation_id).unwrap();
        op.steps.push(step.clone());
        Ok(step)
    }

    fn update_step(
        &self,
        id: &str,
        status: StepStatus,
        output_text: Option<&str>,
        _completed_at: Option<&str>,
    ) -> Result<(), AppError> {
        let mut ops = self.operations.lock().unwrap();
        for op in ops.iter_mut() {
            if let Some(step) = op.steps.iter_mut().find(|s| s.id == id) {
                step.status = status;
                step.output_text = output_text.map(str::to_string);
                return Ok(());
            }
        }
        Err(AppError::Storage {
            detail: format!("missing step {id}"),
        })
    }

    fn list_operations(
        &self,
        project_id: &str,
        limit: u32,
    ) -> Result<Vec<OperationRecord>, AppError> {
        let mut rows: Vec<_> = self
            .operations
            .lock()
            .unwrap()
            .iter()
            .filter(|o| o.project_id == project_id)
            .cloned()
            .collect();
        rows.reverse();
        rows.truncate(limit as usize);
        Ok(rows)
    }
}

impl TestSessionStore for FakeStore {
    fn create_test_session(
        &self,
        session: NewTestSession<'_>,
    ) -> Result<TestSessionRecord, AppError> {
        let record = TestSessionRecord {
            id: session.id.into(),
            project_id: session.project_id.into(),
            session_id: session.session_id.into(),
            commit_sha: session.commit_sha.into(),
            remote_ref: session.remote_ref.into(),
            workflow_name: session.workflow_name.into(),
            run_id: None,
            status: session.status,
            result_json: session.result_json.into(),
            evidence_dir: None,
            created_at: session.now.into(),
            updated_at: session.now.into(),
        };
        self.sessions.lock().unwrap().push(record.clone());
        Ok(record)
    }

    fn update_test_session(&self, update: TestSessionUpdate<'_>) -> Result<(), AppError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .iter_mut()
            .find(|session| {
                session.project_id == update.project_id && session.session_id == update.session_id
            })
            .ok_or_else(|| AppError::Storage {
                detail: format!(
                    "missing test session {}/{}",
                    update.project_id, update.session_id
                ),
            })?;
        session.run_id = update.run_id;
        session.status = update.status;
        session.result_json = update.result_json.into();
        session.evidence_dir = update.evidence_dir.map(str::to_string);
        session.updated_at = update.now.into();
        Ok(())
    }

    fn get_test_session(
        &self,
        project_id: &str,
        session_id: &str,
    ) -> Result<Option<TestSessionRecord>, AppError> {
        Ok(self
            .sessions
            .lock()
            .unwrap()
            .iter()
            .find(|session| session.project_id == project_id && session.session_id == session_id)
            .cloned())
    }

    fn list_test_sessions(
        &self,
        project_id: &str,
        limit: u32,
    ) -> Result<Vec<TestSessionRecord>, AppError> {
        let mut rows: Vec<_> = self
            .sessions
            .lock()
            .unwrap()
            .iter()
            .filter(|session| session.project_id == project_id)
            .cloned()
            .collect();
        rows.reverse();
        rows.truncate(limit as usize);
        Ok(rows)
    }

    fn enqueue_cleanup(&self, item: NewCleanupItem<'_>) -> Result<CleanupItemRecord, AppError> {
        let record = CleanupItemRecord {
            id: item.id.into(),
            project_id: item.project_id.into(),
            resource_kind: item.resource_kind.into(),
            resource_id: item.resource_id.into(),
            expected_identity: item.expected_identity.into(),
            due_at: item.due_at.into(),
            status: "pending".into(),
            created_at: item.now.into(),
            updated_at: item.now.into(),
        };
        self.cleanup.lock().unwrap().push(record.clone());
        Ok(record)
    }

    fn get_cleanup_item(
        &self,
        project_id: &str,
        item_id: &str,
    ) -> Result<Option<CleanupItemRecord>, AppError> {
        Ok(self
            .cleanup
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.project_id == project_id && item.id == item_id)
            .cloned())
    }

    fn list_cleanup_items(&self, project_id: &str) -> Result<Vec<CleanupItemRecord>, AppError> {
        Ok(self
            .cleanup
            .lock()
            .unwrap()
            .iter()
            .filter(|item| item.project_id == project_id)
            .cloned()
            .collect())
    }

    fn complete_cleanup_item(&self, item_id: &str, now: &str) -> Result<(), AppError> {
        let mut cleanup = self.cleanup.lock().unwrap();
        let item = cleanup
            .iter_mut()
            .find(|item| item.id == item_id)
            .ok_or_else(|| AppError::Storage {
                detail: format!("missing cleanup item {item_id}"),
            })?;
        item.status = "completed".into();
        item.updated_at = now.into();
        Ok(())
    }
}
