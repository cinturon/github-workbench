# Task 12 Report: Scaffold the Thin Tauri Action Tests Desktop

## Status
Complete. The workspace now includes a Tauri 2 desktop member with thin list/start/watch/result commands and a focused React Action Tests interface.

## TDD evidence
1. **RED** — Added the desktop command contract test first; `cargo test -p workbench-desktop` failed with `E0432` because `workbench_desktop::commands` did not exist.
2. **GREEN** — Implemented the discovery delegate and the four Tauri command adapters; the contract test passed.
3. **VERIFY** — TypeScript 7 initially rejected the CSS side-effect import without Vite client declarations. Added `vite-env.d.ts`, then the frontend and full Tauri release builds passed.

## Changes
- Added the `workbench-desktop` workspace member, Tauri configuration/capability, app icon, and Linux-compatible Tauri 2 dependencies.
- Added list, confirmed start, non-waiting watch, and stored-result commands. Git, SQLite, and `gh` work runs through `spawn_blocking` and delegates to existing application use cases.
- Converted `RemotePending` into a serializable watch response and surfaced persisted assertion-failure results without duplicating evaluation rules.
- Added a typed Tauri API plus repository entry, discovery warnings, test selection, plan confirmation, run status/result evidence, and copyable CLI cleanup guidance.
- Kept Home, Changes, pull-request, and cleanup-queue screens out of scope.

## Test results
```text
cargo test -p workbench-desktop
1 integration test passed; 0 failed

npm ci --prefix crates/workbench-desktop
PASS; 0 vulnerabilities

npm run build --prefix crates/workbench-desktop
PASS; TypeScript and Vite production build completed

cargo fmt --check -p workbench-desktop
cargo clippy -p workbench-desktop --all-targets -- -D warnings
PASS

npm run tauri -- build
PASS; release binary: target/release/workbench-desktop

cargo test --workspace
PASS; 0 failed
```

## Commits
- `7fdf01c` feat(desktop): add Action Tests Tauri shell
- `04d430c` fix(desktop): declare Vite client types

## Concerns
None. The required WebKitGTK 4.1/GTK development libraries were installed, and the full Tauri release build succeeds in this environment.

## Important review fixes
- Commit `d3c74b9` changes confirmation to send the serialized `RemoteTestSessionPlan` returned by preview back to Rust. The confirmed command validates its repository and passes that exact plan to `execute_remote_test`; it does not call `plan_remote_test` again.
- `execute_remote_test` now accepts the application-level wait mode. Desktop starts with `wait == false`, returns `result: null` on `RemotePending`, and the React UI polls stored results plus `watch_action_test` until terminal. CLI behavior remains blocking with `wait == true`.
- Added `non_waiting_execution_pushes_exact_reviewed_plan_and_returns_pending`, which verifies the pushed session stores the exact reviewed plan and remains available for live watch.

## Review-fix verification
```text
cargo test -p workbench-desktop
PASS; 1 integration test passed, 0 failed

npm run build --prefix crates/workbench-desktop
PASS; TypeScript and Vite production build completed

cargo clippy -p workbench-desktop --all-targets -- -D warnings
PASS

cargo test -p workbench-application --test remote_test_execute
PASS; 3 tests passed, including exact-plan pending execution regression

cargo check -p workbench-cli
PASS
```
