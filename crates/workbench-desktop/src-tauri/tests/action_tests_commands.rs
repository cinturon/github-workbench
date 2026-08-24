use tempfile::tempdir;
use workbench_desktop::commands::list_action_tests_from_root;

#[test]
fn list_command_delegates_to_application_discovery() {
    let repo = tempdir().unwrap();
    std::fs::write(
        repo.path().join("action.yml"),
        "name: Smoke\nruns:\n  using: composite\n  steps: []\n",
    )
    .unwrap();
    std::fs::create_dir_all(repo.path().join(".github-workbench/tests")).unwrap();
    std::fs::write(
        repo.path().join(".github-workbench/tests/smoke.yml"),
        "schema-version: 1\nname: smoke\naction:\n  path: .\nrunner:\n  os: [ubuntu-latest]\nexpect:\n  conclusion: success\n",
    )
    .unwrap();

    let catalog = list_action_tests_from_root(repo.path()).unwrap();

    assert_eq!(catalog.actions.len(), 1);
    assert_eq!(catalog.tests.len(), 1);
}
