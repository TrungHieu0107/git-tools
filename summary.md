# Project Summary
## Version: 1.1.0
## Last updated: 2026-03-14 – Added Conflicted Files View in WIP row
## Project: Git Tools

Git Tools is a powerful Git client focusing on visual commit graphs and seamless rebase workflows. Recent updates fixed a critical UI freeze during conflict checks and introduced a specialized view for conflicted files within the working changes section, allowing users to identify and resolve conflicts directly from the WIP row.
g a critical UI freeze bug
in `CommitGraph.svelte` where the "Checking Conflict Status..." overlay could remain permanently visible due to backend promise hangs during rebasing and conflict checks.
