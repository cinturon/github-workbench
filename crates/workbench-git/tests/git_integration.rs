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

fn git_output(home: &Path, cwd: &Path, args: &[&str]) -> String {
    let config = home.join("gitconfig");
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(sanitized_env(&[
            ("HOME".into(), home.display().to_string()),
            ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
            ("GIT_CONFIG_GLOBAL".into(), config.display().to_string()),
        ]))
        .output()
        .expect("git must be installed");
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8(output.stdout).expect("git stdout must be utf-8")
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

#[test]
fn commit_paths_push_rev_parse_and_delete_remote_ref() {
    let h = harness();
    let client = ProcessGitClient::new(StdProcessRunner).with_extra_env(h.extra_env.clone());
    let root = client.resolve_toplevel(&h.work).unwrap();
    let remote_name = client.snapshot(&root).unwrap().remotes[0].name.clone();
    let workflow_path = ".github/workflows/github-workbench-test-01JABC.yml";
    let remote_ref = "github-workbench/test/01JABC";

    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::write(
        root.join(workflow_path),
        "name: remote action test\non: workflow_dispatch\njobs:\n  noop:\n    runs-on: ubuntu-latest\n",
    )
    .unwrap();

    client
        .commit_paths(
            &root,
            "chore: add remote action test",
            &[workflow_path.into()],
        )
        .unwrap();

    let committed_paths = git_output(
        &h.home,
        &root,
        &["show", "--name-only", "--format=", "HEAD"],
    );
    assert_eq!(
        committed_paths.trim(),
        workflow_path,
        "commit should include only the generated workflow path"
    );

    let pushed_sha = client
        .rev_parse(&root, "HEAD")
        .unwrap()
        .expect("HEAD should resolve after commit_paths");

    client
        .push_ref(&root, &remote_name, "HEAD", remote_ref, false)
        .unwrap();
    client.fetch(&root, &remote_name).unwrap();

    let fetched_sha = client
        .rev_parse(&root, &format!("refs/remotes/{remote_name}/{remote_ref}"))
        .unwrap()
        .expect("pushed ref should resolve after fetch");
    assert_eq!(fetched_sha, pushed_sha);

    client
        .delete_remote_ref(&root, &remote_name, remote_ref)
        .unwrap();

    let remote_heads = git_output(&h.home, &root, &["ls-remote", "--heads", "origin"]);
    assert!(
        !remote_heads.contains(remote_ref),
        "remote ref should be deleted: {remote_heads}"
    );
}
