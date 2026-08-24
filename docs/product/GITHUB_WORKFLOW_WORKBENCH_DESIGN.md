# GitHub Workflow Workbench

## Product and Technical Design Document

**Status:** Draft proposal  
**Audience:** Maintainers, contributors, designers, and early adopters  
**Initial implementation:** Cross-platform desktop application using Tauri, Rust, React, and TypeScript  
**License target:** Apache-2.0 or MIT/Apache-2.0 dual license  
**Document purpose:** Define a buildable, testable plan for a desktop application that guides developers through a GitHub branching strategy and tests custom GitHub Actions on genuine GitHub-hosted runners.

---

## 1. Executive summary

GitHub Workflow Workbench is a desktop development assistant that connects local Git operations, GitHub collaboration, repository policy, and remote GitHub Actions testing into one guided workflow.

The application helps a developer move work through this lifecycle:

```text
Issue
  -> policy-compliant local branch or worktree
  -> commits and synchronization
  -> remote validation on GitHub-hosted runners
  -> draft pull request
  -> required checks and reviews
  -> policy-compliant merge
  -> local and remote cleanup
```

The product is deliberately not a general-purpose replacement for Git, GitHub Desktop, the GitHub CLI, or the GitHub Actions runner. Its value is coordination and explanation. It understands a repository's chosen workflow, identifies the current state, recommends the next safe action, previews mutations, and connects branch operations to real-runner validation.

The first release should support one opinionated path well:

- GitHub Flow.
- A repository hosted on GitHub.com.
- An existing local clone.
- Authentication through the installed GitHub CLI.
- Branch creation from a GitHub issue.
- Status, commit, push, and draft pull request creation.
- Remote testing of a composite GitHub Action on `ubuntu-latest`.
- Test result and completed-log retrieval.
- Squash merge readiness and explicit cleanup.

The long-term product can support additional branching models, action types, runner matrices, worktrees, releases, organizational policies, and other Git hosting providers without expanding the first milestone prematurely.

---

## 2. Problem statement

### 2.1 Branching workflows are documented but not operationalized

Teams often document branching conventions in a README or wiki:

- Branch from `main` or `develop`.
- Include an issue number in the branch name.
- Open pull requests as drafts.
- Rebase before review.
- Use squash merges.
- Back-merge hotfixes.
- Delete merged branches.

These rules are easy to forget, vary by repository, and are only partially enforceable through GitHub rulesets. Existing Git clients expose operations, but they generally do not model the team's development process as a state machine or explain the next correct step.

### 2.2 Custom GitHub Actions are difficult to test faithfully

Local workflow runners are valuable, but a local container cannot perfectly reproduce every GitHub-hosted environment. Windows and macOS behavior, installed toolchains, filesystem semantics, generated contexts, permissions, and platform-specific action behavior can differ.

Action authors commonly test by committing changes, pushing a branch, manually finding a run, inspecting logs, changing code, and repeating. The mechanics are scattered across Git, GitHub's web interface, workflow YAML, and command-line tools.

### 2.3 Existing tools expose commands rather than a cohesive process

Git and the GitHub CLI already provide excellent primitives. The gap is an application that safely composes them:

- Interpret repository policy.
- Observe local and remote state.
- Produce a proposed operation plan.
- Execute the plan with explicit confirmation.
- Monitor remote work.
- Evaluate results against structured expectations.
- Record enough history to explain what happened.

---

## 3. Product vision

### 3.1 Vision statement

Make a repository's intended development workflow executable, understandable, and testable from one open-source desktop application.

### 3.2 Product promise

> Know what to do next, perform it safely, and validate custom GitHub Actions on real GitHub runners before release.

### 3.3 Product principles

1. **Explain before executing.** Every mutating operation has a human-readable plan.
2. **Use native primitives.** Prefer installed `git` and `gh` behavior over incompatible reimplementations.
3. **Policy is versioned.** Repository workflow configuration lives in a reviewable text file.
4. **Safe by default.** Avoid force pushes, secret persistence, implicit deletion, and hidden remote mutations.
5. **The CLI and desktop app share one core.** Automation and accessibility must not require the graphical interface.
6. **Real runners are authoritative.** Remote tests use GitHub-hosted runners when fidelity matters.
7. **Partial adoption is useful.** A repository may use only branch guidance or only action testing.
8. **Offline behavior degrades gracefully.** Local repository inspection should remain available without GitHub connectivity.
9. **Failures are first-class states.** Cancellation, conflicts, rate limits, and incomplete cleanup are modeled rather than treated as exceptional mysteries.
10. **No hosted service is required for the initial product.** Credentials and repository data remain on the developer's machine and GitHub.

---

## 4. Goals and non-goals

### 4.1 Goals

- Guide developers through a repository-defined branching strategy.
- Connect issues, branches, worktrees, pull requests, checks, and cleanup.
- Detect policy violations before a push or merge attempt.
- Test custom GitHub Actions through generated workflows on real runners.
- Support declarative, version-controlled action test cases.
- Present local and remote status in one coherent task view.
- Provide safe previews for Git and GitHub mutations.
- Build a highly testable Rust domain core.
- Offer both a desktop interface and a headless CLI.
- Provide an extension architecture without requiring plugins in the MVP.

### 4.2 Non-goals for the MVP

- Reimplement Git.
- Reimplement the official GitHub Actions runner.
- Replace the full GitHub website.
- Provide a complete graphical merge-conflict editor.
- Support GitLab, Bitbucket, Azure DevOps, or self-hosted GitHub Enterprise Server.
- Support every branching strategy.
- Execute untrusted pull-request code with repository secrets.
- Manage organization-wide rulesets.
- Provide a hosted synchronization or collaboration backend.
- Automatically rewrite published history.
- Automatically delete branches without a visible plan and user-approved policy.
- Guarantee byte-for-byte equivalence between generated tests and every possible production workflow context.

---

## 5. Target users

### 5.1 Primary persona: custom Action maintainer

Maintains a composite, JavaScript, or Docker GitHub Action. Needs to verify behavior across runners and input combinations before publishing a release. Comfortable with Git but tired of repetitive remote test mechanics.

Needs:

- Repeatable remote test cases.
- Cross-platform matrices.
- Clear output and artifact assertions.
- Safe release preparation.
- Fast navigation from a local change to its remote failure.

### 5.2 Secondary persona: contributor following an unfamiliar workflow

Contributes to multiple repositories with different branching conventions. Understands basic Git but does not remember whether a repository uses `main`, `develop`, release branches, rebases, or merge commits.

Needs:

- A clear next step.
- Correct branch creation.
- Protection from accidental direct work on a protected branch.
- Explanations of policy failures.
- A simple path from issue to pull request.

### 5.3 Secondary persona: project maintainer

Wants contributors to follow a consistent process without writing extensive onboarding instructions or reviewing the same workflow mistakes repeatedly.

Needs:

- Versioned policy.
- Shared action test definitions.
- Merge-readiness summaries.
- Reproducible contributor behavior.
- Low administrative overhead.

---

## 6. Core user journeys

### 6.1 Open and assess a repository

1. User selects an existing local repository.
2. Application verifies that `git` and `gh` are available.
3. Application discovers remotes and identifies the GitHub repository.
4. Application reads `.github-workbench.yml`, if present.
5. Application gathers local Git state.
6. If online, application gathers default branch, issues, pull requests, checks, and applicable repository rules.
7. Application displays the current workflow state and recommended next action.

Success criteria:

- No repository mutation occurs during discovery.
- Missing authentication produces actionable instructions.
- Invalid policy is reported with file locations and does not silently fall back.
- Local status remains visible when GitHub is unavailable.

### 6.2 Start work from an issue

1. User selects an assigned or searchable GitHub issue.
2. Policy engine determines branch type, base, and name.
3. Application fetches remote state.
4. Application previews the proposed commands and resulting branch.
5. User chooses a normal checkout or linked worktree.
6. Application creates the branch.
7. Application records the issue-to-branch association locally.

Example result:

```text
Issue: #42 Add resumable uploads
Base: origin/main
Branch: feature/42-resumable-uploads
Worktree: C:\worktrees\project-42
```

### 6.3 Prepare and push changes

1. Application shows staged and unstaged changes.
2. User selects files and enters a commit message.
3. Policy engine checks commit-message and signing requirements.
4. Application commits through installed Git.
5. Before push, application fetches and computes ahead/behind state.
6. If the base moved, application recommends the configured synchronization method.
7. Application previews destination remote and branch.
8. User confirms push.

### 6.4 Test a custom Action remotely

1. Application discovers `action.yml` or user selects its directory.
2. User selects one or more test cases.
3. Application validates action metadata and test configuration.
4. Application determines whether a build step is required.
5. Application creates a run plan containing runner matrix, permissions, inputs, fixtures, and assertions.
6. Application generates a workflow with a unique session identifier.
7. Application commits or stages generated test infrastructure according to the selected isolation strategy.
8. Application pushes a test ref.
9. GitHub executes the generated workflow.
10. Application monitors workflow, job, and step status.
11. At completion, application downloads logs, result manifests, and selected artifacts.
12. Application evaluates assertions and presents a structured report.
13. User retains the remote ref for investigation or approves cleanup.

### 6.5 Create and finish a pull request

1. Application verifies that the branch is pushed.
2. Application determines the required base branch.
3. It generates a title and body from commits, issue data, and repository templates.
4. It creates a draft pull request by default if configured.
5. Application displays required checks, reviews, conflicts, and policy gates.
6. Once gates pass, user marks the pull request ready.
7. Application offers only allowed merge methods.
8. After merge, application proposes local branch, remote branch, worktree, and temporary-test-ref cleanup.

### 6.6 Recover from a failed operation

Every multi-step mutation is represented as an operation journal. If a push succeeds but run discovery fails, the application records the completed push and offers to resume monitoring. If merge succeeds but branch cleanup fails, it records the merge as complete and cleanup as pending.

The user should never be forced to guess whether an operation partially succeeded.

---

## 7. Functional requirements

### 7.1 Repository discovery

- Open repository by folder picker or recent-project list.
- Resolve repository root using `git rev-parse --show-toplevel`.
- Detect bare repositories and linked worktrees.
- Read current branch, detached HEAD state, upstream, remotes, and worktree status.
- Recognize GitHub HTTPS and SSH remote formats.
- Allow explicit repository mapping when multiple GitHub remotes exist.
- Never assume `origin` is the authoritative remote.

### 7.2 Strategy and policy

- Load policy from `.github-workbench.yml` at the repository root.
- Validate against a versioned schema.
- Support a built-in GitHub Flow preset in the MVP.
- Allow repository-specific overrides.
- Explain every policy decision with the rule that caused it.
- Distinguish warnings from blockers.
- Provide a `policy check` CLI command suitable for CI.
- Make unknown configuration fields an error during early development to catch misspellings.

### 7.3 Git operations

- Fetch without changing the working tree.
- Create and switch branches.
- Create linked worktrees.
- Stage selected paths.
- Commit with existing Git configuration.
- Push and establish upstream.
- Show ahead/behind and merge-base information.
- Preview merge or rebase recommendations.
- Delete local branches only when safely merged unless explicitly overridden.
- Delete remote branches only with confirmation or an enabled cleanup policy.
- Preserve command output with automatic secret redaction.

### 7.4 GitHub operations

- Authenticate through `gh auth status` in the MVP.
- List and search issues.
- Query pull request state.
- Create and update draft pull requests.
- Request reviewers and apply labels when configured.
- Read checks and reviews.
- Mark a pull request ready.
- Enable auto-merge or perform an allowed merge.
- Retrieve repository metadata and available merge methods.
- Open relevant pages in the browser.

### 7.5 Action discovery

- Find `action.yml` and `action.yaml` files without scanning ignored build directories by default.
- Parse action name, description, inputs, outputs, and runtime type.
- Validate referenced scripts or Dockerfiles.
- Support composite actions first.
- Later support Node-based and Docker actions.
- Warn when generated JavaScript bundles appear older than their sources.

### 7.6 Action test definitions

- Store tests in `.github-workbench/tests/*.yml`.
- Support test name, action path, inputs, environment, runner matrix, permissions, timeout, expected conclusion, outputs, artifacts, and log expectations.
- Permit fixture directories inside the repository.
- Reject literal secrets in committed test definitions.
- Allow local mappings from symbolic secret names to GitHub environment or repository secret names.
- Support test tags such as `smoke`, `full`, `release`, and `destructive`.
- Require explicit confirmation for destructive tests.

### 7.7 Remote execution

- Generate deterministic workflow YAML from a normalized test plan.
- Assign every run a globally unique session identifier.
- Use minimal `GITHUB_TOKEN` permissions.
- Support an ephemeral branch strategy first.
- Correlate the pushed commit with the resulting workflow run.
- Monitor queued, in-progress, completed, cancelled, timed-out, and action-required states.
- Allow cancellation.
- Download completed logs and selected artifacts.
- Evaluate assertions locally from a machine-readable result manifest.
- Preserve failed test refs by default for a configurable retention period.
- Never silently consume or expose repository secrets.

### 7.8 Operation planning and journaling

- Separate planning from execution.
- Present commands, API mutations, files generated, refs created, and cleanup behavior.
- Assign every operation a stable local ID.
- Journal each step as pending, running, succeeded, failed, skipped, or compensation-needed.
- Make interrupted operations resumable where possible.
- Provide manual remediation instructions when automatic recovery is unsafe.

---

## 8. Policy configuration design

### 8.1 Example complete configuration

```yaml
schema-version: 1

strategy:
  preset: github-flow
  default-branch: main

branches:
  feature:
    pattern: "feature/{issue}-{slug}"
    start-from: main
    require-issue: true
  fix:
    pattern: "fix/{issue}-{slug}"
    start-from: main
    require-issue: true
  allowed-prefixes:
    - feature
    - fix
    - docs
    - chore

commits:
  require-signing: false
  conventional-commits: warning

pull-requests:
  draft-by-default: true
  required-base: main
  merge-method: squash
  delete-branch-after-merge: prompt
  require-linked-issue: true

synchronization:
  method: rebase
  allow-force-with-lease: prompt
  forbid-force-push-to:
    - main

validation:
  before-push:
    - id: rust-tests
      command: cargo test
  before-ready:
    - id: smoke
      action-tests:
        tags: [smoke]
        runners: [ubuntu-latest]
  before-merge:
    - id: full
      action-tests:
        tags: [full]
        runners: [ubuntu-latest, windows-latest, macos-latest]

remote-testing:
  isolation: ephemeral-branch
  branch-prefix: github-workbench/test
  successful-ref-retention: 0h
  failed-ref-retention: 72h
  max-matrix-jobs: 6
  default-timeout-minutes: 15

cleanup:
  local-branch: prompt
  remote-branch: prompt
  worktree: prompt
```

### 8.2 Configuration precedence

From least to most specific:

1. Built-in defaults.
2. Selected strategy preset.
3. Repository `.github-workbench.yml`.
4. User-local repository settings that are not committed.
5. One-time run overrides.

Security-sensitive repository restrictions cannot be weakened by a one-time override without an explicit warning and confirmation. Future organization policy may define non-overridable constraints.

### 8.3 Policy evaluation result

Every evaluated rule returns structured evidence:

```json
{
  "rule_id": "pull-requests.required-base",
  "severity": "blocker",
  "expected": "main",
  "actual": "develop",
  "message": "This repository requires feature pull requests to target main.",
  "remediation": "Change the pull request base to main."
}
```

The UI renders this structure, and the CLI can emit it as JSON for automation.

---

## 9. Action test specification

### 9.1 Example

```yaml
schema-version: 1
name: uploads-a-directory
description: Uploads every fixture and publishes a result manifest.
tags: [smoke, full]

action:
  path: .

runner:
  os:
    - ubuntu-latest
    - windows-latest
  timeout-minutes: 10

permissions:
  contents: read

inputs:
  source: .github-workbench/fixtures/uploads
  recursive: "true"

environment:
  LOG_LEVEL: debug

expect:
  conclusion: success
  outputs:
    files-uploaded:
      equals: "4"
  files:
    - path: .github-workbench/results/report.json
      exists: true
      snapshot: snapshots/upload-report.json
  logs:
    - contains: "Upload completed"
    - not-contains: "secret="
```

### 9.2 Test normalization

Before workflow generation, the engine converts configuration into a normalized `TestPlan`:

- Resolve defaults.
- Validate runner labels.
- Expand matrices.
- Normalize string inputs because action inputs are strings.
- Validate action input names.
- Verify expected output names are declared.
- Resolve fixture paths relative to repository root.
- Calculate the expected number of jobs.
- Enforce the configured matrix limit.
- Calculate required token permissions.
- Mark any secret-bearing or destructive cases.

### 9.3 Assertion transport

GitHub Actions does not expose every intermediate value directly through the run API. Generated workflows should include a final result-recording step that writes a JSON manifest and uploads it as an artifact.

Example result manifest:

```json
{
  "schema_version": 1,
  "session_id": "01JABC...",
  "case": "uploads-a-directory",
  "runner": "ubuntu-latest",
  "action_outcome": "success",
  "outputs": {
    "files-uploaded": "4"
  },
  "files": [
    {
      "path": ".github-workbench/results/report.json",
      "sha256": "...",
      "size": 284
    }
  ]
}
```

The desktop app downloads this manifest and evaluates assertions locally. Assertion logic therefore remains versioned in the application and is easier to test than generated shell snippets.

### 9.4 Runner-generated files

Paths and shell syntax differ across operating systems. The workflow generator must avoid embedding platform-specific shell where possible. A small, pinned helper executable or JavaScript assertion collector can provide a consistent result-manifest format across runners. Its provenance and checksum must be visible in the generated workflow.

For the MVP, Ubuntu-only composite tests may use a generated Bash collector. Cross-platform support should wait until the helper strategy is designed and tested.

---

## 10. Remote execution strategies

### 10.1 Ephemeral branch in the source repository

The application creates a branch such as:

```text
github-workbench/test/<session-id>
```

It commits the current action code plus a generated workflow and pushes the branch. A push trigger starts the workflow.

Advantages:

- Tests repository-relative behavior accurately.
- Can use repository Actions configuration.
- Simple commit-to-run correlation.

Risks:

- Other push-triggered workflows might run.
- Private repositories consume Actions minutes.
- Generated workflow changes are visible remotely.
- Branch cleanup must be reliable.

Mitigations:

- Analyze existing `push` triggers and warn about collateral workflows.
- Use a reserved branch prefix repositories can explicitly exclude.
- Show the exact commit before push.
- Add a unique concurrency group.
- Use minimal permissions.
- Never use production environments by default.

### 10.2 Dedicated sandbox repository

The application copies the action and fixtures into a repository created or selected for tests.

Advantages:

- Better isolation from production workflows and environments.
- Clearer cleanup and billing boundary.
- Suitable for untrusted experiments.

Limitations:

- Repository-relative behavior may differ.
- Private dependencies and reusable workflows need additional access.
- Copying submodules or large fixtures is complex.

Recommendation:

- Offer this after the ephemeral-source-branch MVP.
- Eventually recommend sandbox mode for new users and higher-risk tests.

### 10.3 Existing harness workflow

A stable workflow on the default branch accepts inputs and tests a selected ref.

Advantages:

- Avoids generating a new workflow for every session.
- Easier to audit permissions.

Limitations:

- Cannot dynamically express arbitrary job matrices and steps through inputs.
- Requires repository setup.
- Changes to the action and harness must be correlated carefully.

Use this as an optimized mode for repositories that adopt the tool permanently.

---

## 11. Domain model

### 11.1 Main entities

#### Repository

- Local root path.
- Git directory or common directory.
- GitHub owner and name.
- Selected remote.
- Default branch.
- Policy configuration.
- Authentication profile reference.

#### WorkItem

- GitHub issue number and node ID.
- Title and state.
- Labels and assignees.
- Associated local branch.
- Associated worktree.
- Associated pull request.

#### BranchState

- Local and remote names.
- HEAD object ID.
- Upstream object ID.
- Base branch and merge base.
- Ahead and behind counts.
- Dirty state.
- Policy compliance findings.

#### PullRequestState

- Number and URL.
- Base and head refs.
- Draft state.
- Review decision.
- Mergeability.
- Check summary.
- Allowed merge methods.
- Linked issues.

#### ActionDefinition

- Path and metadata.
- Runtime type.
- Inputs and outputs.
- Referenced executable resources.
- Validation findings.

#### TestCase

- Source file.
- Tags.
- Inputs and environment.
- Runner definition.
- Permission requirements.
- Assertions.

#### TestSession

- Session ID.
- Repository and commit.
- Generated ref.
- Workflow run ID.
- Expanded test jobs.
- Status and timestamps.
- Logs and artifact locations.
- Cleanup state.

#### Operation

- Operation ID and kind.
- Proposed plan.
- User approval record.
- Individual step states.
- Redacted command output.
- Compensation or cleanup steps.

### 11.2 Workflow state machine

An issue-oriented GitHub Flow task can occupy these states:

```text
Unstarted
  -> BranchCreated
  -> ChangesPresent
  -> Committed
  -> Pushed
  -> PullRequestDraft
  -> ValidationPending
  -> ReviewPending
  -> ReadyToMerge
  -> Merged
  -> CleanupPending
  -> Complete
```

Failure is not a terminal state. Each transition may carry blockers and recovery actions. For example, a failed remote test keeps the task in `ValidationPending` with evidence and a rerun operation.

### 11.3 Operation state machine

```text
Planned -> AwaitingApproval -> Running -> Succeeded
                                  |          |
                                  v          v
                                Failed   CleanupPending
                                  |
                                  v
                             RecoveryRequired
```

Operations must be idempotent where feasible. Re-running discovery is safe. Re-running branch creation should detect an existing matching branch. Re-running PR creation should locate an existing PR instead of creating a duplicate.

---

## 12. System architecture

### 12.1 High-level architecture

```text
React + TypeScript UI
        |
        | typed Tauri commands and events
        v
Rust application layer
  |- repository service
  |- policy service
  |- work-item service
  |- pull-request service
  |- action-test service
  |- operation coordinator
  |- run monitor
        |
        +---------------------+
        |                     |
        v                     v
Git/GitHub adapters       SQLite repository
  |- git process             |- projects
  |- gh process              |- local mappings
  |- GitHub REST API         |- operations
  |- filesystem              |- test sessions
  |- browser opener          |- cached summaries
        |
        v
Local repository and GitHub.com
```

### 12.2 Layering

#### Domain layer

Pure Rust types and rules:

- Policy schema.
- Branch naming.
- Workflow state transitions.
- Merge readiness.
- Test-plan normalization.
- Assertion evaluation.
- Operation planning.

This layer must not call the filesystem, Git, GitHub, SQLite, or Tauri. It should contain most unit and property-based tests.

#### Application layer

Coordinates use cases:

- Open repository.
- Start issue.
- Plan push.
- Create PR.
- Plan remote test.
- Start and monitor test.
- Merge and cleanup.

The application layer depends on traits representing external capabilities.

#### Adapter layer

Implements external capabilities:

- `GitClient` using the `git` executable.
- `GitHubClient` initially using `gh` and later direct REST/GraphQL APIs.
- `ProcessRunner` with structured arguments rather than shell strings.
- `FileStore` for generated workflows and test artifacts.
- `OperationStore` backed by SQLite.
- `Clock` and `IdGenerator` for deterministic tests.

#### Presentation layer

- Tauri command handlers.
- Event streaming from long-running operations.
- React view models.
- CLI command mapping.

Tauri handlers should be thin. Business rules must not live in UI components or command handlers.

### 12.3 Suggested Rust workspace

```text
crates/
  workbench-domain/
    src/
      policy/
      repository/
      workflow/
      testing/
      operations/
  workbench-application/
    src/
      ports.rs
      use_cases/
  workbench-git/
    src/
      process.rs
      parser.rs
  workbench-github/
    src/
      gh.rs
      api.rs
      models.rs
  workbench-storage/
    src/
      sqlite.rs
      migrations/
  workbench-cli/
    src/main.rs
  workbench-desktop/
    src/lib.rs
    src/main.rs
```

The frontend can remain under `apps/desktop-ui/` or the conventional Tauri web source directory.

### 12.4 Candidate Rust dependencies

Dependencies should be confirmed during implementation rather than copied blindly.

- `serde`, `serde_json`, and `serde_yaml` for data formats.
- `thiserror` for library errors.
- `anyhow` only at executable boundaries if desired.
- `tokio` for asynchronous orchestration.
- `reqwest` for future direct GitHub API access.
- `rusqlite` or `sqlx` for local persistence.
- `uuid` or ULID implementation for session identifiers.
- `tracing` and `tracing-subscriber` for structured diagnostics.
- `secrecy` for sensitive in-memory values.
- `proptest` for property-based tests.
- `insta` for reviewed snapshots and golden output.
- `tempfile` for integration test repositories.

Avoid adopting `libgit2` in the MVP unless a concrete need emerges. The installed Git executable better respects users' credential helpers, signing setup, attributes, filters, hooks, and version-specific behavior.

---

## 13. Process execution design

### 13.1 No shell interpolation

Commands must be executed as a program plus an argument vector:

```rust
CommandSpec {
    program: "git",
    args: vec!["push", "origin", "HEAD:refs/heads/feature/42-retry"],
    cwd: repository_root,
    environment: sanitized_environment,
}
```

Do not construct a shell string. This prevents quoting bugs and reduces command-injection risk from branch names, issue titles, and paths.

### 13.2 Command allowlist

The operation planner creates typed commands. The executor accepts only known operation variants rather than arbitrary commands supplied by the UI.

Examples:

- `GitCommand::Fetch`.
- `GitCommand::CreateBranch`.
- `GitCommand::PushRef`.
- `GhCommand::CreatePullRequest`.
- `GhCommand::WatchRun`.

User-configured validation commands are an explicit exception. They must be displayed verbatim and run only after approval.

### 13.3 Output handling

- Capture stdout and stderr separately.
- Decode lossily for display while retaining raw bytes if needed.
- Apply redaction before persistence or UI emission.
- Bound retained output size.
- Attach output to operation steps.
- Preserve exit status and termination cause.
- Support cancellation through child-process termination.

---

## 14. Git safety model

### 14.1 Repository snapshot before mutation

Before a mutating plan begins, record:

- Repository root.
- Current branch or detached HEAD.
- HEAD object ID.
- Index tree ID when obtainable.
- Dirty paths.
- Remotes and selected remote.
- Upstream ref.
- Relevant remote-tracking object IDs.

The snapshot is evidence, not a promise of automatic rollback.

### 14.2 Mutation categories

#### Low risk

- Fetch.
- Create a new local branch.
- Create a new worktree.
- Push a new uniquely named remote branch.
- Create a draft PR.

#### Medium risk

- Stage files.
- Commit.
- Update an existing feature branch.
- Rebase unpublished commits.
- Merge a PR.

#### High risk

- Force push, even with lease.
- Delete unmerged branches.
- Delete remote branches.
- Reset or discard changes.
- Rewrite commits already used by a pull request.

High-risk operations should not be part of the MVP except remote temporary-ref cleanup with exact-target validation and confirmation.

### 14.3 Force-push policy

- Never use unconditional `--force`.
- If later supported, use `--force-with-lease=<ref>:<expected-object-id>`.
- Show the expected remote object ID.
- Block protected/default branches unconditionally.
- Require an explicit repository policy and per-operation confirmation.

### 14.4 Cleanup safety

Before deleting a generated test branch:

- Confirm its fully resolved repository.
- Confirm the branch starts with the configured reserved prefix.
- Confirm its tip matches the session journal.
- Confirm no open pull request uses the branch.
- Show whether the branch contains commits not reachable elsewhere.
- Use explicit ref names, never globs.

---

## 15. GitHub authentication and permissions

### 15.1 MVP authentication

Use the installed `gh` CLI:

- Check `gh auth status`.
- Let `gh` use its configured credential storage.
- Do not retrieve or persist the underlying token.
- Request scope upgrades through normal `gh` flows only when needed.

Benefits:

- Minimal authentication implementation.
- Respects GitHub Enterprise hostname configuration later.
- Avoids initially handling OAuth refresh tokens.

### 15.2 Future authentication

A GitHub OAuth App or GitHub App can provide direct API integration. Prefer short-lived, narrowly scoped tokens. Store refresh credentials in the operating system credential vault, never SQLite or plaintext configuration.

### 15.3 Generated workflow permissions

Generated workflows begin with:

```yaml
permissions:
  contents: read
```

Additional permissions must come from the normalized test plan and be displayed before execution. Tests requiring write permissions receive a prominent warning and should default to sandbox mode.

### 15.4 Fork and untrusted-code policy

- Never make repository secrets available to tests originating from an untrusted fork.
- Treat issue titles, branch names, PR bodies, event payloads, and action output as untrusted input.
- Do not interpolate untrusted values into shell scripts.
- Do not use `pull_request_target` for generated test workflows in the MVP.
- Display action references not pinned to a commit SHA as supply-chain findings.

---

## 16. Local persistence

### 16.1 Data that may be stored

- Recent repository paths.
- Repository-to-GitHub mapping.
- Local issue-to-branch associations.
- Operation journals.
- Test-session metadata.
- Redacted summaries and local artifact paths.
- UI preferences.
- Cleanup schedules.

### 16.2 Data that must not be stored in SQLite

- GitHub tokens.
- Repository secrets.
- Raw environment variables without classification.
- Unredacted command output known to contain secrets.
- Private keys.

### 16.3 Suggested tables

```sql
projects(id, local_path, github_host, owner, repo, remote_name, created_at, updated_at)
work_items(id, project_id, issue_number, branch_name, worktree_path, pr_number, updated_at)
operations(id, project_id, kind, status, plan_json, started_at, completed_at)
operation_steps(id, operation_id, sequence, kind, status, detail_json, output_path)
test_sessions(id, project_id, commit_sha, remote_ref, run_id, status, result_json, created_at)
cleanup_items(id, project_id, resource_kind, resource_id, expected_identity, due_at, status)
```

Use migrations from the first release. Database contents should be disposable; repository policy and tests remain the durable source of truth.

---

## 17. Desktop user experience

### 17.1 Information architecture

#### Home

- Recent repositories.
- Incomplete operations.
- Active remote tests.
- Pending cleanup.

#### Repository overview

- Current work item.
- Branch health.
- Pull request state.
- Policy findings.
- Recommended next action.

#### Changes

- Staged and unstaged file list.
- Diff viewer.
- Commit editor.
- Commit-policy feedback.

#### Action tests

- Detected actions.
- Test-case list and tags.
- Runner selection.
- Plan preview.
- Live status and completed results.

#### Pull request

- Title and body preview.
- Reviewers, labels, checks, and conflicts.
- Merge-readiness explanation.

#### History and recovery

- Operation journal.
- Failed steps.
- Resume, retry, or manual remediation.
- Cleanup queue.

### 17.2 Recommended-action card

The primary screen should avoid overwhelming users with every possible command. It should display:

```text
Recommended next action

Push feature/42-resumable-uploads and run smoke tests.

Why
- The branch has 2 unpublished commits.
- The working tree is clean.
- Repository policy requires Ubuntu smoke tests before a draft PR.

[Review plan] [Run]
```

Advanced operations remain discoverable but secondary.

### 17.3 Plan confirmation dialog

Every plan includes:

- Intended outcome.
- Preconditions.
- Local commands.
- GitHub mutations.
- Files generated.
- Remote refs created or changed.
- Expected Actions jobs and potential minute usage.
- Cleanup behavior.
- Risk classification.

### 17.4 Accessibility

- Full keyboard navigation.
- Semantic labels and predictable focus order.
- Status conveyed by text and icons, not color alone.
- Copyable commands and errors.
- Reduced-motion support.
- Screen-reader announcements for run-state changes.

---

## 18. CLI design

The CLI makes core behavior scriptable and ensures domain logic is not coupled to the desktop UI.

```text
gww open <path>
gww status [--json]
gww policy check [--json]
gww issue start <number> [--worktree]
gww branch sync
gww push --plan
gww pr create [--draft]
gww pr readiness [--json]
gww action discover
gww action test [name] [--tag smoke] [--runner ubuntu-latest]
gww runs list
gww runs watch <session-id>
gww cleanup list
gww cleanup run <item-id>
```

Naming should be revisited before release; `gww` is only a document placeholder.

Exit-code convention:

- `0`: requested operation or check succeeded.
- `1`: operation failed.
- `2`: invalid invocation or configuration.
- `3`: policy blockers found.
- `4`: authentication or authorization required.
- `5`: remote operation still pending when a non-waiting command returns.

---

## 19. Error model

Errors should be typed and actionable:

```rust
enum WorkbenchError {
    InvalidPolicy { findings: Vec<PolicyFinding> },
    GitUnavailable,
    GitHubAuthenticationRequired,
    RepositoryNotMapped,
    DirtyWorkingTree { paths: Vec<PathBuf> },
    RemoteMoved { expected: ObjectId, actual: ObjectId },
    PolicyBlocked { findings: Vec<PolicyFinding> },
    WorkflowNotStarted { correlation_id: String },
    GitHubRateLimited { retry_at: DateTime<Utc> },
    ArtifactUnavailable { run_id: u64 },
    CleanupUnsafe { reason: String },
}
```

Every error shown to the user should contain:

- What failed.
- What state changed before failure.
- What did not happen.
- Whether retry is safe.
- Suggested remediation.
- A diagnostics identifier.

---

## 20. Observability and privacy

This is a developer tool, but it should still diagnose itself well.

### 20.1 Local diagnostics

- Structured logs with operation and session IDs.
- Configurable log level.
- Automatic secret and token redaction.
- Rotating local log files.
- Exportable diagnostics bundle with a manifest of included files.
- Preview and user approval before sharing a diagnostics bundle.

### 20.2 Product telemetry

The open-source initial release should default to no analytics. If opt-in telemetry is later introduced:

- Document every collected field.
- Never collect repository names, file paths, source, logs, issue text, branch names, tokens, or secrets.
- Provide a build-time option to remove telemetry.
- Make self-hosted collection possible if organizational users request it.

---

## 21. Testing strategy

Testing is a primary learning and product goal, not a final hardening step.

### 21.1 Test pyramid

#### Domain unit tests

Test pure decisions exhaustively:

- Branch-name generation.
- Slug normalization.
- Policy precedence.
- State transitions.
- Merge-readiness rules.
- Test normalization.
- Permission calculation.
- Assertion evaluation.
- Cleanup safety predicates.

#### Property-based tests

Useful properties include:

- Generated branch names never contain prohibited ref characters.
- Normalization is idempotent.
- A plan cannot contain cleanup before creation.
- A default/protected branch is never an eligible force-push target.
- Matrix expansion count equals the Cartesian product within limits.
- Redaction never returns a registered secret substring.
- Serialization followed by deserialization preserves normalized policy.

#### Golden and snapshot tests

Review stable output for:

- Generated workflow YAML.
- Operation plans.
- Policy explanations.
- CLI JSON.
- Git command sequences.
- Remote test reports.

Normalize timestamps, IDs, and paths before comparison.

#### Adapter contract tests

Define shared behavioral suites for fake and real adapters:

- Git status parsing.
- GitHub issue and PR mapping.
- Run-state transitions.
- Pagination.
- Rate-limit errors.
- Cancellation.

#### Local Git integration tests

Use temporary repositories with explicit local paths:

- Initialize bare remote and working clone.
- Create divergent histories.
- Exercise branch creation, upstream tracking, and ahead/behind logic.
- Verify worktree behavior.
- Verify cleanup safety.
- Test unusual filenames, Unicode, spaces, and detached HEAD.

Do not depend on the developer's global Git configuration. Provide isolated test config and deterministic identities.

#### GitHub API tests

- Use recorded, sanitized fixtures for most tests.
- Run a bounded live contract suite against a dedicated repository.
- Never run live mutation tests against a contributor's arbitrary repository.
- Give live tests unique identifiers and a cleanup audit.
- Schedule a separate cleanup workflow as defense in depth.

#### End-to-end tests

Critical flows:

1. Start issue, create branch, push, create draft PR.
2. Package composite action, push test ref, observe successful Ubuntu test, retrieve result.
3. Observe failing assertion and retain ref.
4. Cancel an active test.
5. Resume monitoring after application restart.
6. Detect remote ref movement and refuse unsafe cleanup.
7. Merge a qualifying PR and clean its worktree.

#### UI tests

- Repository onboarding.
- Plan review and confirmation.
- Policy error rendering.
- Run progress updates.
- Keyboard navigation.
- Destructive-action safeguards.
- Recovery after restart.

### 21.2 Fault injection

Every external port should support scripted failures:

- Git process exits non-zero.
- GitHub returns 401, 403, 404, 409, 422, 429, or 5xx.
- Network disappears after push.
- Run appears after a discovery delay.
- Artifact expires or is corrupt.
- Application exits between operation steps.
- Cleanup finds a ref with an unexpected object ID.
- User cancels while a child process is active.

### 21.3 Test fixtures

Create small fixture repositories for:

- Valid composite action.
- Missing action metadata.
- Nested composite action.
- JavaScript action with stale `dist` output.
- Multiple actions in a monorepo.
- GitHub Flow repository.
- Diverged feature branch.
- Existing PR and completed check suites.

### 21.4 Coverage philosophy

Coverage percentage is a signal, not the objective. Require high branch coverage in the pure policy and planning crates. Favor behavioral integration coverage around Git and GitHub adapters. Every reported bug should receive the smallest regression test that reproduces it.

---

## 22. Security threat model

### 22.1 Protected assets

- GitHub credentials.
- Repository secrets.
- Source code.
- Local uncommitted changes.
- Git history and remote refs.
- GitHub Actions minutes and runner access.
- Release tags and protected branches.

### 22.2 Trust boundaries

- Desktop UI to Rust core.
- Rust core to child processes.
- Local application to GitHub.
- Repository configuration to operation planner.
- Generated workflow to hosted runner.
- Hosted runner artifacts back to local assertion engine.

### 22.3 Key threats and controls

#### Malicious branch or issue text becomes a command

Control: typed argument vectors; strict ref validation; no shell interpolation.

#### A test exfiltrates secrets

Control: no secrets by default; minimal permissions; sandbox recommendation; explicit secret mapping; untrusted-fork prohibition.

#### Generated test triggers deployment workflows

Control: existing-workflow trigger analysis; reserved branch prefix; visible warning; sandbox mode.

#### Cleanup deletes the wrong ref

Control: explicit repository, full ref, reserved prefix, expected object ID, open-PR check, and confirmation.

#### Logs persist credentials

Control: redaction before storage; bounded logs; diagnostics preview; avoid invoking commands that print tokens.

#### Compromised third-party Action runs with write permissions

Control: surface unpinned references; calculate and display permissions; default `contents: read`; later integrate security scanners rather than duplicating them.

#### UI compromise invokes arbitrary backend commands

Control: narrow Tauri command surface; typed requests; backend validation; no general shell command endpoint.

---

## 23. Performance and reliability

### 23.1 Performance targets

- Open and display local repository status within 500 ms for ordinary repositories after warm start.
- Keep the UI responsive during every Git, GitHub, and database operation.
- Update remote run status within five seconds under normal API availability.
- Avoid scanning ignored directories and large object databases unnecessarily.
- Cache remote summaries with explicit freshness timestamps.

### 23.2 Reliability targets

- No unjournaled multi-step remote mutation.
- No silent branch deletion.
- Survive application restart while a workflow continues remotely.
- Resume monitoring using stored run or correlation identifiers.
- Ensure cancellation is idempotent.
- Make cleanup retryable.

### 23.3 Concurrency model

- One serialized mutating operation per local repository.
- Multiple read-only refreshes may coalesce.
- Multiple remote test sessions may be monitored concurrently.
- UI events carry sequence numbers to avoid stale state overwriting newer state.
- Background refresh uses cancellation tokens when switching repositories.

---

## 24. MVP implementation plan

The combined concept is larger than a polished two-week release. A two-week prototype is feasible if it proves one vertical slice and omits generality.

### Phase 0: technical spikes (1-2 days)

- Verify GitHub authentication through `gh` on target platforms.
- Push a generated workflow to a disposable repository.
- Correlate commit SHA to workflow run.
- Retrieve completed logs and an uploaded JSON result artifact.
- Document GitHub API and workflow-dispatch constraints discovered experimentally.

Exit criterion: one manually prepared composite action is tested remotely from a Rust prototype.

### Phase 1: domain foundation (2-3 days)

- Establish Rust workspace and crate boundaries.
- Implement repository and branch domain types.
- Define policy schema version 1.
- Implement GitHub Flow branch-name and base rules.
- Implement typed operation plans.
- Add unit, property, and golden tests.

Exit criterion: given fixture state, the domain produces correct explanations and command plans without touching a real repository.

### Phase 2: local repository vertical slice (2-3 days)

- Implement Git process adapter.
- Open repository and render status.
- Create feature branch from manually entered issue number/title.
- Push with preview.
- Persist operation journal.
- Add temporary-repository integration tests.

Exit criterion: desktop app safely creates and pushes a policy-compliant branch.

### Phase 3: remote composite-action test (3-4 days)

- Discover one composite action.
- Parse one declarative test case.
- Generate an Ubuntu workflow.
- Push a uniquely named test branch.
- Discover and watch the run.
- Download the result manifest and completed log.
- Display pass/fail.
- Offer safe cleanup.

Exit criterion: a local composite-action change is tested on `ubuntu-latest` end to end.

### Phase 4: draft pull request (1-2 days)

- Query GitHub issue.
- Create draft PR with correct base and issue reference.
- Show check summary.
- Link the remote test session to the work item.

Exit criterion: the prototype demonstrates issue-to-branch-to-test-to-draft-PR.

### Features explicitly deferred from the prototype

- Windows and macOS.
- JavaScript and Docker actions.
- Interactive staging or diff editor.
- Rebase UI.
- Merge and branch deletion.
- OAuth.
- Multiple strategies.
- Organization policy.
- Direct REST client replacing `gh`.

---

## 25. Release roadmap

### Milestone 0.1: proof of workflow

- GitHub Flow only.
- Composite actions on Ubuntu.
- Existing repository and `gh` authentication.
- Issue branch, push, remote test, draft PR.
- Operation plans and journal.

### Milestone 0.2: useful action test lab

- Test suites and tags.
- Output and artifact assertions.
- Windows and macOS runner matrices.
- Cancellation and reruns.
- Sandbox repository mode.
- CLI parity for action testing.

### Milestone 0.3: useful branching assistant

- Worktree management.
- Branch synchronization guidance.
- Review and check status.
- Merge readiness.
- Squash merge and safe cleanup.
- Policy check command for CI.

### Milestone 0.4: action release workbench

- JavaScript build verification.
- Docker action testing.
- Release test profile.
- Tag and major-tag planning.
- Dependency pinning findings.
- Release report.

### Milestone 0.5: strategy expansion

- Trunk-based preset.
- Git Flow preset.
- Release and hotfix workflows.
- Custom state-machine building blocks.
- Policy migration tooling.

### Version 1.0 criteria

- Stable policy and test schemas.
- Safe recovery from all journaled operations.
- Cross-platform desktop distributions.
- Documented security model.
- Reliable Ubuntu, Windows, and macOS action testing.
- CLI usable in CI.
- Accessibility review completed.
- Migration strategy for local database and repository schemas.
- Contributor documentation and governance established.

---

## 26. MVP acceptance criteria

### Repository onboarding

- Given an authenticated user and GitHub repository clone, the app identifies the repository and default branch.
- Given missing `git` or `gh`, the app reports the missing prerequisite without crashing.
- Given invalid configuration, the app reports exact fields and does not mutate the repository.

### Branch workflow

- Given issue 42 titled "Add resumable uploads," the app proposes a valid configured branch name.
- The plan identifies the base commit and remote destination.
- Confirmation creates the branch without losing local changes.
- Push establishes the intended upstream and records the result.

### Remote action test

- The app discovers a valid composite `action.yml`.
- A valid test definition generates deterministic workflow YAML.
- The plan shows permissions, runner, generated ref, and cleanup policy.
- The app pushes a unique test ref and locates the corresponding run.
- Completion produces a structured pass/fail result.
- Failed tests retain their ref by default.
- Cleanup refuses to delete a ref whose tip differs from the journaled object ID.

### Pull request

- The app creates a draft PR from the feature branch to the configured base.
- The PR body links the issue.
- The app displays remote checks and the associated action-test result.

### Recovery

- Restarting the app during a remote run does not lose the session.
- A partial failure shows completed and pending steps.
- Retrying monitoring does not create another workflow run.

---

## 27. Open-source project design

### 27.1 Repository contents

```text
README.md
LICENSE-APACHE
LICENSE-MIT
CONTRIBUTING.md
CODE_OF_CONDUCT.md
SECURITY.md
GOVERNANCE.md
docs/
  architecture.md
  policy-schema.md
  action-test-schema.md
  threat-model.md
crates/
apps/
fixtures/
scripts/
.github/
  workflows/
  ISSUE_TEMPLATE/
```

### 27.2 Contribution boundaries

Good early contribution areas:

- Policy rules.
- Additional branch-name templates.
- Test fixtures.
- Git output parsers.
- Accessibility improvements.
- Documentation.
- New assertion types.

Sensitive areas requiring careful review:

- Command execution.
- Credential handling.
- Workflow generation.
- Ref deletion and history rewriting.
- Secret redaction.
- Automatic updates.

### 27.3 Compatibility policy

- Version repository schemas explicitly.
- Preserve backward compatibility after 1.0.
- Treat generated workflow format as internal until documented otherwise.
- Test supported Git and GitHub CLI version ranges in CI.
- Publish a platform support matrix.

---

## 28. Major risks and mitigations

### Risk: scope becomes a full Git client

Mitigation: prioritize guided lifecycle operations. Link or copy commands for advanced Git behavior instead of implementing everything.

### Risk: scope becomes a full Actions runner

Mitigation: GitHub remains the execution environment. The application only packages, dispatches, observes, and evaluates.

### Risk: GitHub API behavior makes run correlation unreliable

Mitigation: embed a unique session identifier in branch, commit message, workflow name, concurrency group, and workflow inputs where available. Confirm by commit SHA and creation time.

### Risk: temporary pushes trigger unrelated automation

Mitigation: analyze triggers, warn users, support sandbox repositories, document reserved branch exclusions, and provide a stable harness mode.

### Risk: remote tests are slow and costly

Mitigation: show matrix job count, likely billing impact, queue state, smoke/full profiles, cancellation, and local preflight checks.

### Risk: generated workflow changes expose secrets

Mitigation: reject literal secret material, use symbolic mappings, display permissions, and keep secret use out of the MVP.

### Risk: cross-platform path and shell behavior is inconsistent

Mitigation: begin with Ubuntu, add a portable result helper, and maintain real-runner end-to-end fixtures for every supported OS.

### Risk: repository policy conflicts with GitHub rulesets

Mitigation: GitHub enforcement is authoritative. Surface conflicts and use the stricter effective rule; never imply local policy can bypass remote protection.

---

## 29. Decisions recommended now

1. Use Tauri with a Rust core and React/TypeScript UI.
2. Ship a CLI from the same Rust application layer.
3. Use installed `git` and `gh` executables for the MVP.
4. Support GitHub Flow only in the first vertical slice.
5. Support composite actions on Ubuntu first.
6. Use typed operation plans and a persistent journal from the beginning.
7. Keep credentials outside application storage.
8. Keep repository policy and action tests in version-controlled YAML.
9. Use ephemeral branches initially, with a dedicated sandbox mode next.
10. Treat cleanup as an explicit, independently retryable operation.

---

## 30. Questions to resolve through prototypes

1. What is the most reliable method to correlate an ephemeral push with its workflow run across repositories with multiple workflows?
2. Which run and log details are available while a job is active versus only after completion?
3. How should branch-only generated workflows be triggered under current GitHub workflow registration behavior?
4. Can a stable harness workflow cover enough cases to become the preferred long-term execution mode?
5. What is the smallest portable result collector for Ubuntu, Windows, and macOS?
6. How accurately can the application estimate billable usage before dispatch?
7. Which GitHub ruleset details are visible with ordinary read access and which require elevated permissions?
8. Should repository-local settings use `.github-workbench.local.yml`, Git config, or SQLite?
9. How should monorepos associate multiple actions and policies with one Git repository?
10. Which product name is available and clearly communicates both guided Git workflow and Action testing?

These questions should be answered by small executable spikes and documented decisions, not by expanding the MVP spec speculatively.

---

## 31. Definition of project success

The project succeeds initially when an Action maintainer can open a repository, start work from an issue, create the correct branch, test a local composite Action change on a genuine GitHub Ubuntu runner, inspect a structured result, and open the correct draft pull request without manually assembling commands or workflow files.

It succeeds as an open-source tool when repositories can commit a readable workflow policy and test suite, contributors can understand why an operation is or is not allowed, and the application earns trust by making remote mutations explicit, recoverable, and safe.

