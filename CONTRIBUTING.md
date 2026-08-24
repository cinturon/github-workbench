# Contributing

Thank you for your interest in GitHub Workflow Workbench.

## License

By contributing, you agree that your contributions will be licensed under the project's dual license (MIT OR Apache-2.0). See `LICENSE-MIT` and `LICENSE-APACHE`.

## Development workflow

1. Format: `cargo fmt --all`
2. Lint: `cargo clippy --workspace --all-targets -- -D warnings`
3. Test: `cargo test --workspace`

Run all three before opening a pull request.

## Sensitive areas

The product design (`docs/product/GITHUB_WORKFLOW_WORKBENCH_DESIGN.md`) calls out areas that need extra care:

- **Command execution** — Git and shell operations must be previewed, validated, and journaled; never run arbitrary user-supplied commands.
- **Credentials and tokens** — GitHub CLI auth, PATs, and secrets must never be logged, committed, or exposed in error messages.

Review changes touching adapters (`workbench-git`, `workbench-github`) or credential handling with extra scrutiny.

## Questions

Open a discussion or issue for design questions before large changes.
