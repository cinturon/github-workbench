use serde::Deserialize;
use workbench_application::ports::{WorkflowRunDetail, WorkflowRunSummary};
use workbench_application::AppError;

#[derive(Deserialize)]
struct RunsResponse {
    workflow_runs: Vec<WorkflowRunSummary>,
}

pub fn parse_runs(
    json: &str,
    head_sha: &str,
    workflow_file_name: &str,
) -> Result<Vec<WorkflowRunSummary>, AppError> {
    let response: RunsResponse = serde_json::from_str(json).map_err(parse_error)?;
    Ok(response
        .workflow_runs
        .into_iter()
        .filter(|run| {
            run.head_sha == head_sha
                && run
                    .path
                    .rsplit('/')
                    .next()
                    .is_some_and(|name| name == workflow_file_name)
        })
        .collect())
}

pub fn parse_run(json: &str) -> Result<WorkflowRunDetail, AppError> {
    serde_json::from_str(json).map_err(parse_error)
}

fn parse_error(error: serde_json::Error) -> AppError {
    AppError::GithubFailed {
        program: "gh".into(),
        args_summary: "parse JSON response".into(),
        status: 0,
        stderr_redacted: error.to_string(),
    }
}
