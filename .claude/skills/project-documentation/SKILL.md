---
name: project-documentation
description: Defines the required documentation structure for every project. Covers root-level files (README, CONTRIBUTING, LICENSE, SECURITY, CLAUDE.md) and docs/ directory (architecture, testing, database, migrations, release, security design, how-to, and per-feature docs).
version: 1.0.0
---

# Project Documentation

Every project must include a standard set of documentation. Root-level files serve public/contributor-facing purposes. The `docs/` directory contains detailed internal documentation.

## Required structure

```
README.md                    High-level overview, quickstart
CONTRIBUTING.md              Developer workflow, branching, PR process, commit conventions
LICENSE.md                   License
SECURITY.md                  Vulnerability reporting policy (GitHub convention)
CLAUDE.md                    Agent context, references docs/

docs/
├── ARCHITECTURE.md          System architecture, component design, domain workflows
├── TESTING.md               Testing strategy, how to run, what to test where
├── DATABASE.md              Schema, data models, relationships
├── MIGRATIONS.md            Migration strategy, how to add/rollback
├── RELEASE.md               Release process, versioning, CI/CD
├── SECURITY.md              App security design (auth, API security, encryption)
├── HOW_TO.md                Detailed usage, setup, configuration, gotchas, troubleshooting
├── TUI.md                   TUI architecture (TUI apps only)
└── feature/
    └── FEATURE_N.md         Per-feature deep dives
```

## Root-level files

### README.md
- Project name and one-sentence description
- Prerequisites
- Quickstart (build, run, test)
- High-level architecture overview
- Project structure (directory tree)
- Link to `docs/` for detailed documentation

### CONTRIBUTING.md
- How to set up the development environment
- Branching strategy and PR process
- Commit conventions (conventional commits if applicable)
- How to add new features/skills
- Project conventions (naming, structure, testing)

### LICENSE.md
- Full license text
- Choose appropriate license for the project

### SECURITY.md (root)
- How to report security vulnerabilities
- Supported versions
- Response timeline expectations
- This is the **public-facing** security policy (GitHub surfaces it in the Security tab)

### CLAUDE.md
- Agent context for Claude Code
- Project overview, build commands, architecture summary
- References to `docs/` for detailed documentation
- Lists available skills

## docs/ files

### ARCHITECTURE.md
- System overview diagram or description
- Component responsibilities and interactions
- Domain model overview
- Key design decisions and rationale
- Domain workflows (how data flows through the system)
- Technology choices and why

### TESTING.md
- Testing strategy (unit vs integration, when to use which)
- How to run tests
- Test infrastructure (databases, test doubles)
- Test conventions (naming, location, assertions)
- Coverage expectations
- CI test pipeline

### DATABASE.md
- Schema overview (tables, columns, types, constraints)
- Entity relationships
- Data model diagrams or descriptions
- Index strategy
- Key queries and their purpose

### MIGRATIONS.md
- Migration tool and configuration (embedded SQL, diesel, refinery, etc.)
- How to create a new migration
- Migration naming conventions
- How to rollback
- Migration testing strategy
- Common pitfalls

### RELEASE.md
- Release process (manual or automated)
- Versioning strategy (semver, conventional commits)
- CI/CD pipeline description
- How to trigger a release
- How to verify a release
- Troubleshooting failed releases

### SECURITY.md (docs/)
- Authentication strategy
- Authorization model
- API security (rate limiting, input validation, CORS)
- Data encryption (at rest, in transit)
- Secret management
- Dependency security scanning
- For CLI/TUI apps: filesystem access, privilege considerations
- This is the **internal** security design doc, distinct from root SECURITY.md

### HOW_TO.md
- Detailed setup instructions (step-by-step)
- Configuration options and environment variables
- Common operations and workflows
- Gotchas and known quirks
- Troubleshooting guide (symptoms -> causes -> solutions)
- FAQ

### TUI.md (TUI/GUI apps only)
- TUI framework and architecture pattern (e.g., ratatui + immediate mode)
- Application state management (what state exists, who owns it)
- Input handling (key bindings, event loop)
- Screen/view architecture (how views compose, how to add a new view)
- Widget design (reusable components)
- Render loop lifecycle
- How to extend (add a screen, add a keybinding, add a widget)
- Design decisions (why this framework, tradeoffs)
- Limitations and known issues

### Feature docs (docs/feature/FEATURE_N.md)

Each feature gets its own document. Use a descriptive filename (e.g., `EXPENSE_MANAGEMENT.md`, not `FEATURE_1.md`).

Required sections:

```markdown
# Feature Name

## What
Brief description of the feature and its business purpose.

## How

### User flow
Step-by-step from the user's perspective (CLI command, TUI interaction, etc.).

### Data flow
How data moves through the system (input -> service -> repository -> output).

## Architecture

### Design decisions
Key choices made and why.

### Core models
Domain types involved in this feature.

### Core types
Service layer, repository, client traits relevant to this feature.

### File organization
Which files implement this feature and where they live.

## Configuration
Feature flags, environment variables, configuration files.

## Dependencies
What other features or services this feature depends on.

## Testing
How to test this feature — which test files, what scenarios are covered,
how to add new test cases.

## Maintenance
Operational concerns — common failure modes, data management.

## Limitations
Known limitations, edge cases, future improvements.
```

## When to create/update docs

| Event | Action |
|---|---|
| New project | Create all root files and docs/ structure |
| New feature | Add `docs/feature/FEATURE_NAME.md` |
| Schema change | Update `DATABASE.md` and `MIGRATIONS.md` |
| Architecture change | Update `ARCHITECTURE.md` |
| New test pattern | Update `TESTING.md` |
| TUI change (new screen/widget) | Update `TUI.md` |
| Release process change | Update `RELEASE.md` |
| Security change | Update `docs/SECURITY.md` |

## Principles

- **Docs describe what exists** — never document aspirational features
- **Keep docs close to code** — update docs in the same PR as the code change
- **CLAUDE.md is the entry point** — it should reference docs/ for details, not duplicate them
- **Feature docs are the most valuable** — they answer "how does X work?" which is the most common question
- **README is for newcomers** — keep it focused on getting started, not deep architecture
