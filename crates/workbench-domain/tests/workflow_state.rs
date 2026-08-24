use workbench_domain::workflow::state::{can_transition, transition, WorkflowState};

#[test]
fn happy_path_allows_unstarted_to_branch_created() {
    assert!(can_transition(
        WorkflowState::Unstarted,
        WorkflowState::BranchCreated
    ));
}

#[test]
fn rejects_skip_to_merged() {
    assert!(!can_transition(
        WorkflowState::Unstarted,
        WorkflowState::Merged
    ));
}

#[test]
fn transition_returns_new_state() {
    let next = transition(WorkflowState::Pushed, WorkflowState::PullRequestDraft).unwrap();
    assert_eq!(next, WorkflowState::PullRequestDraft);
}
