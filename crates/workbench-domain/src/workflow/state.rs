use crate::WorkbenchError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowState {
    Unstarted,
    BranchCreated,
    ChangesPresent,
    Committed,
    Pushed,
    PullRequestDraft,
    ValidationPending,
    ReviewPending,
    ReadyToMerge,
    Merged,
    CleanupPending,
    Complete,
}

pub fn can_transition(from: WorkflowState, to: WorkflowState) -> bool {
    matches!(
        (from, to),
        (WorkflowState::Unstarted, WorkflowState::BranchCreated)
            | (WorkflowState::BranchCreated, WorkflowState::ChangesPresent)
            | (WorkflowState::ChangesPresent, WorkflowState::Committed)
            | (WorkflowState::Committed, WorkflowState::Pushed)
            | (WorkflowState::Pushed, WorkflowState::PullRequestDraft)
            | (WorkflowState::PullRequestDraft, WorkflowState::ValidationPending)
            | (WorkflowState::ValidationPending, WorkflowState::ReviewPending)
            | (WorkflowState::ValidationPending, WorkflowState::ValidationPending)
            | (WorkflowState::ReviewPending, WorkflowState::ReadyToMerge)
            | (WorkflowState::ReadyToMerge, WorkflowState::Merged)
            | (WorkflowState::Merged, WorkflowState::CleanupPending)
            | (WorkflowState::CleanupPending, WorkflowState::Complete)
    )
}

pub fn transition(from: WorkflowState, to: WorkflowState) -> Result<WorkflowState, WorkbenchError> {
    if can_transition(from, to) {
        Ok(to)
    } else {
        Err(WorkbenchError::IllegalTransition { from, to })
    }
}
