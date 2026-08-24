# Architecture

GitHub Workflow Workbench uses a layered Rust core with strict dependency
direction:

1. **Domain** (`workbench-domain`) — pure policy, naming, plans, assertions.
2. **Application** (`workbench-application`) — use cases over ports.
3. **Adapters** — `workbench-git`, `workbench-github`, `workbench-storage`.
4. **Presentation** — `workbench-cli` (`gww`) and `workbench-desktop` (Tauri).

**Dependency rule:** domain has no I/O; application depends on domain and port
traits only; adapters implement ports; presentation wires adapters to use
cases. **workbench-application does not depend on adapter crates** — it
consumes `GitClient`, `GithubClient`, `OperationStore`, and related port
interfaces only.

The domain layer must not perform filesystem access, spawn processes, or talk
to SQLite or other databases. Parsing, normalization, workflow generation, and
assertion evaluation stay pure and testable without adapters.

## Phase 2 — local repository slice

Phase 2 implements application use cases, the process-based Git adapter, the
SQLite project and operation journal, and the `gww` CLI. The GitHub adapter
remains a stub for manually supplied issue numbers and titles.

The Git adapter executes Git directly with argument vectors, never through a
shell command string. Workbench state defaults to the platform data directory;
`GWW_DATA_DIR` overrides that location for isolated or portable operation.

Policy parsing does not support partial nested overrides for
`BranchTypeConfig`: when `branches.feature` or `branches.fix` is present, its
`pattern`, `start-from`, and `require-issue` fields are all required.

## Phase 3 — remote composite-action testing

Phase 3 plans and executes one composite action test on `ubuntu-latest` per
session. Two plan types coordinate the work:

- **`OperationPlan`** (Git) — create branch, commit generated workflow paths,
  and push the temporary test branch. Shared with Phase 2 push/issue flows.
- **`RemoteTestSessionPlan`** — session identity, branch and workflow naming,
  GitHub correlation, polling, evidence download, assertion evaluation, and
  cleanup enqueue. Distinct from Git ref operations.

### Persisted resume sequence

1. Plan and execute a remote test; persist session state after push.
2. Correlate and poll the workflow run; download manifest and logs into
   `GWW_DATA_DIR/evidence/<session-id>/`.
3. Evaluate assertions against the result manifest and optional log file.
4. Enqueue cleanup with an exact expected remote-ref identity.
5. `gww runs watch <session-id>` resumes from stored state without replanning
   or pushing a new branch.

### Manifest and log assertion flow

The workflow uploads artifact `github-workbench-result` containing
`github-workbench-result.json`. The application downloads evidence, parses the
manifest, and evaluates optional log `contains` / `not-contains` rules from the
declarative test case. A result manifest is required even when the workflow
concludes with failure.

### GitHub and cleanup adapters

GitHub access uses a process adapter that invokes `gh` with program plus
`Vec<String>` arguments only — never shell command strings. Cleanup validates
exact-ref identity: delete the recorded temporary remote ref only when
`rev-parse` confirms the current SHA matches the expected identity captured at
enqueue time. If the ref moved, cleanup refuses deletion.

### Safety invariants

- **Never force push** — the Git adapter rejects `--force`, `--force-with-lease`,
  and equivalent force refspecs in generated argv and defensive checks.
- **No live GitHub traffic in CI** — default CI sets
  `GWW_GH_PROGRAM=gww-gh-live-access-is-disabled-in-ci`. Fixture-driven tests
  may override `GWW_GH_PROGRAM` locally. CI must not authenticate `gh`, pass
  tokens to tests, or run the optional live end-to-end manual.

Product design: `docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md`.
Phase 1 spec: `docs/superpowers/specs/2026-08-23-phase1-domain-foundation-design.md`.
Phase 2 spec: `docs/superpowers/specs/2026-08-23-phase2-local-repository-vertical-slice-design.md`.
Phase 3 spec: `docs/superpowers/specs/2026-08-24-phase3-remote-composite-action-test-design.md`.
Optional live procedure: `docs/superpowers/manual/phase3-live-e2e.md`.
