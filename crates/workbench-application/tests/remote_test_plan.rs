mod support;

use std::fs;

use support::RemoteTestHarness;
use workbench_application::use_cases::action_discovery::discover_action_tests;
use workbench_application::use_cases::remote_test::plan_remote_test;
use workbench_application::AppError;
use workbench_domain::operations::plan::GitCommand;

#[test]
fn clean_repository_plans_generated_only_ephemeral_push() {
    let harness = RemoteTestHarness::new();
    let plan = plan_remote_test(
        &harness.git,
        &harness.store,
        &harness.policy,
        &harness.ids,
        harness.repo.path(),
        "smoke-composite",
        None,
    )
    .unwrap();

    assert_eq!(
        plan.workflow_path,
        format!(
            ".github/workflows/github-workbench-test-{}.yml",
            plan.session_id
        )
    );
    assert_eq!(
        plan.cleanup_identity.ref_name,
        format!("github-workbench/test/{}", plan.session_id)
    );
    assert!(matches!(
        plan.git_plan.commands.as_slice(),
        [
            GitCommand::CreateBranch { .. },
            GitCommand::CommitPaths { paths, .. },
            GitCommand::PushRef {
                set_upstream: false,
                ..
            }
        ] if paths == &[plan.workflow_path.clone()]
    ));
    assert!(plan.workflow_yaml.contains("ubuntu-latest"));
    assert!(!harness.repo.path().join(&plan.workflow_path).exists());
    assert!(harness.store.operations.lock().unwrap().is_empty());
    assert!(harness.store.sessions.lock().unwrap().is_empty());
    assert!(harness.github.calls().is_empty());
}

#[test]
fn dirty_repository_is_rejected_before_files_or_store_change() {
    let harness = RemoteTestHarness::new();
    harness.git.snapshot.borrow_mut().dirty_paths = vec!["src/lib.rs".into()];

    let error = plan_remote_test(
        &harness.git,
        &harness.store,
        &harness.policy,
        &harness.ids,
        harness.repo.path(),
        "smoke-composite",
        None,
    )
    .unwrap_err();

    assert!(matches!(error, AppError::DirtyWorkingTree { .. }));
    assert!(harness.store.sessions.lock().unwrap().is_empty());
}

#[test]
fn discovery_skips_build_directories_and_warns_for_unsupported_actions() {
    let harness = RemoteTestHarness::new();
    fs::create_dir_all(harness.repo.path().join("tools")).unwrap();
    fs::write(
        harness.repo.path().join("tools/action.yaml"),
        "name: JS action\nruns:\n  using: node20\n",
    )
    .unwrap();
    fs::create_dir_all(harness.repo.path().join("target/ignored")).unwrap();
    fs::write(
        harness.repo.path().join("target/ignored/action.yml"),
        "name: ignored\nruns:\n  using: composite\n",
    )
    .unwrap();

    let catalog = discover_action_tests(harness.repo.path()).unwrap();

    assert_eq!(catalog.actions.len(), 2);
    let unsupported = catalog
        .actions
        .iter()
        .find(|action| action.definition.name == "JS action")
        .unwrap();
    assert!(!unsupported.supported);
    assert!(unsupported.warning.as_deref().unwrap().contains("node20"));
    assert_eq!(catalog.tests.len(), 1);
    assert_eq!(catalog.tests[0].name, "smoke-composite");
}

#[test]
fn existing_push_workflow_adds_a_planning_warning() {
    let harness = RemoteTestHarness::new();
    fs::create_dir_all(harness.repo.path().join(".github/workflows")).unwrap();
    fs::write(
        harness.repo.path().join(".github/workflows/ci.yml"),
        "on:\n  push:\n    branches: [main]\n",
    )
    .unwrap();

    let plan = harness.plan();

    assert!(plan
        .git_plan
        .rationale
        .iter()
        .any(|rationale| rationale.contains("push")));
}
