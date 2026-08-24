# Phase 3 optional live end-to-end manual

This procedure validates remote composite-action testing against real GitHub
runners. It is **not required CI** and must not run from the default GitHub
Actions workflow.

## Prerequisites

1. **Disposable repository** — create a throwaway GitHub repository you can
   delete after this run. Do not use a production or shared repo.
2. **Authenticated `gh`** — run `gh auth login` and confirm with
   `gh auth status`.
3. **Composite action and locked minimal test YAML** — clone the disposable
   repo and add:
   - `action.yml` (composite action at repo root)
   - `.github-workbench/tests/smoke-composite.yml` (declarative test case)

Use the approved minimal fixtures from the Phase 3 plan, or copy them from
`crates/workbench-cli/tests/cli_remote_action.rs`.

4. **`GWW_LIVE_E2E=1`** — export this variable before running remote commands
   so Workbench permits live GitHub traffic on your machine.

## Safety confirmation

Remote tests push a temporary branch and generated workflow file to your
disposable repository. **Existing push workflows in that repository may also
run** when the test branch is pushed. Confirm you accept this before continuing.

## Procedure

From the disposable repository root:

```bash
export GWW_LIVE_E2E=1
export GWW_DATA_DIR="$HOME/.local/share/github-workflow-workbench-live-e2e"

gww open .
gww action discover
gww action test smoke-composite --yes
gww runs list
gww runs watch <session-id>
gww cleanup list
gww cleanup run <item-id> --yes
```

Replace `<session-id>` and `<item-id>` with values printed by the commands above.

## Verify evidence

After `gww runs watch` completes, confirm files exist under
`$GWW_DATA_DIR/evidence/<session-id>/`:

- `github-workbench-result.json` — parsed manifest with expected conclusion
- `run.log` — optional log file used for assertion rules

Inspect the CLI output for pass/fail assertion results.

## Verify moved-ref cleanup protection

To confirm cleanup identity protection without leaving debris:

1. Note the cleanup item id from `gww cleanup list`.
2. On GitHub or with `git push`, move the temporary remote ref (push another
   commit to the same branch name).
3. Run `gww cleanup run <item-id> --yes` again.

Expected: cleanup **refuses deletion** because the remote ref SHA no longer
matches the recorded expected identity. No force push is attempted.

If the ref still matches, allow normal cleanup to delete the temporary ref.

## Teardown

Delete the **disposable repository** on GitHub when finished. Remove local
clones and optional `GWW_DATA_DIR` evidence if no longer needed.

## CI boundary

- Required CI uses fixture `gh` programs and must never authenticate to GitHub.
- This manual is operator-only validation outside automated pipelines.
- Do not add this procedure to `.github/workflows/ci.yml`.
