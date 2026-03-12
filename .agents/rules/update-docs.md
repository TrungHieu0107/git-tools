---
trigger: always_on
---

# Antigravity Rules (Core System Prompt)

---

## 1. Memory System – MANDATORY

Every single conversation must follow this exact process:

- Whenever the user requests to **edit, add, delete, or update any feature** (even the smallest detail):
  1. Immediately save the **full details** (original request, reason, old → new version, date & time) into `memory.md`.
  2. Immediately update `summary.md` with a concise 1-3 sentence summary of the current project status and all latest changes.

- `memory.md` + `summary.md` are the **ONLY official memory system** for Antigravity.
- Do NOT use any other memory mechanism (chat history, vector DB, etc.) unless explicitly requested and already logged in `memory.md`.
- After every save, always reply with:
  > **"Changes saved to memory.md and summary.md has been updated ✓"**

---

## 2. Document Generation System – MANDATORY

When starting to implement any part of the project (code, feature, bug fix, refactor, etc.), you **must** generate and maintain all relevant documents from the **Document Registry** below.

Every document file must start with this standard header:
```markdown
# [Document Title]
## Version: x.x.x
## Last updated: YYYY-MM-DD – [short summary of changes]
## Project: [Project Name]
```

---

## 3. Document Registry

The following is the **full set of technical documents** required for long-term, stable software development.
Generate only the documents **relevant to the current task**, but **never skip a document that applies**.

### 📁 Core Documents (Always Required)

| File | Purpose |
|---|---|
| `docs.md` | Full documentation: usage instructions, APIs, configuration, examples |
| `spec.md` | Technical specification: requirements, architecture, tech stack, edge cases, constraints |
| `user_flow.md` | User flow with step-by-step description and Mermaid diagrams |
| `summary.md` | Current project status summary (1–5 sentences, always up to date) |
| `memory.md` | Full change log: every edit, reason, old → new, timestamp |

---

### 📁 Architecture Documents

| File | Purpose |
|---|---|
| `architecture.md` | System architecture overview: components, layers, data flow, deployment topology. Include Mermaid diagrams. |
| `adr/ADR-XXXX-[title].md` | **Architecture Decision Records** – one file per major decision. Captures: Context, Decision, Consequences, Alternatives Considered. |
| `data_model.md` | Entity definitions, relationships, DB schema or data structures, field descriptions. |
| `api.md` | Full API reference: endpoints/methods, request/response schemas, auth, error codes, examples (REST, gRPC, or internal API). |
| `integration.md` | All external integrations: 3rd-party services, webhooks, SDKs, auth flows, rate limits, failure handling. |

---

### 📁 Development & Quality Documents

| File | Purpose |
|---|---|
| `README.md` | Project overview, prerequisites, quick start, environment setup, run/build commands. |
| `setup.md` | Detailed environment setup: tools, versions, environment variables, local config. |
| `test_plan.md` | Test strategy: unit/integration/e2e scope, tools, coverage targets, test data, known edge cases. |
| `bug_registry.md` | Bug log: ID, title, severity, status, steps to reproduce, root cause, fix description. |
| `refactor_log.md` | Refactoring history: what changed, SOLID principle applied, before/after summary. |

---

### 📁 Operations & Deployment Documents

| File | Purpose |
|---|---|
| `deployment.md` | Deployment guide: environments (dev/staging/prod), CI/CD pipeline, steps, rollback procedure, required secrets. |
| `runbook.md` | Operational runbook: common issues, diagnostic steps, recovery procedures, on-call checklists. |
| `config.md` | All configuration options: environment variables, flags, defaults, validation rules, per-environment notes. |
| `security.md` | Security considerations: auth/authz model, data protection, known risks, dependency audit notes. |

---

### 📁 Project Management Documents

| File | Purpose |
|---|---|
| `changelog.md` | User-facing version history following [Keep a Changelog](https://keepachangelog.com) format: Added / Changed / Deprecated / Removed / Fixed / Security. |
| `roadmap.md` | Feature roadmap: planned work, priorities, milestones, deferred items. |
| `decisions.md` | Lightweight log of product/business decisions (not architecture-level). |
| `glossary.md` | Project-specific terms, abbreviations, domain vocabulary. |

---

## 4. Document Update Rules

- **On every new feature:** Update `docs.md`, `spec.md`, `user_flow.md`, `changelog.md`, `summary.md`, `memory.md`.
- **On every architecture decision:** Create a new `adr/ADR-XXXX-[title].md`.
- **On every bug fix:** Update `bug_registry.md`, `changelog.md`, `memory.md`.
- **On every refactor:** Update `refactor_log.md`, `spec.md`, `memory.md`.
- **On every deployment change:** Update `deployment.md`, `config.md`, `runbook.md`.
- **On every API change:** Update `api.md`, `changelog.md`.
- **Version numbering:** Use **Semantic Versioning** – `MAJOR.MINOR.PATCH`.
  - PATCH: bug fix, minor tweak
  - MINOR: new feature, non-breaking
  - MAJOR: breaking change or architectural overhaul

---

## 5. Mermaid Diagram Standards

Always use Mermaid diagrams where applicable:

- `user_flow.md` → `flowchart TD` or `sequenceDiagram`
- `architecture.md` → `C4Context`, `graph LR`, or `block-beta`
- `data_model.md` → `erDiagram`
- `api.md` → `sequenceDiagram`
- `deployment.md` → `graph LR` or `flowchart LR`

---

## 6. Antigravity Execution Checklist

Before closing any task, verify:

- [ ] `memory.md` updated with full change details
- [ ] `summary.md` updated (reflects current state)
- [ ] All relevant documents from the Document Registry updated
- [ ] Mermaid diagrams added or updated where applicable
- [ ] Version numbers incremented in affected documents
- [ ] `changelog.md` updated if user-facing behavior changed
- [ ] ADR created if an architectural decision was made

> **"Changes saved to memory.md and summary.md has been updated ✓"**