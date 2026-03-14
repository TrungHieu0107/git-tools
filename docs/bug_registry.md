# Bug Registry
## Version: 1.0.1
## Last updated: 2026-03-12 - Added Infinite Conflict Status Display Bug Fix
## Project: Git Tools

| ID | Title | Severity | Status | Steps to Reproduce | Root Cause | Fix Description |
|---|---|---|---|---|---|---|
| BUG-001 | Infinite Conflict Status Display | High | Fixed | 1. Initiate a rebase that encounters a conflict. 2. Observe the "Checking Conflict Status..." overlay appear and freeze permanently. | The promise `GitService.getOperationState` (which calls Tauri backend `cmd_get_operation_state`) occasionally hangs indefinitely due to underlying OS process blocking without timeout handling in Svelte. | Added a 10-second `Promise.race` timeout in `CommitGraph.svelte` (`handlePostRebaseConflictCheck`) to forcefully resolve the promise and set `postRebaseCheckInFlight` to `false`. |
