
# Changelog
## Version: 1.0.1
## Last updated: 2026-03-13 – Fixed infinite conflict status UI bug
## Project: GitHelper

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Created `memory.md` and `summary.md` tracking files to initialize the Antigravity system memory rules.
- Fully reorganized `docs/` folder to adhere to the standard Document Registry.

### Fixed
- Fixed a bug where the "Checking Conflict Status..." overlay would remain permanently visible indefinitely if the backend promise hung during a rebase. Added a robust frontend timeout fallback in `CommitGraph.svelte`.
