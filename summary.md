# Project Summary
## Version: 1.0.1
## Last updated: 2026-03-13 – Fixed infinite conflict status UI bug
## Project: GitHelper

GitHelper is a lightweight, high-performance Git GUI application built with Rust and Svelte 5.
Recent changes include establishing the Antigravity Document Registry and fixing a critical UI freeze bug
in `CommitGraph.svelte` where the "Checking Conflict Status..." overlay could remain permanently visible due to backend promise hangs during rebasing and conflict checks.
