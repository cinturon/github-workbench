# GitHub Workflow Workbench

Desktop and CLI assistant that guides GitHub Flow and tests custom GitHub Actions on real runners.

**Status:** Phase 2 — local repository CLI vertical slice.

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

Set `GWW_DATA_DIR` to override the directory containing the Workbench SQLite
database. Set `GWW_GIT_PROGRAM` to override the Git executable used by the
process adapter.

## License

Licensed under either of

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT license (`LICENSE-MIT`)

at your option.

## Development

```bash
cargo test --workspace
cargo run -p workbench-cli -- --version
```

See `docs/architecture.md`, the
[Phase 2 design](docs/superpowers/specs/2026-08-23-phase2-local-repository-vertical-slice-design.md),
and the
[Phase 2 implementation plan](docs/superpowers/plans/2026-08-24-phase2-local-repository-vertical-slice.md).
