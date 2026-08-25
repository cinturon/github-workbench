use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use workbench_application::ports::GitClient;
use workbench_git::argv::assert_no_force;
use workbench_git::{sanitized_env, ProcessGitClient, StdProcessRunner};

struct Harness {
    _tmp: TempDir,
    home: PathBuf,
    remote: PathBuf,
    work: PathBuf,
    extra_env: Vec<(String, String)>,
}

fn write_gitconfig(home: &Path) {
    fs::write(
        home.join("gitconfig"),
        "[user]\n    name = Workbench Test\n    email = workbench@example.test\n[init]\n    defaultBranch = main\n[commit]\n    gpgsign = false\n",
    )
    .unwrap();
}

fn git(home: &Path, cwd: &Path, args: &[&str]) {
    let config = home.join("gitconfig");
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(sanitized_env(&[
            ("HOME".into(), home.display().to_string()),
            ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
            ("GIT_CONFIG_GLOBAL".into(), config.display().to_string()),
        ]))
        .status()
        .expect("git must be installed");
    assert!(status.success(), "git {args:?} failed");
}

fn harness() -> Harness {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_gitconfig(&home);
    let remote = tmp.path().join("remote.git");
    let work = tmp.path().join("work");
    git(
        &home,
        tmp.path(),
        &["init", "--bare", "-b", "main", remote.to_str().unwrap()],
    );
    git(
        &home,
        tmp.path(),
        &["clone", remote.to_str().unwrap(), work.to_str().unwrap()],
    );
    fs::write(work.join("README.md"), "hi\n").unwrap();
    git(&home, &work, &["add", "README.md"]);
    git(&home, &work, &["commit", "-m", "init"]);
    git(&home, &work, &["push", "-u", "origin", "main"]);
    let extra_env = vec![
        ("HOME".into(), home.display().to_string()),
        ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
        (
            "GIT_CONFIG_GLOBAL".into(),
            home.join("gitconfig").display().to_string(),
        ),
    ];
    Harness {
        _tmp: tmp,
        home,
        remote,
        work,
        extra_env,
    }
}

#[test]
fn create_branch_push_status_and_dirty_space_path() {
    let h = harness();
    let client = ProcessGitClient::new(StdProcessRunner).with_extra_env(h.extra_env.clone());
    let root = client.resolve_toplevel(&h.work).unwrap();
    assert_eq!(root, h.work.canonicalize().unwrap());

    let snap = client.snapshot(&root).unwrap();
    assert!(!snap.detached_head);
    assert_eq!(snap.branch.as_deref(), Some("main"));
    assert!(snap.dirty_paths.is_empty());
    assert_eq!(snap.remotes.len(), 1);
    let remote_name = snap.remotes[0].name.clone();

    client
        .create_branch(&root, "feature/42-add-resumable-uploads", "main")
        .unwrap();
    let snap = client.snapshot(&root).unwrap();
    assert_eq!(
        snap.branch.as_deref(),
        Some("feature/42-add-resumable-uploads")
    );
    client
        .create_branch(&root, "feature/42-add-resumable-uploads", "ignored")
        .unwrap();

    fs::write(root.join("note.txt"), "n\n").unwrap();
    git(&h.home, &root, &["add", "note.txt"]);
    git(&h.home, &root, &["commit", "-m", "note"]);
    let before = client.branch_state(&root, "main").unwrap();
    assert!(before.ahead >= 1);

    client
        .push_ref(
            &root,
            &remote_name,
            "feature/42-add-resumable-uploads",
            "feature/42-add-resumable-uploads",
            true,
        )
        .unwrap();
    let after = client.branch_state(&root, "main").unwrap();
    assert_eq!(after.ahead, 0);

    fs::write(root.join("hello world.txt"), "x\n").unwrap();
    let dirty = client.snapshot(&root).unwrap();
    assert!(dirty
        .dirty_paths
        .iter()
        .any(|p| p.contains("hello world.txt")));

    assert_no_force(&[
        "push".into(),
        "-u".into(),
        "--".into(),
        remote_name,
        "feature/42-add-resumable-uploads:feature/42-add-resumable-uploads".into(),
    ]);
    let _ = h.remote;
}

#[test]
fn create_branch_resolves_start_from_listed_remote() {
    let h = harness();
    git(&h.home, &h.work, &["checkout", "-b", "remote-base", "main"]);
    git(&h.home, &h.work, &["push", "origin", "remote-base"]);
    git(&h.home, &h.work, &["checkout", "main"]);
    git(&h.home, &h.work, &["branch", "-D", "remote-base"]);
    git(
        &h.home,
        &h.work,
        &["remote", "rename", "origin", "upstream"],
    );

    let client = ProcessGitClient::new(StdProcessRunner).with_extra_env(h.extra_env);
    client
        .create_branch(&h.work, "feature/from-remote", "remote-base")
        .unwrap();

    let snapshot = client.snapshot(&h.work).unwrap();
    assert_eq!(snapshot.branch.as_deref(), Some("feature/from-remote"));
    assert_eq!(snapshot.remotes[0].name, "upstream");
}
