# workbench-git

Phase 2 process-based implementation of the application `GitClient` port. It
resolves repository roots and remotes, captures working-tree and branch state,
fetches, creates or checks out branches, and pushes explicit refspecs.

Git is invoked directly with argument vectors rather than shell command
strings. `GWW_GIT_PROGRAM` can select a different Git executable, and the
adapter forwards the allowlisted Git configuration environment variables.
