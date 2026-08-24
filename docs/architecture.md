# Architecture

GitHub Workflow Workbench uses a layered Rust core:

1. **Domain** (`workbench-domain`) — pure policy, naming, plans, assertions.
2. **Application** (`workbench-application`) — use cases over ports.
3. **Adapters** — `workbench-git`, `workbench-github`, `workbench-storage`.
4. **Presentation** — `workbench-cli` (`gww`); desktop UI deferred.

Phase 2 implements the application use cases, the process-based Git adapter,
the SQLite project and operation journal, and the `gww` CLI. The GitHub adapter
remains a stub; Phase 2 uses manually supplied issue numbers and titles.

The Git adapter executes Git directly with argument vectors, never through a
shell command string. Workbench state defaults to the platform data directory;
`GWW_DATA_DIR` overrides that location for isolated or portable operation.

Policy parsing does not support partial nested overrides for
`BranchTypeConfig`: when `branches.feature` or `branches.fix` is present, its
`pattern`, `start-from`, and `require-issue` fields are all required.

Product design: `docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md`.
Phase 1 spec: `docs/superpowers/specs/2026-08-23-phase1-domain-foundation-design.md`.
Phase 2 spec: `docs/superpowers/specs/2026-08-23-phase2-local-repository-vertical-slice-design.md`.
