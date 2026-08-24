# Phase 2: Local Repository Vertical Slice

**Status:** Approved for implementation planning  
**Date:** 2026-08-23  
**Product:** GitHub Workflow Workbench  
**Depends on:** Phase 1 domain foundation (`docs/superpowers/specs/2026-08-23-phase1-domain-foundation-design.md`)  
**Product design:** `docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md` (§7.1–7.3, §12–14, §16, §18, §24 Phase 2)

---

## 1. Goal

Deliver a CLI-first vertical slice that opens a real local Git repository, creates a policy-compliant feature branch from a manually entered issue number and title, previews and executes a push to the configured remote, and journals every multi-step operation in SQLite—without a desktop UI and without GitHub API/`gh` integration.

## 2. Decisions locked

| Decision | Choice |
|---|---|
| User-facing surface | CLI only (`gww`); no Tauri/React |
| Operation persistence | SQLite via `workbench-storage` with migrations |
| Push targets | Real remotes with explicit plan confirmation; temp bare remotes in tests |
| Approach | Full vertical slice: application use cases + Git adapter + SQLite + CLI wiring |
| Issue input | Manual number + title (no `gh` issue fetch) |
| Force push | Never in Phase 2 |

## 3. Scope

### In scope

- **`workbench-git`**: Process-based Git client using program + argv only (no shell strings, no `libgit2`). Support at least: resolve toplevel, status (branch, dirty paths), fetch, create branch, checkout, push, ahead/behind (or equivalent parsing).
- **`workbench-storage`**: SQLite schema + migrations for `projects`, `operations`, `operation_steps` (subset of product design §16). Redacted step output paths or bounded text.
- **`workbench-application`**: Concrete ports and use cases: open repository, status, start issue branch (plan → confirm → execute), plan push, execute push, list operations. Orchestrates domain planners from Phase 1.
- **`workbench-cli` (`gww`)**: Commands listed in §6.
- Repository snapshot before mutation; typed operation plans; journal step states.
- Temporary-repository Git integration tests with isolated Git config/identity.
- SQLite unit/integration tests; application tests with fake ports plus one real-Git happy path.

### Out of scope

- Tauri / React desktop UI (defer; product design’s “desktop” exit wording is adapted to CLI for this phase).
- `gh` / GitHub REST/GraphQL / draft PRs / Action remote testing.
- Force-push, rebase execution, worktree management, merge, branch deletion/cleanup.
- Interactive staging / commit authoring UI (Phase 2 pushes already-committed work on the feature branch; see §5).
- OAuth, secret storage, telemetry.

## 4. Architecture

### Layering

```text
gww (CLI)
  → workbench-application (use cases)
      → workbench-domain (plans, policy, naming)     [existing]
      → GitClient port → workbench-git
      → OperationStore port → workbench-storage (SQLite)
      → ProcessRunner / Clock / IdGenerator
```

CLI handlers stay thin. Business rules remain in domain + application.

### Ports (application)

- `GitClient` — typed methods for the Git operations above; returns structured results / errors (not raw exit codes alone).
- `OperationStore` — create operation, append/update steps, list recent by project.
- `ProcessRunner` — `program`, `args: Vec<String>`, `cwd`, sanitized env; capture stdout/stderr separately; support cancellation later (stub OK).
- `Clock`, `IdGenerator` — injectable for deterministic tests.

`workbench-github` remains a stub in Phase 2.

### Data flow

1. User runs a mutating command.
2. Use case loads project + policy + Git snapshot.
3. Domain builds an `OperationPlan` (reuse `plan_create_branch_from_issue`; add push planner as needed).
4. CLI prints the plan (commands, risk, preconditions, rationale).
5. On confirmation, executor runs only allowlisted `GitCommand` variants from the plan.
6. Each step is journaled (`pending` → `running` → `succeeded`/`failed`) with redacted output.

### Database location

Default: platform app-data directory (e.g. `%LOCALAPPDATA%/github-workbench/workbench.db` on Windows; XDG data home on Unix). Override with `GWW_DATA_DIR`. Database is disposable; policy YAML in the repo remains source of truth.

## 5. Safety model (Phase 2)

- Record a repository snapshot before mutation: root, current branch/detached HEAD, HEAD OID, dirty paths, remotes, selected remote, upstream when present.
- **No shell interpolation**; argv only.
- **Never** `--force` or `--force-with-lease` in Phase 2.
- `gww push --plan` is dry-run only.
- `gww push` requires interactive confirmation or `--yes`.
- **Block push if the working tree is dirty.**
- Push executes only the planned ref update (expected remote + branch names from the plan).
- Do not assume the authoritative remote is named `origin`: use project mapping, the sole remote, or an explicit `--remote` flag.
- High-risk ops (reset, delete, force) are out of scope and must not appear in the allowlist.

### Commit policy for this phase

Phase 2 does not implement a full commit UX. The happy path is: create/checkout feature branch, user commits with their own Git tooling if needed, then `gww push`. If the branch has no commits ahead of base, push planning should explain that there is nothing to push.

## 6. CLI commands

| Command | Behavior |
|---|---|
| `gww open <path>` | Resolve toplevel; detect remotes; load `.github-workbench.yml` if present (invalid policy → error, no mutation); upsert `projects`; print summary |
| `gww status [--json]` | Current branch, dirty state, ahead/behind when upstream exists, policy findings summary, recommended next action |
| `gww issue start <number> --title <text> [--yes]` | Build create-branch plan via domain; print plan; execute on confirm/`--yes`; journal |
| `gww push --plan` | Build and print push plan only |
| `gww push [--yes]` | Execute push plan after confirm; dirty-tree block; journal |
| `gww ops list` | List recent operations from SQLite |

Exit codes (design §18): `0` success, `1` failure, `2` invalid usage/config, `3` policy blockers. Auth exit `4` is unused in Phase 2.

## 7. Error model

Reuse Phase 1 `WorkbenchError` where applicable. Add adapter/application errors such as:

- `GitUnavailable`
- `GitFailed { program, args_summary, status, stderr_redacted }`
- `DirtyWorkingTree { paths }` (block push)
- `RemoteNotResolved` / `RepositoryNotMapped` when remote selection is ambiguous

User-facing errors must state what failed, what already changed, what did not, whether retry is safe, and remediation.

## 8. Testing strategy

- **Git adapter integration:** temp bare remote + working clone; create branch; push; status/ahead-behind parsing; unusual paths as feasible; isolated config (`GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` disabled or redirected) and fixed `user.name` / `user.email`.
- **Storage:** migrations apply cleanly; operation + step CRUD round-trips.
- **Application:** fake `GitClient` + in-memory or temp SQLite for use-case unit tests; one end-to-end happy path with real Git.
- **CI:** existing fmt/clippy/test; ensure Git is available on the runner (ubuntu-latest default is fine). No live GitHub.com calls.

## 9. Exit criterion

Using only the CLI against a local clone with a configured remote:

1. `gww open` identifies the repository and records the project.
2. `gww issue start <n> --title …` creates a GitHub Flow–compliant branch after plan confirmation.
3. `gww push --plan` shows the intended remote ref update.
4. `gww push --yes` (or confirmed push) publishes the branch when the tree is clean and commits exist to push.
5. `gww ops list` shows journaled steps for the operations.

No desktop UI required. No `gh` required.

## 10. Non-goals reminder

Phase 2 proves safe local Git coordination and journaling. It does not ship GitHub collaboration features or Action testing—that remains Phase 3+.
