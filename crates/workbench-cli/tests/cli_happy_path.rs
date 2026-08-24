use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const FEATURE_BRANCH: &str = "feature/42-add-resumable-uploads";

struct Harness {
    _temp: TempDir,
    home: PathBuf,
    gitconfig: PathBuf,
    data_dir: PathBuf,
    remote: PathBuf,
    work: PathBuf,
    remote_name: String,
}

impl Harness {
    fn new() -> Self {
        let temp = TempDir::new().expect("create temporary test directory");
        let home = temp.path().join("home");
        let data_dir = temp.path().join("data");
        fs::create_dir_all(&home).expect("create isolated home");
        fs::create_dir_all(&data_dir).expect("create isolated data directory");

        let gitconfig = home.join("gitconfig");
        fs::write(
            &gitconfig,
            "[user]\n    name = Workbench Test\n    email = workbench@example.test\n[init]\n    defaultBranch = main\n[commit]\n    gpgsign = false\n",
        )
        .expect("write isolated git config");

        let remote = temp.path().join("remote.git");
        let work = temp.path().join("work");
        run_git(
            &home,
            &gitconfig,
            temp.path(),
            &["init", "--bare", "-b", "main", path(&remote)],
        );
        run_git(
            &home,
            &gitconfig,
            temp.path(),
            &[
                "clone",
                "--origin",
                "workbench-test",
                path(&remote),
                path(&work),
            ],
        );

        let remote_name = git_stdout(&home, &gitconfig, &work, &["remote"]);
        assert!(!remote_name.is_empty(), "clone must configure a remote");
        assert_eq!(
            remote_name.lines().count(),
            1,
            "test clone must have exactly one remote"
        );

        fs::write(work.join("README.md"), "initial\n").expect("write initial content");
        run_git(&home, &gitconfig, &work, &["add", "README.md"]);
        run_git(&home, &gitconfig, &work, &["commit", "-m", "initial"]);
        run_git(
            &home,
            &gitconfig,
            &work,
            &["push", "-u", &remote_name, "main"],
        );

        Self {
            _temp: temp,
            home,
            gitconfig,
            data_dir,
            remote,
            work,
            remote_name,
        }
    }

    fn gww(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_gww"))
            .args(args)
            .current_dir(&self.work)
            .env("HOME", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &self.gitconfig)
            .env("GWW_DATA_DIR", &self.data_dir)
            .output()
            .expect("run gww")
    }
}

#[test]
fn real_git_cli_happy_path_meets_phase_two_exit_criteria() {
    let harness = Harness::new();

    let opened = harness.gww(&["open", path(&harness.work)]);
    assert_exit(&opened, 0, "gww open");
    assert!(
        stdout(&opened).contains(&harness.work.canonicalize().unwrap().display().to_string()),
        "open output did not contain repository root:\n{}",
        combined(&opened)
    );

    let database_before_invalid_policy =
        fs::read(harness.data_dir.join("workbench.db")).expect("read project database");
    fs::write(
        harness.work.join(".github-workbench.yml"),
        "typo-field: true\n",
    )
    .expect("write invalid policy");
    let invalid = harness.gww(&["open", path(&harness.work)]);
    assert_exit(&invalid, 2, "gww open with invalid policy");
    assert_eq!(
        git_stdout(
            &harness.home,
            &harness.gitconfig,
            &harness.work,
            &["branch", "--show-current"],
        ),
        "main"
    );
    assert_eq!(
        fs::read(harness.data_dir.join("workbench.db")).expect("reread project database"),
        database_before_invalid_policy,
        "invalid policy must not mutate SQLite"
    );
    fs::remove_file(harness.work.join(".github-workbench.yml")).expect("remove invalid policy");

    let started = harness.gww(&[
        "issue",
        "start",
        "42",
        "--title",
        "Add resumable uploads",
        "--yes",
    ]);
    assert_exit(&started, 0, "gww issue start");
    assert_eq!(
        git_stdout(
            &harness.home,
            &harness.gitconfig,
            &harness.work,
            &["branch", "--show-current"],
        ),
        FEATURE_BRANCH
    );

    fs::write(harness.work.join("uploads.txt"), "resumable\n").expect("write feature content");
    run_git(
        &harness.home,
        &harness.gitconfig,
        &harness.work,
        &["add", "uploads.txt"],
    );
    run_git(
        &harness.home,
        &harness.gitconfig,
        &harness.work,
        &["commit", "-m", "add resumable uploads"],
    );

    let plan = harness.gww(&["push", "--plan"]);
    assert_exit(&plan, 0, "gww push --plan");
    assert!(
        stdout(&plan).contains(&harness.remote_name),
        "push plan did not name the clone remote:\n{}",
        combined(&plan)
    );
    assert!(
        stdout(&plan).contains(&format!("{FEATURE_BRANCH}:{FEATURE_BRANCH}")),
        "push plan did not contain the feature refspec:\n{}",
        combined(&plan)
    );
    assert!(
        remote_feature_ref(&harness).is_empty(),
        "push plan unexpectedly created the remote feature branch"
    );

    fs::write(harness.work.join("dirty.txt"), "not committed\n").expect("dirty working tree");
    let dirty_push = harness.gww(&["push", "--yes"]);
    assert_exit(&dirty_push, 1, "gww push --yes with dirty tree");
    assert!(
        combined(&dirty_push).to_lowercase().contains("dirty"),
        "dirty-tree error did not explain the block:\n{}",
        combined(&dirty_push)
    );
    fs::remove_file(harness.work.join("dirty.txt")).expect("restore clean working tree");

    let pushed = harness.gww(&["push", "--yes"]);
    assert_exit(&pushed, 0, "gww push --yes");
    assert!(
        remote_feature_ref(&harness).contains(&format!("refs/heads/{FEATURE_BRANCH}")),
        "remote feature branch was not published"
    );

    let status = harness.gww(&["status", "--json"]);
    assert_exit(&status, 0, "gww status --json");
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status stdout is JSON");
    assert_eq!(status_json["dirty"], false);
    assert!(
        status_json["recommended_next_action"]
            .as_str()
            .is_some_and(|action| !action.is_empty()),
        "recommended_next_action must be non-empty"
    );

    let operations = harness.gww(&["ops", "list"]);
    assert_exit(&operations, 0, "gww ops list");
    let operations_stdout = stdout(&operations);
    for expected in [
        "create-branch-from-issue",
        "create-branch  succeeded",
        "push  succeeded",
        "push-ref  succeeded",
    ] {
        assert!(
            operations_stdout.contains(expected),
            "operations output did not contain {expected:?}:\n{}",
            combined(&operations)
        );
    }

    let empty_plan = harness.gww(&["push", "--plan"]);
    assert_exit(&empty_plan, 0, "gww push --plan after push");
    assert!(
        stdout(&empty_plan).contains("Nothing to push"),
        "empty push plan was not explained:\n{}",
        combined(&empty_plan)
    );
}

fn remote_feature_ref(harness: &Harness) -> String {
    git_stdout(
        &harness.home,
        &harness.gitconfig,
        &harness.work,
        &[
            "ls-remote",
            "--heads",
            &harness.remote_name,
            &format!("refs/heads/{FEATURE_BRANCH}"),
        ],
    )
}

fn run_git(home: &Path, gitconfig: &Path, cwd: &Path, args: &[&str]) {
    let output = git_output(home, gitconfig, cwd, args);
    assert!(
        output.status.success(),
        "git {args:?} failed:\n{}",
        combined(&output)
    );
}

fn git_stdout(home: &Path, gitconfig: &Path, cwd: &Path, args: &[&str]) -> String {
    let output = git_output(home, gitconfig, cwd, args);
    assert!(
        output.status.success(),
        "git {args:?} failed:\n{}",
        combined(&output)
    );
    stdout(&output).trim().to_string()
}

fn git_output(home: &Path, gitconfig: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", gitconfig)
        .output()
        .expect("git must be installed")
}

fn assert_exit(output: &Output, expected: i32, command: &str) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{command} returned an unexpected exit code:\n{}",
        combined(output)
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn combined(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn path(path: &Path) -> &str {
    path.to_str().expect("test paths must be UTF-8")
}
