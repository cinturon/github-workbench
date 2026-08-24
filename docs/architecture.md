# Architecture

GitHub Workflow Workbench uses a layered Rust core:

1. **Domain** (`workbench-domain`) — pure policy, naming, plans, assertions.
2. **Application** (`workbench-application`) — use cases over ports.
3. **Adapters** — `workbench-git`, `workbench-github`, `workbench-storage`.
4. **Presentation** — `workbench-cli` (`gww`); desktop UI deferred.

Phase 1 implements domain logic only. Adapter and CLI crates are stubs.

Policy parsing in Phase 1 does not support partial nested overrides for
`BranchTypeConfig`: when `branches.feature` or `branches.fix` is present, its
`pattern`, `start-from`, and `require-issue` fields are all required.

Product design: `docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md`.
Phase 1 spec: `docs/superpowers/specs/2026-08-23-phase1-domain-foundation-design.md`.
