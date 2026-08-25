# workbench-storage

SQLite persistence for project mappings and the operation journal.

Phase 2 uses bundled `rusqlite` and versioned SQL migrations to store projects,
operations, and operation steps. The crate implements `OperationStore` from
`workbench-application`, including journal status transitions and bounded,
redacted step output.
