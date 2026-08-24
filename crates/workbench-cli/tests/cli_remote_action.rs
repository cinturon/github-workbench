use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const REMOTE_URL: &str = "git@github.com:acme/widgets.git";

struct Harness {
    _temp: TempDir,
    home: PathBuf,
    gitconfig: PathBuf,
    data_dir: PathBuf,
    work: PathBuf,
    bare: PathBuf,
    gh_program: PathBuf,
}

impl Harness {
    fn new() -> Self {
        let temp = TempDir::new().expect("create temporary test directory");
        let home = temp.path().join("home");
        let data_dir = temp.path().join("data");
        let work = temp.path().join("work");
        let bare = temp.path().join("remote.git");
        fs::create_dir_all(&home).expect("create isolated home");
        fs::create_dir_all(&work).expect("create working repository");

        let gitconfig = home.join("gitconfig");
        fs::write(
            &gitconfig,
            format!(
                "[user]\n    name = Workbench Test\n    email = workbench@example.test\n\
                 [init]\n    defaultBranch = main\n\
                 [commit]\n    gpgsign = false\n\
                 [url \"{}\"]\n    insteadOf = {REMOTE_URL}\n",
                bare.display()
            ),
        )
        .expect("write isolated git config");

        run_git(
            &home,
            &gitconfig,
            temp.path(),
            &["init", "--bare", "-b", "main", path(&bare)],
        );
        run_git(&home, &gitconfig, &work, &["init", "-b", "main"]);
        fs::write(
            work.join("action.yml"),
            "name: Smoke composite\nruns:\n  using: composite\n  steps:\n    - shell: bash\n      run: echo \"Upload completed\"\n",
        )
        .expect("write composite action");
        fs::create_dir_all(work.join(".github-workbench/tests"))
            .expect("create test case directory");
        fs::write(
            work.join(".github-workbench/tests/smoke-composite.yml"),
            "schema-version: 1\nname: smoke-composite\naction:\n  path: .\nrunner:\n  os: [ubuntu-latest]\npermissions:\n  contents: read\ninputs: {}\nenvironment: {}\nexpect:\n  conclusion: success\n  logs:\n    - contains: Upload completed\n",
        )
        .expect("write remote test case");
        run_git(&home, &gitconfig, &work, &["add", "."]);
        run_git(&home, &gitconfig, &work, &["commit", "-m", "initial"]);
        run_git(
            &home,
            &gitconfig,
            &work,
            &["remote", "add", "origin", REMOTE_URL],
        );
        run_git(&home, &gitconfig, &work, &["push", "-u", "origin", "main"]);

        let gh_program = temp.path().join("fixture-gh");
        fs::write(
            &gh_program,
            r#"#!/bin/sh
set -eu
command_name="${1:-}"
subcommand="${2:-}"
if [ "$command_name" = "auth" ] && [ "$subcommand" = "status" ]; then
  exit 0
fi
sha="$(git rev-parse HEAD)"
workflow="$(basename "$(find .github/workflows -name 'github-workbench-test-*.yml' -print -quit)")"
if [ "$command_name" = "api" ]; then
  case "$*" in
    *"/git/ref/heads/"*)
      ref="${4#*git/ref/}"
      ref_sha="$(git --git-dir="$HOME/../remote.git" rev-parse "$ref")"
      printf '{"object":{"sha":"%s"}}\n' "$ref_sha"
      ;;
    *"/git/refs/heads/"*)
      ref="${4#*git/refs/}"
      git --git-dir="$HOME/../remote.git" update-ref -d "$ref"
      ;;
    *"/actions/runs/42"*)
      printf '{"id":42,"head_sha":"%s","path":".github/workflows/%s","status":"completed","conclusion":"success","html_url":"https://github.com/acme/widgets/actions/runs/42"}\n' "$sha" "$workflow"
      ;;
    *"/actions/runs"*)
      printf '{"workflow_runs":[{"id":42,"head_sha":"%s","path":".github/workflows/%s","status":"completed","conclusion":"success","html_url":"https://github.com/acme/widgets/actions/runs/42"}]}\n' "$sha" "$workflow"
      ;;
    *)
      printf '%s\n' "unexpected api call: $*" >&2
      exit 9
      ;;
  esac
  exit 0
fi
if [ "$command_name" = "run" ] && [ "$subcommand" = "download" ]; then
  while [ "$#" -gt 0 ]; do
    if [ "$1" = "--dir" ]; then
      shift
      destination="$1"
      break
    fi
    shift
  done
  session_id="$(basename "$destination")"
  mkdir -p "$destination"
  printf '{"schema_version":1,"session_id":"%s","case":"smoke-composite","runner":"ubuntu-latest","action_outcome":"success","outputs":{}}\n' "$session_id" > "$destination/github-workbench-result.json"
  exit 0
fi
if [ "$command_name" = "run" ] && [ "$subcommand" = "view" ]; then
  printf 'Run action under test\nUpload completed\n'
  exit 0
fi
printf '%s\n' "unexpected gh call: $*" >&2
exit 9
"#,
        )
        .expect("write gh fixture");
        let mut permissions = fs::metadata(&gh_program)
            .expect("read gh fixture metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&gh_program, permissions).expect("make gh fixture executable");
        fs::write(
            &gitconfig,
            "[user]\n    name = Workbench Test\n    email = workbench@example.test\n\
             [init]\n    defaultBranch = main\n\
             [commit]\n    gpgsign = false\n",
        )
        .expect("hide URL rewrite while mapping the GitHub repository");

        Self {
            _temp: temp,
            home,
            gitconfig,
            data_dir,
            work,
            bare,
            gh_program,
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
            .env("GWW_GH_PROGRAM", &self.gh_program)
            .output()
            .expect("run gww")
    }

    fn enable_remote_mapping(&self) {
        fs::write(
            &self.gitconfig,
            format!(
                "[user]\n    name = Workbench Test\n    email = workbench@example.test\n\
                 [init]\n    defaultBranch = main\n\
                 [commit]\n    gpgsign = false\n\
                 [url \"{}\"]\n    insteadOf = {REMOTE_URL}\n",
                self.bare.display()
            ),
        )
        .expect("enable local GitHub URL rewrite");
    }

    fn make_log_assertion_fail(&self) {
        let script = fs::read_to_string(&self.gh_program).expect("read gh fixture");
        let script = script.replace(
            r"Run action under test\nUpload completed\n",
            r"Run action under test\nUnexpected output\n",
        );
        fs::write(&self.gh_program, script).expect("update gh fixture");
    }
}

#[test]
fn remote_action_cli_runs_watches_and_cleans_up_a_fixture_session() {
    let harness = Harness::new();

    assert_exit(&harness.gww(&["open", "."]), 0, "gww open");
    harness.enable_remote_mapping();

    let discovered = harness.gww(&["action", "discover"]);
    assert_exit(&discovered, 0, "gww action discover");
    assert!(stdout(&discovered).contains("Smoke composite"));
    assert!(stdout(&discovered).contains("smoke-composite"));

    let tested = harness.gww(&["action", "test", "smoke-composite", "--yes"]);
    assert_exit(&tested, 0, "gww action test");
    let tested_stdout = stdout(&tested);
    assert!(
        tested_stdout.contains("Assertions: passed"),
        "{tested_stdout}"
    );
    assert!(tested_stdout.contains("Run URL: https://github.com/acme/widgets/actions/runs/42"));
    let session_id = value_after(&tested_stdout, "Session id: ");
    assert_eq!(session_id.len(), 26);

    let remote_ref = format!("refs/heads/github-workbench/test/{session_id}");
    assert!(
        !bare_ref(&harness, &remote_ref).is_empty(),
        "temporary remote ref was not pushed"
    );
    let commit_before_watch = git_stdout(
        &harness.home,
        &harness.gitconfig,
        &harness.work,
        &["rev-parse", "HEAD"],
    );
    let refs_before_watch = bare_refs(&harness);

    let listed = harness.gww(&["runs", "list"]);
    assert_exit(&listed, 0, "gww runs list");
    assert!(stdout(&listed).contains(&session_id));
    assert!(stdout(&listed).contains("passed"));

    let watched = harness.gww(&["runs", "watch", &session_id]);
    assert_exit(&watched, 0, "gww runs watch");
    assert!(stdout(&watched).contains("Assertions: passed"));
    assert_eq!(
        git_stdout(
            &harness.home,
            &harness.gitconfig,
            &harness.work,
            &["rev-parse", "HEAD"],
        ),
        commit_before_watch
    );
    assert_eq!(bare_refs(&harness), refs_before_watch);

    let cleanup = harness.gww(&["cleanup", "list"]);
    assert_exit(&cleanup, 0, "gww cleanup list");
    let cleanup_stdout = stdout(&cleanup);
    let item_id = cleanup_stdout
        .lines()
        .nth(1)
        .and_then(|line| line.split_whitespace().next())
        .expect("cleanup list has one item");
    assert!(cleanup_stdout.contains(&format!("origin/github-workbench/test/{session_id}")));

    let cleaned = harness.gww(&["cleanup", "run", item_id, "--yes"]);
    assert_exit(&cleaned, 0, "gww cleanup run");
    assert!(stdout(&cleaned).contains("Cleanup completed."));
    assert!(bare_ref(&harness, &remote_ref).is_empty());
    assert!(!bare_ref(&harness, "refs/heads/main").is_empty());
}

#[test]
fn assertion_failure_prints_persisted_result_for_execute_and_watch() {
    let harness = Harness::new();
    assert_exit(&harness.gww(&["open", "."]), 0, "gww open");
    harness.enable_remote_mapping();
    harness.make_log_assertion_fail();

    let tested = harness.gww(&["action", "test", "smoke-composite", "--yes"]);
    assert_exit(&tested, 1, "gww action test with failed assertion");
    let tested_stdout = stdout(&tested);
    for expected in [
        "Run URL: https://github.com/acme/widgets/actions/runs/42",
        "Conclusion: success",
        "Assertions: failed",
        "Manifest:",
        "Logs:",
        "Cleanup: run `gww cleanup list`",
    ] {
        assert!(tested_stdout.contains(expected), "{tested_stdout}");
    }
    let tested_stderr = String::from_utf8_lossy(&tested.stderr);
    assert!(!tested_stderr.contains("gww runs show"), "{tested_stderr}");
    assert!(tested_stderr.contains("gww runs list"), "{tested_stderr}");
    assert!(tested_stderr.contains("gww runs watch"), "{tested_stderr}");

    let session_id = value_after(&tested_stdout, "Session id: ");
    let watched = harness.gww(&["runs", "watch", &session_id]);
    assert_exit(&watched, 1, "gww runs watch with failed assertion");
    let watched_stdout = stdout(&watched);
    assert!(
        watched_stdout.contains("Run URL: https://github.com/acme/widgets/actions/runs/42"),
        "{watched_stdout}"
    );
    assert!(
        watched_stdout.contains("Assertions: failed"),
        "{watched_stdout}"
    );
}

fn bare_ref(harness: &Harness, reference: &str) -> String {
    git_stdout(
        &harness.home,
        &harness.gitconfig,
        harness._temp.path(),
        &["--git-dir", path(&harness.bare), "show-ref", reference],
    )
}

fn bare_refs(harness: &Harness) -> String {
    git_stdout(
        &harness.home,
        &harness.gitconfig,
        harness._temp.path(),
        &["--git-dir", path(&harness.bare), "show-ref"],
    )
}

fn value_after(output: &str, prefix: &str) -> String {
    output
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .map(str::to_owned)
        .unwrap_or_else(|| panic!("output did not contain {prefix:?}:\n{output}"))
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
    if !output.status.success() && output.status.code() != Some(1) {
        panic!("git {args:?} failed:\n{}", combined(&output));
    }
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
