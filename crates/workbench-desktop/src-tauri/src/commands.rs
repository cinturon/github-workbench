use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;
use workbench_application::action_tests::{RemoteTestResult, RemoteTestSessionPlan};
use workbench_application::clock::{SystemClock, ThreadSleeper};
use workbench_application::ids::UlidGenerator;
use workbench_application::policy_source::FilePolicySource;
use workbench_application::use_cases::action_discovery::{
    discover_action_tests, ActionTestCatalog,
};
use workbench_application::use_cases::remote_test::{
    execute_remote_test, plan_remote_test, watch_session,
};
use workbench_application::use_cases::test_sessions::get_session_result;
use workbench_application::AppError;
use workbench_git::{ProcessGitClient, StdProcessRunner};
use workbench_github::ProcessGithubClient;
use workbench_storage::SqliteStore;

use crate::DesktopState;

#[derive(Debug, Serialize)]
pub struct StartActionTestResponse {
    pub plan: RemoteTestSessionPlan,
    pub result: Option<RemoteTestResult>,
}

#[derive(Debug, Serialize)]
pub struct WatchActionTestResponse {
    pub pending: bool,
    pub result: Option<RemoteTestResult>,
}

pub fn list_action_tests_from_root(repo_root: &Path) -> Result<ActionTestCatalog, AppError> {
    discover_action_tests(repo_root)
}

#[tauri::command]
pub async fn list_action_tests(repo_root: String) -> Result<ActionTestCatalog, String> {
    run_blocking(move || list_action_tests_from_root(Path::new(&repo_root))).await
}

#[tauri::command]
pub async fn start_action_test(
    state: State<'_, DesktopState>,
    repo_root: String,
    test_name: String,
    confirmed: bool,
) -> Result<StartActionTestResponse, String> {
    let data_dir = state.data_dir().to_path_buf();
    run_blocking(move || {
        start_action_test_from_root(&data_dir, Path::new(&repo_root), &test_name, confirmed)
    })
    .await
}

#[tauri::command]
pub async fn watch_action_test(
    state: State<'_, DesktopState>,
    repo_root: String,
    session_id: String,
) -> Result<WatchActionTestResponse, String> {
    let data_dir = state.data_dir().to_path_buf();
    run_blocking(move || watch_action_test_from_root(&data_dir, Path::new(&repo_root), &session_id))
        .await
}

#[tauri::command]
pub async fn get_action_test_result(
    state: State<'_, DesktopState>,
    repo_root: String,
    session_id: String,
) -> Result<Option<RemoteTestResult>, String> {
    let data_dir = state.data_dir().to_path_buf();
    run_blocking(move || {
        let git = ProcessGitClient::new(StdProcessRunner);
        let store = open_store(&data_dir)?;
        get_session_result(&git, &store, Path::new(&repo_root), &session_id)
    })
    .await
}

fn start_action_test_from_root(
    data_dir: &Path,
    repo_root: &Path,
    test_name: &str,
    confirmed: bool,
) -> Result<StartActionTestResponse, AppError> {
    let git = ProcessGitClient::new(StdProcessRunner);
    let store = open_store(data_dir)?;
    let policy = FilePolicySource;
    let clock = SystemClock;
    let ids = UlidGenerator;
    let sleeper = ThreadSleeper;
    let plan = plan_remote_test(&git, &store, &policy, &ids, repo_root, test_name, None)?;

    if !confirmed {
        return Ok(StartActionTestResponse { plan, result: None });
    }

    let github = ProcessGithubClient::new(StdProcessRunner, plan.repo_root.clone());
    let result = match execute_remote_test(
        &git,
        &github,
        &store,
        &clock,
        &ids,
        &sleeper,
        &plan,
        &evidence_root(data_dir),
    ) {
        Ok(result) => result,
        Err(error) if matches!(error, AppError::AssertionFailed { .. }) => {
            get_session_result(&git, &store, repo_root, &plan.session_id)?.ok_or(error)?
        }
        Err(error) => return Err(error),
    };

    Ok(StartActionTestResponse {
        plan,
        result: Some(result),
    })
}

fn watch_action_test_from_root(
    data_dir: &Path,
    repo_root: &Path,
    session_id: &str,
) -> Result<WatchActionTestResponse, AppError> {
    let git = ProcessGitClient::new(StdProcessRunner);
    let store = open_store(data_dir)?;
    let github = ProcessGithubClient::new(StdProcessRunner, repo_root.to_path_buf());
    let clock = SystemClock;
    let ids = UlidGenerator;
    let sleeper = ThreadSleeper;

    match watch_session(
        &git,
        &github,
        &store,
        &clock,
        &ids,
        &sleeper,
        repo_root,
        session_id,
        &evidence_root(data_dir),
        false,
    ) {
        Ok(result) => Ok(WatchActionTestResponse {
            pending: false,
            result: Some(result),
        }),
        Err(AppError::RemotePending { .. }) => Ok(WatchActionTestResponse {
            pending: true,
            result: None,
        }),
        Err(error) if matches!(error, AppError::AssertionFailed { .. }) => {
            let result = get_session_result(&git, &store, repo_root, session_id)?.ok_or(error)?;
            Ok(WatchActionTestResponse {
                pending: false,
                result: Some(result),
            })
        }
        Err(error) => Err(error),
    }
}

fn open_store(data_dir: &Path) -> Result<SqliteStore, AppError> {
    SqliteStore::open(&data_dir.join("workbench.db"))
}

fn evidence_root(data_dir: &Path) -> PathBuf {
    data_dir.join("evidence")
}

async fn run_blocking<T, F>(operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| format!("desktop task failed: {error}"))?
        .map_err(|error| error.user_report())
}
