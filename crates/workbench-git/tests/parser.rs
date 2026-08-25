use workbench_git::parser::{parse_ahead_behind, parse_porcelain_z, parse_remotes_verbose};

#[test]
fn porcelain_z_lists_modified_and_untracked() {
    let raw = " M file.txt\0?? hello world.txt\0";
    let paths = parse_porcelain_z(raw);
    assert_eq!(paths, vec!["file.txt", "hello world.txt"]);
}

#[test]
fn remotes_verbose_dedupes_fetch_and_push() {
    let raw = "origin\tgit@github.com:acme/widgets.git (fetch)\norigin\tgit@github.com:acme/widgets.git (push)\ngithub\thttps://github.com/acme/widgets.git (fetch)\n";
    let remotes = parse_remotes_verbose(raw);
    assert_eq!(remotes.len(), 2);
    assert_eq!(remotes[0].name, "origin");
    assert_eq!(remotes[1].name, "github");
}

#[test]
fn ahead_behind_tab_separated() {
    assert_eq!(parse_ahead_behind("2\t3\n").unwrap(), (2, 3));
}
