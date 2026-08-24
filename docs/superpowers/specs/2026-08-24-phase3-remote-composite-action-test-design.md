# Phase 3: Remote Composite-Action Test

**Status:** Approved for implementation planning  
**Date:** 2026-08-24  
**Product:** GitHub Workflow Workbench  
**Depends on:** Phase 2 local repository vertical slice (`docs/superpowers/specs/2026-08-23-phase2-local-repository-vertical-slice-design.md`). Implementation of this phase expects Phase 2 to be present on the default branch (Git adapter, SQLite journal, `gww` open/status/issue/push/ops).  
**Product design:** `docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md` (§6.4, §7.5–7.8, §9–10.1, §11, §12, §16, §17.1 Action tests / §17.3, §18, §24 Phase 3)

---

## 1. Goal

Deliver an end-to-end remote test of one local composite GitHub Action change on `ubuntu-latest`: discover the action, parse one declarative test case, generate a workflow, push an ephemeral test branch, watch the Actions run through `gh`, download the result manifest and logs, evaluate pass/fail locally, and offer safe cleanup.

Ship both the `gww` CLI and a thin Tauri **Action Tests** screen that call the same application use cases. No live `github.com` calls in CI.

## 2. Decisions locked

| Decision | Choice |
|---|---|
| User-facing surface | CLI + thin Tauri Action Tests (list / start / watch / result); cleanup CLI-first |
| GitHub access | `gh` process adapter only (`gh auth status`, `gh api`, related read/download commands via argv) |
| Isolation | Ephemeral branch in the source repository: `github-workbench/test/<session-id>` (product §10.1) |
| Assertions | Minimal: expected `conclusion` + optional log `contains` / `not-contains`; result manifest required |
| Runners / runtimes | `ubuntu-latest` and composite actions only |
| Approach | Single Phase 3 vertical slice (domain + adapters + CLI + Tauri) |
| CI | No live `github.com`; recorded `gh` fixtures; optional documented opt-in live e2e |
| Force push | Never |
| Draft PRs / issue fetch | Out of scope (Phase 4) |

## 3. Scope

### In scope

- **Domain (`workbench-domain` / `testing/`):**
  - Discoverable `ActionDefinition` from `action.yml` / `action.yaml` (composite only; other runtimes → structured skip/warn findings).
  - Minimal test-case schema under `.github-workbench/tests/*.yml` (see §5).
  - Normalize to `TestPlan` (defaults, string inputs, runner label validation, single-job Ubuntu matrix).
  - Generate deterministic workflow YAML with unique session id, minimal permissions, concurrency group, and Ubuntu Bash result-manifest collector + artifact upload.
  - Evaluate assertions from downloaded result manifest + completed logs.
- **`workbench-github`:** Process-based `GithubClient` over `gh` argv (no shell strings). Auth status, workflow run list/get, artifact download, log download as needed for watch/assert.
- **`workbench-application`:** Use cases for discover, plan remote test, execute session (Git push + correlate + watch + download + assert), list/watch sessions, list/run cleanup. Reuse Phase 2 `GitClient`, `OperationStore`, journaling, plan confirmation.
- **`workbench-storage`:** Migrations for `test_sessions` and `cleanup_items` (product §16.3 subset); paths to redacted local artifacts only—never tokens/secrets.
- **`workbench-cli` (`gww`):** Commands in §7; keep Phase 2 commands unchanged.
- **Tauri shell:** Action Tests screen only—list discovered actions and tests, start (show plan + confirm), live watch, pass/fail + run URL / local evidence paths. No Home / Changes / PR screens.
- Fixture-based adapter and use-case tests; optional manual live e2e document.

### Out of scope

- Draft pull requests, issue search/fetch, reviewers, merge (Phase 4).
- Dedicated sandbox repository isolation (§10.2) and default-branch harness mode (§10.3).
- Windows / macOS runners; Node and Docker action runtimes.
- Full product §9 assertions (outputs equals, file snapshots, tags matrix, destructive confirmation UX beyond refusal).
- Literal or mapped repository secrets in test definitions.
- OAuth, direct REST/GraphQL client replacing `gh`, org policy.
- Force-push, rebase UI, worktree management, branch deletion except exact ephemeral test-ref cleanup.
- Full desktop IA (Home, Changes, PR, History screens).

## 4. Architecture

### Layering

```text
gww (CLI) ──┐
            ├──→ workbench-application (use cases)
Tauri UI ───┘         → workbench-domain (action / test / plan / assert / workflow IR)
                      → GitClient → workbench-git                 [Phase 2]
                      → GithubClient → workbench-github           [Phase 3]
                      → OperationStore / TestSessionStore → workbench-storage
                      → ProcessRunner / Clock / IdGenerator / PolicySource
```

CLI and Tauri stay thin. Business rules remain in domain + application. Tauri must not reimplement planning, assertion, or cleanup safety.

### Ports (application)

Extend Phase 2 ports:

- `GithubClient` — auth status; resolve owner/repo from project mapping; list/get workflow runs by commit SHA and/or workflow file name; download artifact by name; download job logs. Returns structured results / `AppError` (including auth → exit 4).
- `TestSessionStore` — create/update/list sessions; attach local evidence paths and result JSON; enqueue/list/complete cleanup items with `expected_identity`.
- Existing `GitClient`, `OperationStore`, `ProcessRunner`, `Clock`, `IdGenerator`, `PolicySource`.

### Data flow (remote test)

1. `gh auth status` — fail closed with exit `4` if unauthenticated.
2. Discover actions / load one test case; domain builds `TestPlan` or returns validation findings. Invalid test/action YAML and unsupported Phase 3 features → exit `2`. Repository policy blockers (when evaluated) → exit `3`.
3. Application builds an `OperationPlan` whose steps cover: write generated workflow file(s), create ephemeral branch from HEAD, commit only generated test infrastructure, push ref, correlate run, watch to terminal, download manifest/logs, assert, enqueue cleanup. Watch/download/assert may run in the same operation or resume via `gww runs watch` after a successful push.
4. CLI/UI print the plan (outcome, preconditions, local Git commands, `gh`/API mutations, files generated, remote refs, expected jobs / minutes caveat, cleanup behavior, risk).
5. On confirmation / `--yes`, executor runs allowlisted steps; each step journaled.
6. On terminal run: download result-manifest artifact + logs; domain evaluates assertions; persist `test_sessions` row; enqueue `cleanup_items`.
7. Cleanup (CLI) deletes only the exact reserved remote ref when `expected_identity` still matches; refuse if the remote ref moved.

### Database

Default path unchanged (`GWW_DATA_DIR` / platform data dir). Add:

```text
test_sessions(id, project_id, session_key, commit_sha, remote_ref, workflow_name,
              run_id, status, result_json, evidence_dir, created_at, updated_at)
cleanup_items(id, project_id, resource_kind, resource_id, expected_identity,
              due_at, status, created_at, updated_at)
```

Database remains disposable; action sources and `.github-workbench/tests/` remain durable.

## 5. Minimal test-case schema (Phase 3)

Committed tests live at `.github-workbench/tests/<name>.yml`.

```yaml
schema-version: 1
name: smoke-composite
description: Optional one-line description.

action:
  path: .   # relative to repo root; must resolve to composite action.yml/yaml

runner:
  os:
    - ubuntu-latest
  timeout-minutes: 10   # optional; default from policy remote-testing or 15

permissions:
  contents: read

inputs: {}          # optional; all values strings
environment: {}     # optional; non-secret only

expect:
  conclusion: success
  logs:               # optional
    - contains: "Upload completed"
    - not-contains: "secret="
```

**Rejected in Phase 3 (usage/config error, no mutation):** missing `action.path`, non-composite runtime, runners other than `ubuntu-latest`, matrix with >1 OS, literal secret-looking values in `environment`/`inputs` when they match a conservative denylist (e.g. keys named `*SECRET*`, `*TOKEN*`, `*PASSWORD*`), `schema-version` ≠ 1, unknown fields (strict parse, consistent with policy YAML).

Tags, output equals, file snapshots, and destructive markers are deferred.

### Result manifest (required)

Generated workflows write and upload a JSON artifact (name stable, e.g. `github-workbench-result`) shaped like:

```json
{
  "schema_version": 1,
  "session_id": "01JABC...",
  "case": "smoke-composite",
  "runner": "ubuntu-latest",
  "action_outcome": "success",
  "outputs": {}
}
```

Assertion evaluation rules (locked):

1. GitHub workflow run `conclusion` must equal `expect.conclusion`.
2. Manifest `action_outcome` must equal `expect.conclusion`.
3. Each `logs.contains` substring must appear in the downloaded job logs.
4. Each `logs.not-contains` substring must not appear in the downloaded job logs.

Missing manifest on a completed run fails assertion (2) with remediation to open the run URL. A failed GitHub conclusion fails (1) even if logs look fine.

## 6. Safety model (Phase 3)

- Repository snapshot before mutation (reuse Phase 2 fields).
- **No shell interpolation** for Git or `gh`; program + `Vec<String>` argv only.
- **Never** `--force` or `--force-with-lease`.
- Ephemeral refs **only** under `github-workbench/test/` (or policy `remote-testing.branch-prefix` when present; default prefix `github-workbench/test`).
- Test push commits **only** generated workflow / harness files on the ephemeral branch; does not rewrite the user’s feature-branch history.
- Require a clean working tree before planning a new remote test (same class of guard as Phase 2 push). Resuming watch after a successful push does not require a clean tree.
- Cleanup deletes only when remote ref tip equals `expected_identity` (ref name + commit SHA recorded at push); otherwise fail with remediation and leave the ref.
- Best-effort warn if existing workflows trigger on `push` in ways that may also run on the test branch; generated workflow sets a unique concurrency group.
- Generated workflow permissions are fixed to `contents: read` in Phase 3 (test YAML may omit `permissions` or set exactly `contents: read`; any other permission key is rejected).
- No production `environment:` in generated workflows.
- High-risk ops remain out of allowlist except confirmed temporary-ref cleanup with exact-target validation.
- Redact secrets in journaled `gh`/Git output (reuse Phase 2 redaction).

### Auth

- Before GitHub mutations or run polling: `gh auth status`.
- Unauthenticated / insufficient auth → `AppError` mapped to exit code **`4`**.
- Do not store tokens in SQLite.

## 7. CLI and Tauri commands

### CLI

| Command | Behavior |
|---|---|
| `gww action discover` | Find `action.yml` / `action.yaml` (skip common ignored build dirs); print composite actions; note skipped non-composite |
| `gww action test [name] [--yes]` | Select test by name (or sole test); print plan; execute on confirm/`--yes`; print pass/fail and session id |
| `gww runs list` | List local test sessions for the mapped project |
| `gww runs watch <session-id>` | Poll until terminal; print state transitions and conclusion |
| `gww cleanup list` | List pending cleanup items |
| `gww cleanup run <item-id> [--yes]` | Run exact-target remote ref deletion after confirm |

Phase 2 commands remain available and unchanged in contract.

Exit codes (product §18): `0` success, `1` failure, `2` invalid usage/config, `3` policy blockers, `4` auth required, `5` remote still pending when a non-waiting command returns.

### Tauri (Action Tests only)

- List discovered actions and test cases for the opened repository.
- Start: show the same plan payload as CLI; confirm; start session.
- Live watch: subscribe to session status updates (poll via application/`gh`).
- Result: pass/fail, conclusion, run URL, paths to local evidence.
- If cleanup is pending, show status and a copyable `gww cleanup …` hint; do not implement a full cleanup queue UI in Phase 3.

## 8. Error and recovery model

Reuse Phase 2 `AppError` / user-report shape (what failed, what changed, what did not, retry safety, remediation). Add or map:

- `AuthRequired` → exit `4`
- `RemotePending` → exit `5` for non-watch commands that refuse to block
- `GithubFailed` (redacted stderr/body summary)
- `ActionNotComposite` / `TestCaseInvalid`
- `RunNotCorrelated` (push succeeded; monitoring can resume via `gww runs watch`)
- `CleanupRefMoved` / `CleanupIdentityMismatch`

Interrupted sessions: if push succeeded but watch failed, journal records completed push; `gww runs watch <session-id>` resumes monitoring without re-pushing. Cleanup: enqueue items with `due_at` for display (use policy `failed-ref-retention` / `successful-ref-retention` when present; defaults 72h failed / 0h successful). Phase 3 does **not** auto-delete on a timer—users run `gww cleanup run`. Successful sessions still enqueue cleanup immediately so `cleanup list` offers deletion after confirm.

## 9. Testing strategy

- **Domain:** parse composite vs reject Node/Docker; normalize minimal tests; golden generated workflow YAML (insta); assertion unit tests (pass/fail/missing manifest/log matchers).
- **GitHub adapter:** recorded fixtures for `gh auth status`, run list/get JSON, artifact/log download; never hit `github.com` in default tests.
- **Application:** fake `GitClient` + `GithubClient` + temp/in-memory store for plan/execute/resume/cleanup identity checks.
- **Storage:** migrations + session/cleanup round-trips.
- **CLI:** argv parsing + one orchestrated happy path with fakes (or git temp repo + stubbed `gh` program).
- **Tauri:** smoke that commands invoke application ports; no live GitHub.
- **CI:** existing fmt/clippy/test; `gh` binary may be absent—adapter tests must not require a real authenticated `gh`. Document optional live e2e (secrets + disposable repo) outside required CI.

## 10. Exit criterion

Against a repository that contains one composite action and one Phase 3–valid test file (manual or opt-in live):

1. `gww action discover` lists the composite action.
2. `gww action test <name> --yes` (or Tauri start) pushes `github-workbench/test/<session-id>`, correlates an Ubuntu workflow run, and reports pass/fail from manifest + log assertions.
3. `gww runs list` / Tauri show the session; `gww runs watch` can attach to an in-flight or completed session.
4. `gww cleanup list` shows the ephemeral ref; `gww cleanup run <id> --yes` deletes it only when identity matches.

CLI alone must be able to complete the path. Tauri must cover list → start → watch → result for the same session model.

## 11. Non-goals reminder

Phase 3 proves remote composite-action testing on Ubuntu with safe ephemeral refs and shared CLI/UI use cases. It does not ship draft PRs, multi-OS matrices, non-composite runtimes, or a full desktop shell—that remains Phase 4+ / later milestones.
