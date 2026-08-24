# GitHub Workflow Workbench

Desktop and CLI assistant that guides GitHub Flow and tests custom GitHub Actions on real runners.

**Status:** Phase 3 — remote composite-action testing via CLI and desktop.

Phase 2 provides the `gww` CLI for opening a local repository, inspecting its
workflow state, creating an issue branch, safely planning and executing a push,
and reviewing the operation journal:

```bash
gww open .
gww status
gww issue start 42 --title "Add resumable uploads"
gww push --plan
gww push --yes
gww ops list
```

Phase 3 adds discovery, execution, resume, and cleanup for one composite
GitHub Action per test on `ubuntu-latest`. Remote tests require an authenticated
GitHub CLI (`gh auth login`) and a clean working tree before planning or
executing a new test.

### Minimal action and test locations

- Composite action manifest: `action.yml` or `action.yaml` at the repository
  root (or under a subdirectory discovered by `gww action discover`).
- Declarative test case: `.github-workbench/tests/<name>.yml` (for example
  `.github-workbench/tests/smoke-composite.yml`).

### Phase 3 commands

```bash
gww open .
gww action discover
gww action test smoke-composite --yes
gww runs list
gww runs watch <session-id>
gww cleanup list
gww cleanup run <item-id> --yes
```

- `gww action discover` — find composite actions and test definitions.
- `gww action test <name> [--plan] [--yes]` — plan or run a remote test.
- `gww runs list` — list stored remote test sessions.
- `gww runs watch <session-id>` — resume polling and evaluation for a stored
  session without creating or pushing another branch.
- `gww cleanup list` — list queued remote-ref cleanup items.
- `gww cleanup run <item-id> [--plan] [--yes]` — delete a temporary remote ref
  when its current SHA still matches the recorded expected identity.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Runtime or test failure |
| 2 | Invalid usage or configuration |
| 3 | Policy blocker |
| 4 | Authentication required (`gh auth login`) |
| 5 | Remote run still pending |

### Evidence and state

Set `GWW_DATA_DIR` to override the platform data directory. Workbench stores
the SQLite database at `GWW_DATA_DIR/workbench.db` and downloads remote test
evidence under `GWW_DATA_DIR/evidence/<session-id>/` (manifest
`github-workbench-result.json` and optional `run.log`).

Cleanup deletes only the recorded temporary remote ref after verifying that its
current SHA matches the expected identity recorded at enqueue time. If the ref
has moved, cleanup refuses deletion and records no delete operation.

Set `GWW_GIT_PROGRAM` to override the Git executable used by the process
adapter. Set `GWW_GH_PROGRAM` to override the `gh` executable (fixture tests
use this; CI sets a deliberately invalid default).

## License

Licensed under either of

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT license (`LICENSE-MIT`)

at your option.

## Development

```bash
cargo test --workspace
cargo run -p workbench-cli -- --version
npm ci --prefix crates/workbench-desktop
npm run build --prefix crates/workbench-desktop
npm run tauri --prefix crates/workbench-desktop dev
```

See `docs/architecture.md`, the
[Phase 2 design](docs/superpowers/specs/2026-08-23-phase2-local-repository-vertical-slice-design.md),
the
[Phase 2 implementation plan](docs/superpowers/plans/2026-08-24-phase2-local-repository-vertical-slice.md),
the
[Phase 3 design](docs/superpowers/specs/2026-08-24-phase3-remote-composite-action-test-design.md),
the
[Phase 3 implementation plan](docs/superpowers/plans/2026-08-24-phase3-remote-composite-action-test.md),
and the optional
[Phase 3 live end-to-end manual](docs/superpowers/manual/phase3-live-e2e.md).
