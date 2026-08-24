use std::path::{Path, PathBuf};

use workbench_application::AppError;

pub mod commands;

pub struct DesktopState {
    data_dir: PathBuf,
}

impl DesktopState {
    pub fn new() -> Result<Self, AppError> {
        let data_dir = resolve_data_dir(|key| std::env::var(key).ok());
        std::fs::create_dir_all(&data_dir).map_err(|error| AppError::Io {
            path: data_dir.display().to_string(),
            detail: error.to_string(),
        })?;
        Ok(Self { data_dir })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(DesktopState::new().expect("failed to initialize desktop state"))
        .invoke_handler(tauri::generate_handler![
            commands::list_action_tests,
            commands::start_action_test,
            commands::watch_action_test,
            commands::get_action_test_result,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run GitHub Workflow Workbench");
}

fn resolve_data_dir(vars: impl Fn(&str) -> Option<String>) -> PathBuf {
    if let Some(dir) = vars("GWW_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(xdg) = vars("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("github-workbench");
    }
    if let Some(local) = vars("LOCALAPPDATA") {
        return PathBuf::from(local).join("github-workbench");
    }
    let home = vars("HOME").unwrap_or_else(|| ".".into());
    PathBuf::from(home).join(".local/share/github-workbench")
}
