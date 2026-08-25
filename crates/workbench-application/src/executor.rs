use std::path::Path;

use workbench_domain::operations::plan::{GitCommand, OperationPlan, RiskClass, StepStatus};
use workbench_domain::repository::RepositorySnapshot;

use crate::error::AppError;
use crate::ports::{Clock, CommandOutput, GitClient, IdGenerator, OperationStore};
use crate::redact::{bound_output, redact};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteOutcome {
    pub operation_id: String,
    pub status: String,
    pub changed: Vec<String>,
}

pub fn execute_plan<G, S, C, I>(
    git: &G,
    store: &S,
    clock: &C,
    ids: &I,
    project_id: &str,
    snapshot: &RepositorySnapshot,
    plan: &OperationPlan,
) -> Result<ExecuteOutcome, AppError>
where
    G: GitClient,
    S: OperationStore,
    C: Clock,
    I: IdGenerator,
{
    if plan.risk == RiskClass::High {
        return Err(AppError::Usage {
            message: "high-risk operations are not allowed in Phase 2".into(),
        });
    }
    if plan.commands.is_empty() {
        return Ok(ExecuteOutcome {
            operation_id: String::new(),
            status: "noop".into(),
            changed: vec![],
        });
    }

    let operation_id = ids.next().to_string();
    let started_at = clock.now_rfc3339();
    store.create_operation(
        project_id,
        &operation_id,
        &plan.kind,
        "running",
        plan,
        snapshot,
        &started_at,
    )?;

    let root = Path::new(&snapshot.root);
    let mut changed = Vec::new();
    let mut mutating_step_started = false;

    for (index, command) in plan.commands.iter().enumerate() {
        let sequence = i32::try_from(index + 1).map_err(|_| AppError::Storage {
            detail: "plan has too many steps to journal".into(),
        })?;
        let step_id = ids.next().to_string();
        let detail_json = serde_json::to_string(command).map_err(|error| AppError::Storage {
            detail: format!("could not serialize operation step: {error}"),
        })?;
        let now = clock.now_rfc3339();
        store.append_step(
            &operation_id,
            &step_id,
            sequence,
            command.step_kind(),
            StepStatus::Pending,
            Some(&detail_json),
            &now,
        )?;
        store.update_step(&step_id, StepStatus::Running, None, None)?;

        if matches!(
            command,
            GitCommand::CreateBranch { .. } | GitCommand::PushRef { .. }
        ) {
            mutating_step_started = true;
        }

        match execute_command(git, root, command) {
            Ok(output) => {
                let completed_at = clock.now_rfc3339();
                let output = combined_output(&output);
                store.update_step(
                    &step_id,
                    StepStatus::Succeeded,
                    Some(&output),
                    Some(&completed_at),
                )?;
                changed.push(command_description(command));
            }
            Err(error) => {
                let completed_at = clock.now_rfc3339();
                let failure_output = failure_output(&error);
                store.update_step(
                    &step_id,
                    StepStatus::Failed,
                    Some(&failure_output),
                    Some(&completed_at),
                )?;

                let mut unchanged = Vec::new();
                for (remaining_index, remaining) in plan.commands.iter().enumerate().skip(index + 1)
                {
                    let remaining_sequence =
                        i32::try_from(remaining_index + 1).map_err(|_| AppError::Storage {
                            detail: "plan has too many steps to journal".into(),
                        })?;
                    let skipped_id = ids.next().to_string();
                    let detail_json =
                        serde_json::to_string(remaining).map_err(|serialize_error| {
                            AppError::Storage {
                                detail: format!(
                                    "could not serialize operation step: {serialize_error}"
                                ),
                            }
                        })?;
                    let skipped_at = clock.now_rfc3339();
                    store.append_step(
                        &operation_id,
                        &skipped_id,
                        remaining_sequence,
                        remaining.step_kind(),
                        StepStatus::Skipped,
                        Some(&detail_json),
                        &skipped_at,
                    )?;
                    unchanged.push(command_description(remaining));
                }

                store.update_operation(&operation_id, "failed", Some(&completed_at))?;
                return Err(AppError::OperationFailed {
                    message: error.to_string(),
                    changed,
                    unchanged,
                    retry_safe: !mutating_step_started,
                    remediation:
                        "Inspect the journal and repository state, fix the Git error, then retry."
                            .into(),
                });
            }
        }
    }

    let completed_at = clock.now_rfc3339();
    store.update_operation(&operation_id, "succeeded", Some(&completed_at))?;
    Ok(ExecuteOutcome {
        operation_id,
        status: "succeeded".into(),
        changed,
    })
}

fn execute_command<G: GitClient>(
    git: &G,
    root: &Path,
    command: &GitCommand,
) -> Result<CommandOutput, AppError> {
    match command {
        GitCommand::Fetch { remote } => git.fetch(root, remote),
        GitCommand::CreateBranch { name, start_point } => {
            git.create_branch(root, name, start_point)
        }
        GitCommand::PushRef {
            remote,
            local_ref,
            remote_ref,
            set_upstream,
        } => git.push_ref(root, remote, local_ref, remote_ref, *set_upstream),
    }
}

fn combined_output(output: &CommandOutput) -> String {
    bound_output(&format!("{}{}", output.stdout, output.stderr))
}

fn failure_output(error: &AppError) -> String {
    match error {
        AppError::GitFailed {
            stderr_redacted, ..
        } => bound_output(stderr_redacted),
        other => bound_output(&redact(&other.to_string())),
    }
}

fn command_description(command: &GitCommand) -> String {
    match command {
        GitCommand::Fetch { remote } => format!("Fetched remote `{remote}`."),
        GitCommand::CreateBranch { name, start_point } => {
            format!("Created branch `{name}` from `{start_point}`.")
        }
        GitCommand::PushRef {
            remote,
            local_ref,
            remote_ref,
            ..
        } => format!("Pushed `{local_ref}` to `{remote}/{remote_ref}`."),
    }
}
