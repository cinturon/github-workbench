# Phase 3 Critical and Important Findings Fix Report

- Status: All two Critical and three Important findings are fixed.
- Code commit: `afb5e82 fix: harden Phase 3 remote operations`.

## Critical evidence

1. Cleanup race
   - `execute_cleanup` now fetches and verifies the expected remote-tracking SHA twice immediately before deletion, then fetches and checks again after deletion.
   - A moved/recreated ref produces `CleanupRefMoved`; a surviving expected ref produces `OperationFailed`; either outcome leaves cleanup pending.
   - The residual non-force Git TOCTOU window is documented in the implementation.
   - Tests: `cleanup_ref_move_between_pre_delete_checks_is_refused`, `cleanup_ref_recreated_after_delete_is_reported_and_left_pending`, and the four-fetch command-sequence assertion in `crates/workbench-application/tests/cleanup.rs`.

2. Desktop caller-controlled plans
   - `DesktopState` owns preview plans in an in-memory map keyed by a new opaque ULID. The Tauri confirmation command accepts only `preview_id`; no `RemoteTestSessionPlan`, `OperationPlan`, or `GitCommand` crosses back from the client.
   - Confirmation atomically consumes the server-side plan. The frontend retains only the opaque id for confirmation.
   - Application execution independently requires the generated relative `.github/workflows/github-workbench-test-<session>.yml` path, regenerated workflow YAML, and exactly matching `CreateBranch`, `CommitPaths`, and non-force `PushRef` commands.
   - `assert_no_force` also rejects `+` in either push ref.
   - Tests: absolute workflow path, unexpected command, force-marker execution tests in `remote_test_execute.rs`; local/remote `+` refspec tests in `workbench-git/src/argv.rs`.

## Important evidence

3. Expression injection
   - `normalize_test_case` rejects `${{` in every input and environment value, with field-specific errors.
   - Tests: `rejects_github_expressions_in_input_values` and `rejects_github_expressions_in_environment_values`.

4. CLI assertion failures
   - Both `action test` execution and `runs watch` reload and print the persisted result before returning `AssertionFailed`/exit 1.
   - Output includes run URL, conclusion, manifest/log evidence paths, and cleanup hint.
   - Remediation now names only `gww runs list` and `gww runs watch`.
   - Test: `assertion_failure_prints_persisted_result_for_execute_and_watch`.

5. Remote destination revalidation
   - Plans persist the selected remote URL. Execution re-snapshots remotes and refuses a missing/changed name+URL before workflow creation or Git mutation.
   - Test: `changed_remote_url_is_rejected_before_workflow_or_git_mutation`.

## Verification

- TDD: focused regressions were observed failing for each behavior before their implementation, then passing.
- `cargo test --workspace`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `npm run build` in `crates/workbench-desktop`: passed.

## Final Critical cleanup TOCTOU fix

- Status: Fixed and pushed on `cursor/phase3-implementation-7895`.
- Implementation commit: `c93be25 fix: delete cleanup refs through GitHub API`.
- CLI fixture follow-ups: `be06d70` and `3d12d3b`.
- `GithubClient::delete_ref_if_sha_matches` now reads the authoritative
  `object.sha` with argv-only `gh api --method GET`, returns
  `CleanupRefMoved` on mismatch, and invokes argv-only
  `gh api --method DELETE` only after a match.
- `execute_cleanup` journals that GitHub API action directly. It no longer
  executes or plans `GitCommand::DeleteRemoteRef`, so unconditional
  `git push :refs/heads/<ref>` is not in the cleanup path.
- The REST API's smaller residual GET/DELETE race is documented in code and in
  the cleanup plan because GitHub exposes no documented SHA delete precondition.
- Regressions cover GET/DELETE argv, mismatch-without-DELETE, FakeGithub
  mismatch injection, matched application deletion through GithubClient,
  absence of Git DeleteRemoteRef, operation journaling, and CLI integration.
- Fresh verification passed:
  `cargo test --workspace`;
  `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo fmt --all -- --check`.
