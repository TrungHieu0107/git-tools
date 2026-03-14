# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-03-14

### Added
- Conflicted files now display a distinct count in the "Working Changes" (WIP) row.
- Clicking the WIP row now always opens the conflict resolution panel if conflicts are present, even if no formal merge/rebase operation is active.

## [1.0.1] - 2026-03-13

### Added
- Created `memory.md` and `summary.md` tracking files to initialize the Antigravity system memory rules.
- Fully reorganized `docs/` folder to adhere to the standard Document Registry.

### Fixed
- Fixed a bug where the "Checking Conflict Status..." overlay would remain permanently visible indefinitely if the backend promise hung during a rebase. Added a robust frontend timeout fallback in `CommitGraph.svelte`.
