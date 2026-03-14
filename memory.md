# Memory Log
## Version: 1.1.0
## Last updated: 2026-03-14 – Implemented Conflicted Files View
## Project: Git Tools

- **2026-03-14**: Implemented "Conflicted Files View" in `CommitGraph.svelte` and `GraphWipPanel.svelte`.
  - Updated `WipSummary` to include `conflictCount`.
  - Improved `summarizeWorkingChanges` to detect and flag unmerged files.
  - Modified WIP row UI to show conflict count with distinct styling.
  - Decoupled `hasActiveConflicts` in `GraphWipPanel` from operation state to ensure conflict resolution is always accessible.
- **2026-03-14**: Fixed infinite "Checking Conflict Status..." overlay bug by adding a 10s `Promise.race` timeout and refactoring state to survive component remounts.
- [2026-03-13] Added a 10-second \Promise.race\ timeout in \CommitGraph.svelte\ (\handlePostRebaseConflictCheck\) to forcefully resolve \GitService.getOperationState\ and set \postRebaseCheckInFlight\ to \alse\ when the backend Tauri bridge or Rust executor hangs indefinitely.
