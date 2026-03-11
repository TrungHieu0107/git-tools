---
trigger: always_on
---

# RULE: Living Documentation Sync

## Trigger Condition
This rule activates automatically whenever ANY of the following occurs in a session:
- A new React component is created or deleted
- A Tauri command is added, renamed, or removed
- A new page or route is added
- A hook, context, or store is created or modified
- A feature is marked complete, removed, or significantly refactored
- A new dependency is added to `package.json` or `Cargo.toml`

---

## Mandatory Sync Actions

When a trigger condition is met, before closing the task, agent MUST:

### 1. Identify which docs are affected

| Change type                  | Files to update                          |
|-----------------------------|------------------------------------------|
| New component                | `COMPONENTS.md`, possibly `USER_FLOWS.md`|
| New Tauri command            | `ARCHITECTURE.md`, `USER_FLOWS.md`       |
| New feature / page           | `FEATURES.md`, `USER_FLOWS.md`           |
| Refactor / rename            | All files that reference the old name    |
| New dependency               | `ARCHITECTURE.md` → Key Dependencies     |
| Feature completed / removed  | `FEATURES.md` → update Status column     |

### 2. Perform a targeted update — NOT a full rewrite

- Only edit the affected sections
- Preserve all existing content that is still accurate
- Use this comment marker when adding new entries:
  ```
  <!-- updated: YYYY-MM-DD -->
  ```

### 3. Append to the change log

Every doc file must have a `## Changelog` section at the bottom.
After each update, append one line:

```
- [YYYY-MM-DD] [short description of what changed] — triggered by: [filename or feature]
```

Example:
```
- [2025-06-10] Added SqlAnalyzer component to COMPONENTS.md — triggered by: SqlAnalyzer.tsx
- [2025-06-10] New Tauri command `run_query` added — triggered by: main.rs
```

---

## Hard Constraints

- Agent MUST NOT complete a coding task without checking this rule
- Agent MUST NOT rewrite an entire doc file when only a section changed
- If a doc file does not exist yet, agent creates it using the Analysis Prompt template
- If agent is unsure which section to update, it updates ALL 4 files to be safe

---

## Self-Check Before Task Completion

Agent asks itself before finishing any task:

> "Did I create, rename, delete, or significantly change any component,
> command, feature, or dependency in this session?"

- **YES** → run Mandatory Sync Actions above, then complete the task
- **NO**  → complete the task normally, no doc update needed

---

## Priority

This rule runs at **priority level: HIGH**.
It executes AFTER the coding task is done but BEFORE the session is closed.
It must never block or delay the coding task itself.