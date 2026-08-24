use std::fs;

#[test]
fn phase_three_documentation_names_safety_and_live_boundaries() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    let architecture = fs::read_to_string(root.join("docs/architecture.md")).unwrap();
    let live = fs::read_to_string(root.join("docs/superpowers/manual/phase3-live-e2e.md")).unwrap();

    for command in [
        "gww action discover",
        "gww action test",
        "gww runs watch",
        "gww cleanup run",
    ] {
        assert!(readme.contains(command));
    }

    assert!(architecture.contains("workbench-application does not depend on adapter crates"));
    assert!(architecture.contains("Never force push"));
    assert!(live.contains("disposable repository"));
    assert!(live.contains("GWW_LIVE_E2E=1"));
    assert!(live.contains("not required CI"));
}
