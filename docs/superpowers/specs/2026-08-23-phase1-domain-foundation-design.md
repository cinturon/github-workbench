# Phase 1: Domain Foundation Setup

**Status:** Approved for implementation planning  
**Date:** 2026-08-23  
**Product:** GitHub Workflow Workbench  
**Source design:** Product design will be copied into `docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md` (§12, §24 Phase 1, §27).

---

## 1. Goal

Scaffold the open-source Rust workspace and implement the Phase 1 domain foundation so that, given in-memory fixture state, the domain produces correct policy explanations and typed operation plans without touching a real Git repository or GitHub.

## 2. Decisions locked

| Decision | Choice |
|---|---|
| Scope depth | Phase 1 domain foundation (not skeleton-only, not full MVP) |
| License | MIT / Apache-2.0 dual |
| Non-domain surface | Rust workspace only; no Tauri/React in this phase |
| CLI binary name | `gww` (placeholder; rename later) |
| Approach | Domain-complete Phase 1 in one pass; adapter crates are stubs |

## 3. Scope

### In scope

- Initialize the repository with dual-license files and standard open-source docs (`README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`).
- Copy the product design into `docs/product/` and add a short `docs/architecture.md` that points at crate boundaries and this Phase 1 spec.
- Cargo workspace with these crates:

```text
crates/
  workbench-domain/       # real Phase 1 logic + tests
  workbench-application/  # ports traits; use cases empty or compile-only
  workbench-git/          # stub
  workbench-github/       # stub
  workbench-storage/      # stub
  workbench-cli/          # binary: gww; stub main only
```

- Implement pure domain logic for:
  - Policy schema version 1 (load, validate, GitHub Flow preset, precedence).
  - Repository and branch domain types (data only).
  - GitHub Flow branch naming and base rules.
  - Typed operation plans with risk class, preconditions, and human-readable explanations.
  - Workflow state enum and allowed transitions for the issue-oriented GitHub Flow path.
- Unit, property (`proptest`), and golden (`insta`) tests for the domain.
- Minimal CI: `fmt`, `clippy -D warnings`, `test` on Ubuntu.

### Out of scope

- Tauri / React desktop UI.
- Real `git` / `gh` process adapters.
- SQLite persistence.
- Remote Action testing, workflow generation, PR creation/merge.
- Force-push, branch deletion, OAuth, multiple branching strategies.

## 4. Architecture

### Layering

- **Domain (`workbench-domain`):** Pure Rust. No filesystem, Git, GitHub, SQLite, or Tauri.
- **Application (`workbench-application`):** Ports sketched; use cases deferred or stubbed.
- **Adapters (`workbench-git`, `workbench-github`, `workbench-storage`):** Empty stubs with a short README note pointing to later phases.
- **CLI (`workbench-cli`):** Binary name `gww`; version / not-implemented stub only.

### Domain modules

| Module | Responsibility |
|---|---|
| `policy/` | Schema v1; GitHub Flow preset; defaults → preset → repo YAML precedence; findings (severity, expected, actual, remediation). Unknown fields are errors. |
| `repository/` | Repo identity, remotes, branch state, dirty paths, ahead/behind as data types. |
| `workflow/` | Issue→branch naming (`feature/{issue}-{slug}`), base selection, state machine for GitHub Flow. |
| `operations/` | Typed plans (`CreateBranch`, `Fetch`, `PushRef`, …), risk classification, preconditions, explanations, step status. |
| `testing/` | Placeholder types only for Phase 3; no workflow generation. |

### Error model (Phase 1 subset)

Typed errors / findings aligned with product design §19, including at least:

- `InvalidPolicy { findings }`
- `PolicyBlocked { findings }`
- Domain validation errors for empty slugs, prohibited ref characters, and protected-branch misuse for feature work

## 5. Concrete behaviors to implement

1. Parse and validate `.github-workbench.yml` content (as a string / value; no file I/O required in domain).
2. Generate a branch name from issue number + title under GitHub Flow (`feature/{issue}-{slug}`).
3. Reject invalid refs, empty slugs, and feature work targeting a protected/default branch incorrectly.
4. Build a `CreateBranch` plan (and related plan types as needed for golden tests) from fixture state, with explanation.
5. Distinguish policy warnings from blockers in findings.

## 6. Dependencies

**Domain runtime:** `serde`, `serde_yaml`, `thiserror`, `ulid` (operation/plan IDs; golden tests normalize or inject fixed IDs).

**Domain dev:** `proptest`, `insta`, `pretty_assertions`.

**Explicitly not in Phase 1 domain:** `tokio`, `reqwest`, `rusqlite` / `sqlx`, Tauri crates.

## 7. Testing strategy

- **Unit:** Branch naming, slug normalization, policy parse/validate, finding severity, workflow transitions.
- **Property:** Generated branch names never contain prohibited Git ref characters; policy serialize→deserialize round-trip; normalization is idempotent.
- **Golden:** Policy explanation text; operation plan snapshots with IDs/timestamps normalized.

## 8. Stub crate rules

- Application: define port traits that match upcoming adapters (`GitClient`, `GitHubClient`, `OperationStore`, etc.); no use-case implementations beyond what is required to compile.
- Git / GitHub / storage: compile as empty libraries; document “Phase 2+” in each crate README.
- CLI: `gww --version` (or equivalent) works; other commands may print not-implemented and exit with code `2` (invalid / not ready), without calling domain beyond what is trivial.

## 9. Exit criterion

Given fixture policy + issue + repository/branch state in memory, `workbench-domain` produces correct explanations and typed command plans with no real repository I/O. `cargo test` passes for the workspace. CI workflow is present and green for those checks.

## 10. Non-goals reminder

This phase proves the domain and workspace boundaries. It does not ship a usable developer workflow end-to-end. Phase 2 adds the Git adapter and local repository vertical slice.
