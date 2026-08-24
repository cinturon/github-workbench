# workbench-application

Application use cases and ports for opening a repository, reporting status,
creating an issue branch, planning and executing a push, and listing journaled
operations.

The use cases coordinate domain policy and plans through abstractions for Git,
operation storage, policy loading, time, and ID generation. Process execution
and persistence remain in adapter crates.
