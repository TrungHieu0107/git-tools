# Feature List

| # | Feature | Description | Location | Status |
|---|---------|-------------|----------|--------|
| 1 | Repository Management | Add, remove, and switch between multiple Git repositories with a tabbed interface. | `RepoManager.svelte`, `TabBar.svelte` | Complete |
| 2 | Commit Graph | Visual representation of git history with lanes, branches, and commit details. | `CommitGraph.svelte`, `graph-layout.ts` | Complete |
| 3 | File Status & Staging | View working directory changes, stage/unstage files or individual lines. | `CommitPanel.svelte`, `CommitFileList.svelte` | Complete |
| 4 | Branch Management | List, create, checkout, delete, and rename local and remote branches. | `BranchExplorer.svelte`, `BranchService.ts` | Complete |
| 5 | Commit & Push/Pull | Commit staged changes with messages and perform sync operations (fetch, pull, push). | `CommitPanel.svelte`, `GitService.ts` | Complete |
| 6 | Conflict Resolution | Interactive tool to resolve git merge/rebase conflicts with side-by-side comparison. | `ConflictEditor.svelte`, `ConflictResolveModal.svelte` | Complete |
| 7 | AI Commit Messages | Generate commit messages using Gemini or OpenRouter based on staged diffs. | `ai_commands.rs`, `CommitService.ts` | Complete |
| 8 | File History & Blame | View the evolution of a specific file and line-by-line authorship. | `FileHistoryPanel.svelte`, `BlameView.svelte` | Complete |
| 9 | Embedded Terminal | Integrated terminal for running arbitrary git or shell commands within the repo. | `TerminalPanel.svelte`, `terminal.rs` | Complete |
| 10 | Rebase & Interactive Rebase | Support for standard and interactive rebasing workflows. | `rebase_commands.rs`, `rebaseStore.ts` | Complete |
| 11 | Stash Management | Save and restore work-in-progress changes (stash, pop, apply, drop). | `CommitPanel.svelte`, `FileService.ts` | Complete |
| 12 | Diff View | Detailed diffing of files with support for large files and custom encodings. | `DiffView.svelte`, `git-diff-parser.ts` | Complete |
