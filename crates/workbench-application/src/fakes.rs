use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ulid::Ulid;

use crate::error::AppError;
use crate::ports::{
    Clock, CommandOutput, GitClient, IdGenerator, NewProject, OperationRecord, OperationStore,
    PolicySource, ProjectRecord, StepRecord,
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
}

pub struct FakeStore {
    pub projects: Mutex<Vec<ProjectRecord>>,
    pub operations: Mutex<Vec<OperationRecord>>,
}

impl FakeStore {
    pub fn new() -> Self {
        Self {
            projects: Mutex::new(Vec::new()),
            operations: Mutex::new(Vec::new()),
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
