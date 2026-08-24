use std::path::{Path, PathBuf};

use workbench_application::ports::{CommandOutput, CommandSpec, GitClient, ProcessRunner};
use workbench_application::redact::bound_output;
use workbench_application::AppError;
use workbench_domain::operations::plan::GitCommand;
use workbench_domain::repository::{BranchState, Remote, RepositorySnapshot};

use crate::argv::{command_argv, command_argvs};
use crate::env::sanitized_env;
use crate::parser::{parse_ahead_behind, parse_porcelain_z, parse_remotes_verbose};

pub struct ProcessGitClient<R> {
    runner: R,
    git_program: String,
    extra_env: Vec<(String, String)>,
}

impl<R> ProcessGitClient<R> {
    pub fn new(runner: R) -> Self {
        Self {
            runner,
            git_program: std::env::var("GWW_GIT_PROGRAM").unwrap_or_else(|_| "git".to_string()),
            extra_env: Vec::new(),
        }
    }

    pub fn with_extra_env(mut self, extra_env: Vec<(String, String)>) -> Self {
        self.extra_env = extra_env;
        self
    }
}

impl<R: ProcessRunner> ProcessGitClient<R> {
    fn run(&self, cwd: &Path, args: Vec<String>) -> Result<CommandOutput, AppError> {
        self.runner.run(&CommandSpec {
            program: self.git_program.clone(),
            args,
            cwd: cwd.to_path_buf(),
            env: sanitized_env(&self.extra_env),
        })
    }

    fn run_checked(&self, cwd: &Path, args: Vec<String>) -> Result<CommandOutput, AppError> {
        let output = self.run(cwd, args.clone())?;
        if output.exit_code == 0 {
            Ok(output)
        } else {
            Err(self.failed(args, output))
        }
    }

    fn failed(&self, args: Vec<String>, output: CommandOutput) -> AppError {
        AppError::GitFailed {
            program: self.git_program.clone(),
            args_summary: args.join(" "),
            status: output.exit_code,
            stderr_redacted: bound_output(&output.stderr),
        }
    }

    fn upstream(&self, repo_root: &Path) -> Result<Option<String>, AppError> {
        let output = self.run(
            repo_root,
            strings(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]),
        )?;
        if output.exit_code == 0 {
            Ok(nonempty_trimmed(&output.stdout))
        } else {
            Ok(None)
        }
    }

    fn current_branch(&self, repo_root: &Path) -> Result<String, AppError> {
        let output =
            self.run_checked(repo_root, strings(&["rev-parse", "--abbrev-ref", "HEAD"]))?;
        Ok(output.stdout.trim().to_string())
    }

    fn head_oid(&self, repo_root: &Path) -> Result<Option<String>, AppError> {
        let output = self.run_checked(repo_root, strings(&["rev-parse", "HEAD"]))?;
        Ok(nonempty_trimmed(&output.stdout))
    }

    fn dirty_paths(&self, repo_root: &Path) -> Result<Vec<String>, AppError> {
        let output = self.run_checked(repo_root, strings(&["status", "--porcelain=v1", "-z"]))?;
        Ok(parse_porcelain_z(&output.stdout))
    }

    fn resolve_oid(&self, repo_root: &Path, candidate: &str) -> Result<Option<String>, AppError> {
        let output = self.run(
            repo_root,
            vec![
                "rev-parse".into(),
                "--verify".into(),
                "--quiet".into(),
                "--end-of-options".into(),
                format!("{candidate}^{{commit}}"),
            ],
        )?;
        if output.exit_code == 0 {
            Ok(nonempty_trimmed(&output.stdout))
        } else {
            Ok(None)
        }
    }
}

impl<R: ProcessRunner> GitClient for ProcessGitClient<R> {
    fn resolve_toplevel(&self, path: &Path) -> Result<PathBuf, AppError> {
        let args = strings(&["rev-parse", "--show-toplevel"]);
        let output = self.run(path, args.clone())?;
        if output.exit_code == 0 {
            return Ok(PathBuf::from(output.stdout.trim()));
        }

        if output
            .stderr
            .to_ascii_lowercase()
            .contains("not a git repository")
        {
            Err(AppError::NotAGitRepository {
                path: path.display().to_string(),
            })
        } else {
            Err(self.failed(args, output))
        }
    }

    fn snapshot(&self, repo_root: &Path) -> Result<RepositorySnapshot, AppError> {
        let head_oid = self.head_oid(repo_root)?;
        let symbolic_ref = self.run(repo_root, strings(&["symbolic-ref", "-q", "HEAD"]))?;
        let detached_head = symbolic_ref.exit_code != 0;
        let branch = if detached_head {
            None
        } else {
            Some(self.current_branch(repo_root)?)
        };

        Ok(RepositorySnapshot {
            root: repo_root.display().to_string(),
            branch,
            detached_head,
            head_oid,
            dirty_paths: self.dirty_paths(repo_root)?,
            remotes: self.list_remotes(repo_root)?,
            selected_remote: None,
            upstream: self.upstream(repo_root)?,
        })
    }

    fn branch_state(
        &self,
        repo_root: &Path,
        comparison_ref: &str,
    ) -> Result<BranchState, AppError> {
        let upstream = self.upstream(repo_root)?;
        let right_ref = upstream.as_deref().unwrap_or(comparison_ref);
        let output = self.run_checked(
            repo_root,
            vec![
                "rev-list".into(),
                "--left-right".into(),
                "--count".into(),
                format!("HEAD...{right_ref}"),
            ],
        )?;
        let (ahead, behind) = parse_ahead_behind(&output.stdout)?;

        Ok(BranchState {
            name: self.current_branch(repo_root)?,
            head_oid: self.head_oid(repo_root)?,
            upstream,
            base_branch: Some(comparison_ref.to_string()),
            ahead,
            behind,
            dirty_paths: self.dirty_paths(repo_root)?,
            is_protected: false,
        })
    }

    fn list_remotes(&self, repo_root: &Path) -> Result<Vec<Remote>, AppError> {
        let output = self.run_checked(repo_root, strings(&["remote", "-v"]))?;
        Ok(parse_remotes_verbose(&output.stdout))
    }

    fn fetch(&self, repo_root: &Path, remote: &str) -> Result<CommandOutput, AppError> {
        let command = GitCommand::Fetch {
            remote: remote.to_string(),
        };
        self.run_checked(repo_root, command_argv(&command))
    }

    fn create_branch(
        &self,
        repo_root: &Path,
        name: &str,
        start_point: &str,
    ) -> Result<CommandOutput, AppError> {
        let existing = self.run(
            repo_root,
            vec![
                "show-ref".into(),
                "--verify".into(),
                "--quiet".into(),
                format!("refs/heads/{name}"),
            ],
        )?;
        if existing.exit_code == 0 {
            // `checkout` uses a legacy parser that treats `--end-of-options` as a pathspec.
            return self.run_checked(
                repo_root,
                vec!["switch".into(), "--end-of-options".into(), name.into()],
            );
        }

        let mut resolved_oid = self.resolve_oid(repo_root, start_point)?;
        if resolved_oid.is_none() {
            for remote in self.list_remotes(repo_root)? {
                let candidate = format!("{}/{start_point}", remote.name);
                if let Some(oid) = self.resolve_oid(repo_root, &candidate)? {
                    resolved_oid = Some(oid);
                    break;
                }
            }
        }
        let resolved_oid = resolved_oid.ok_or_else(|| AppError::GitFailed {
            program: self.git_program.clone(),
            args_summary: format!(
                "rev-parse --verify --quiet --end-of-options {start_point}^{{commit}}"
            ),
            status: 1,
            stderr_redacted: "start point did not resolve to a commit".into(),
        })?;

        let command = GitCommand::CreateBranch {
            name: name.to_string(),
            start_point: resolved_oid,
        };
        self.run_checked(repo_root, command_argv(&command))
    }

    fn push_ref(
        &self,
        repo_root: &Path,
        remote: &str,
        local_ref: &str,
        remote_ref: &str,
        set_upstream: bool,
    ) -> Result<CommandOutput, AppError> {
        let command = GitCommand::PushRef {
            remote: remote.to_string(),
            local_ref: local_ref.to_string(),
            remote_ref: remote_ref.to_string(),
            set_upstream,
        };
        self.run_checked(repo_root, command_argv(&command))
    }

    fn commit_paths(
        &self,
        repo_root: &Path,
        message: &str,
        paths: &[String],
    ) -> Result<CommandOutput, AppError> {
        if paths.is_empty() {
            return Err(AppError::Usage {
                message: "commit_paths requires at least one path".into(),
            });
        }

        let command = GitCommand::CommitPaths {
            message: message.to_string(),
            paths: paths.to_vec(),
        };
        let mut combined = CommandOutput {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        };
        for args in command_argvs(&command) {
            let output = self.run_checked(repo_root, args)?;
            combined.stdout.push_str(&output.stdout);
            combined.stderr.push_str(&output.stderr);
        }
        Ok(combined)
    }

    fn delete_remote_ref(
        &self,
        repo_root: &Path,
        remote: &str,
        ref_name: &str,
    ) -> Result<CommandOutput, AppError> {
        let command = GitCommand::DeleteRemoteRef {
            remote: remote.to_string(),
            ref_name: ref_name.to_string(),
        };
        self.run_checked(
            repo_root,
            command_argvs(&command).into_iter().next().unwrap(),
        )
    }

    fn rev_parse(&self, repo_root: &Path, reference: &str) -> Result<Option<String>, AppError> {
        self.resolve_oid(repo_root, reference)
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn nonempty_trimmed(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}
