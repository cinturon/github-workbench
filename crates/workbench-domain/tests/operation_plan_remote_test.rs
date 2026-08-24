use workbench_domain::operations::plan::GitCommand;

#[test]
fn remote_test_commands_have_stable_step_kinds() {
    assert_eq!(
        GitCommand::CommitPaths {
            message: "chore: add test workflow".into(),
            paths: vec![".github/workflows/test.yml".into()],
        }
        .step_kind(),
        "commit-paths"
    );
    assert_eq!(
        GitCommand::DeleteRemoteRef {
            remote: "origin".into(),
            ref_name: "github-workbench/test/01JABC".into(),
        }
        .step_kind(),
        "delete-remote-ref"
    );
}
