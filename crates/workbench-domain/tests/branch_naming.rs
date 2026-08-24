use workbench_domain::workflow::naming::{branch_name, normalize_slug};

#[test]
fn slug_from_issue_title() {
    assert_eq!(
        normalize_slug("Add resumable uploads").unwrap(),
        "add-resumable-uploads"
    );
}

#[test]
fn feature_branch_for_issue_42() {
    let name = branch_name("feature/{issue}-{slug}", 42, "Add resumable uploads").unwrap();
    assert_eq!(name, "feature/42-add-resumable-uploads");
}

#[test]
fn empty_title_is_invalid() {
    assert!(normalize_slug("@@@").is_err());
}

#[test]
fn rejects_double_dot_in_slug_source_after_normalize_path() {
    // normalize strips unsafe sequences; result must not contain ".."
    let slug = normalize_slug("foo..bar").unwrap();
    assert!(!slug.contains(".."));
}

#[test]
fn rejects_git_forbidden_chars_in_pattern() {
    assert!(branch_name("feature/{issue}:{slug}", 1, "x").is_err());
}
