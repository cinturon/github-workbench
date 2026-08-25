use workbench_domain::repository::parse_github_remote;

#[test]
fn parses_ssh_scp_syntax() {
    let id = parse_github_remote("git@github.com:acme/widgets.git").unwrap();
    assert_eq!(id.host, "github.com");
    assert_eq!(id.owner, "acme");
    assert_eq!(id.name, "widgets");
}

#[test]
fn parses_https() {
    let id = parse_github_remote("https://github.com/acme/widgets.git").unwrap();
    assert_eq!(id.owner, "acme");
    assert_eq!(id.name, "widgets");
}

#[test]
fn parses_ssh_url() {
    let id = parse_github_remote("ssh://git@github.com/acme/widgets.git").unwrap();
    assert_eq!(id.owner, "acme");
    assert_eq!(id.name, "widgets");
}

#[test]
fn rejects_non_githubish_path() {
    assert!(parse_github_remote("/tmp/local-bare.git").is_none());
}
