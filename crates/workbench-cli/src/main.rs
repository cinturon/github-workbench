mod args;
mod confirm;
mod data_dir;
mod render;

use std::path::Path;

use args::{Cli, Commands, IssueCommands, OpsCommands};
use clap::Parser;
use workbench_application::clock::SystemClock;
use workbench_application::ids::UlidGenerator;
use workbench_application::policy_source::FilePolicySource;
use workbench_application::ports::{GitClient, OperationStore};
use workbench_application::use_cases::open::open_repository;
use workbench_application::use_cases::ops::list_project_operations;
use workbench_application::use_cases::push::{execute_push, plan_push_changes};
use workbench_application::use_cases::start_issue::{execute_start_issue, plan_start_issue};
use workbench_application::use_cases::status::repository_status;
use workbench_application::AppError;
use workbench_domain::operations::plan::GitCommand;
use workbench_git::{ProcessGitClient, StdProcessRunner};
use workbench_storage::SqliteStore;

use crate::confirm::confirm;
use crate::data_dir::resolve_data_dir;
use crate::render::{render_operations, render_plan, render_status, render_status_json};

enum RunOutcome {
    Success,
    Aborted,
}

fn main() {
    match run(Cli::parse()) {
        Ok(RunOutcome::Success) => {}
        Ok(RunOutcome::Aborted) => {
            println!("Aborted.");
            std::process::exit(1);
        }
        Err(error) => {
            eprint!("{}", error.user_report());
            std::process::exit(error.exit_code());
        }
    }
}

fn run(cli: Cli) -> Result<RunOutcome, AppError> {
    let command = cli.command.ok_or_else(|| AppError::Usage {
        message: "a command is required".into(),
    })?;
    let data_dir = resolve_data_dir(|key| std::env::var(key).ok());
    std::fs::create_dir_all(&data_dir).map_err(|error| AppError::Io {
        path: data_dir.display().to_string(),
        detail: error.to_string(),
    })?;

    let git = ProcessGitClient::new(StdProcessRunner);
    let store = SqliteStore::open(&data_dir.join("workbench.db"))?;
    let policy = FilePolicySource;
    let clock = SystemClock;
    let ids = UlidGenerator;

    match command {
        Commands::Open { path, remote } => {
            let outcome = open_repository(
                &git,
                &store,
                &policy,
                &clock,
                &ids,
                &path,
                remote.as_deref(),
            )?;
            println!("Repository root: {}", outcome.snapshot.root);
            println!("Remotes:");
            for remote in &outcome.snapshot.remotes {
                println!("  {}: {}", remote.name, remote.url);
            }
            println!(
                "Selected remote: {}",
                outcome
                    .snapshot
                    .selected_remote
                    .as_deref()
                    .unwrap_or("(none)")
            );
            println!("Policy source: {}", outcome.policy_source);
            println!("Project id: {}", outcome.project.id);
        }
        Commands::Status { json } => {
            let cwd = current_dir()?;
            let root = git.resolve_toplevel(&cwd)?;
            let mapped_remote = store
                .get_project_by_path(&root)?
                .and_then(|project| project.remote_name);
            let outcome = repository_status(&git, &policy, &cwd, mapped_remote.as_deref(), None)?;
            if json {
                println!("{}", render_status_json(&outcome)?);
            } else {
                println!("{}", render_status(&outcome));
            }
        }
        Commands::Issue { command } => match command {
            IssueCommands::Start {
                number,
                title,
                yes,
                remote,
            } => {
                let cwd = current_dir()?;
                let (plan, _, _) = plan_start_issue(
                    &git,
                    &store,
                    &policy,
                    &cwd,
                    number,
                    &title,
                    remote.as_deref(),
                )?;
                let plan_text = render_plan(&plan);
                if !confirm(&plan_text, yes)? {
                    return Ok(RunOutcome::Aborted);
                }
                let branch_name = plan.commands.iter().find_map(|command| match command {
                    GitCommand::CreateBranch { name, .. } => Some(name.clone()),
                    _ => None,
                });
                let outcome = execute_start_issue(
                    &git,
                    &store,
                    &policy,
                    &clock,
                    &ids,
                    &cwd,
                    number,
                    &title,
                    remote.as_deref(),
                )?;
                println!("Operation id: {}", outcome.operation_id);
                if let Some(branch_name) = branch_name {
                    println!("Created branch: {branch_name}");
                }
            }
        },
        Commands::Push { plan, yes, remote } => {
            let cwd = current_dir()?;
            let (operation_plan, _) =
                plan_push_changes(&git, &store, &policy, &cwd, remote.as_deref())?;
            let plan_text = render_plan(&operation_plan);
            if plan {
                println!("{plan_text}");
                return Ok(RunOutcome::Success);
            }
            if !confirm(&plan_text, yes)? {
                return Ok(RunOutcome::Aborted);
            }
            let outcome =
                execute_push(&git, &store, &policy, &clock, &ids, &cwd, remote.as_deref())?;
            if outcome.status == "noop" {
                println!("Nothing to push.");
            } else {
                println!("Operation id: {}", outcome.operation_id);
                for change in outcome.changed {
                    println!("{change}");
                }
            }
        }
        Commands::Ops { command } => match command {
            OpsCommands::List => {
                let operations = list_project_operations(&git, &store, &current_dir()?, None)?;
                println!("{}", render_operations(&operations));
            }
        },
    }

    Ok(RunOutcome::Success)
}

fn current_dir() -> Result<std::path::PathBuf, AppError> {
    std::env::current_dir().map_err(|error| AppError::Io {
        path: Path::new(".").display().to_string(),
        detail: error.to_string(),
    })
}
