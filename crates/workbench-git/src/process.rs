use std::io::ErrorKind;
use std::process::{Command, Stdio};

use workbench_application::ports::{CommandOutput, CommandSpec, ProcessRunner};
use workbench_application::AppError;

pub struct StdProcessRunner;

impl ProcessRunner for StdProcessRunner {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput, AppError> {
        let cwd_metadata = std::fs::metadata(&spec.cwd).map_err(|err| AppError::Io {
            path: spec.cwd.display().to_string(),
            detail: err.to_string(),
        })?;
        if !cwd_metadata.is_dir() {
            return Err(AppError::Io {
                path: spec.cwd.display().to_string(),
                detail: "working directory is not a directory".into(),
            });
        }

        let output = Command::new(&spec.program)
            .args(&spec.args)
            .current_dir(&spec.cwd)
            .env_clear()
            .envs(spec.env.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|err| {
                if err.kind() == ErrorKind::NotFound && spec.cwd.is_dir() {
                    AppError::GitUnavailable {
                        detail: err.to_string(),
                    }
                } else {
                    AppError::Io {
                        path: spec.cwd.display().to_string(),
                        detail: err.to_string(),
                    }
                }
            })?;

        Ok(CommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use workbench_application::ports::{CommandSpec, ProcessRunner};
    use workbench_application::AppError;

    use crate::env::sanitized_env;
    use crate::process::StdProcessRunner;

    #[test]
    fn runs_git_version_in_temp_dir() {
        let temp = tempfile::tempdir().expect("temp dir");
        let spec = CommandSpec {
            program: "git".into(),
            args: vec!["--version".into()],
            cwd: temp.path().to_path_buf(),
            env: sanitized_env(&[]),
        };

        let runner = StdProcessRunner;
        match runner.run(&spec) {
            Ok(output) => {
                assert_eq!(output.exit_code, 0);
                assert!(
                    output.stdout.contains("git version"),
                    "stdout: {}",
                    output.stdout
                );
            }
            Err(AppError::GitUnavailable { .. }) => {
                panic!("git is installed in this environment");
            }
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn missing_git_program_returns_git_unavailable() {
        let temp = tempfile::tempdir().expect("temp dir");
        let spec = CommandSpec {
            program: "gww-nonexistent-git-program".into(),
            args: vec!["--version".into()],
            cwd: PathBuf::from(temp.path()),
            env: sanitized_env(&[]),
        };

        let runner = StdProcessRunner;
        let err = runner.run(&spec).expect_err("expected GitUnavailable");
        assert!(matches!(err, AppError::GitUnavailable { .. }));
    }

    #[test]
    fn missing_working_directory_returns_io_error() {
        let temp = tempfile::tempdir().expect("temp dir");
        let missing = temp.path().join("does-not-exist");
        let spec = CommandSpec {
            program: "git".into(),
            args: vec!["--version".into()],
            cwd: missing.clone(),
            env: sanitized_env(&[]),
        };

        let runner = StdProcessRunner;
        let err = runner.run(&spec).expect_err("expected missing cwd error");
        assert!(matches!(
            err,
            AppError::Io { ref path, .. } if path == &missing.display().to_string()
        ));
    }
}
