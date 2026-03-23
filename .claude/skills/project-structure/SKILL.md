---
name: project-structure
description: Domain-oriented module layout for Rust CLI/TUI applications. Lib/bin split separates domain logic from the entry point. Modules named after domain concepts, not technologies. Use when creating a new project, adding a new domain area, or restructuring modules.
version: 1.0.0
---

# Project Structure

## Philosophy

Organize modules to reflect the **domain**, not technical layers. The core domain (what the app is about) gets its own directory with internal structure. Supporting subdomains are sibling modules at the project level — they have clear boundaries and can grow independently without requiring refactors.

## Binary/library split

- `src/lib.rs` — library root, exposes all domain modules. All testable logic lives here.
- `src/main.rs` — binary entry point. Wires real implementations and starts the app. Minimal code.

This split enables `tests/` (integration tests) to import from the library crate.

## Module layout

```
src/
├── lib.rs                    Library root
├── main.rs                   Binary entry point
├── error.rs                  All error types
├── {core_domain}/            Core domain — e.g., expense/
│   ├── mod.rs
│   ├── models.rs             Domain types
│   ├── repository.rs         Trait (persistence boundary)
│   ├── sqlite_repository.rs  Production implementation
│   └── service.rs            Orchestrator
├── {subdomain}/              Supporting subdomain — e.g., budget/
│   ├── mod.rs
│   └── calculator.rs         Pure computation
├── {subdomain}/              External boundary — e.g., export/
│   ├── mod.rs
│   ├── client.rs             Trait (external boundary)
│   └── csv_client.rs         Production implementation
└── ui/                       TUI/CLI layer (if applicable)
    ├── mod.rs
    └── app.rs                App state + render loop

tests/
├── support/                  Test infrastructure
│   ├── mod.rs
│   ├── test_repository.rs    In-memory repository
│   └── test_export_client.rs In-memory client
├── {component}_test.rs       Service tests (with test doubles)
├── {component}_test.rs       Pure logic tests
├── {component}_test.rs       Repository tests (real SQLite)
└── {component}_test.rs       I/O tests (real file system)
```

## Why subdomains are siblings, not nested

Subdomains are sibling modules at the crate level (`src/budget/`), not nested under the core domain (`src/expense/budget/`).

Rationale:
- **No refactor when a subdomain grows** — if `budget/` later gets more complex, its module path doesn't change
- **Clear boundaries** — sibling modules communicate that subdomains are independent capabilities, not internals of the core domain
- **Module visibility works the same** — Rust's module system is based on the file tree; nesting is a communication choice

## Core domain

The core domain is the primary business capability — the reason the app exists. It gets its own directory because it has enough complexity (models, orchestration, persistence) to benefit from internal structure.

Rules:
- **One core domain per application.** If you have two core domains, consider separate crates or a workspace.
- **Models shared across subdomains** live in the core `models.rs`. If a model is only used within a subdomain, it can live in that subdomain's module.

## Supporting subdomains

Supporting subdomains provide capabilities the core domain depends on — budgets, exports, external integrations. They're flat modules (no deep internal structure).

Rules:
- **Name the module after the domain concept**, not the technology: `export/` not `csv/`, `budget/` not `calculator/`
- **Keep it flat.** If a subdomain needs its own `models.rs`, it's a sign the subdomain is complex enough to be its own crate.
- **Traits live in the subdomain module.** The trait is the contract boundary — implementations sit next to it.

## Module size constraint

A module directory should contain **no more than 5–8 files**. When a module reaches this threshold, evaluate whether it actually represents multiple domain concepts that should be split.

### How to evaluate

When a module grows past 5–8 files, ask:

1. **Can the files be grouped by a domain concept?** If you see clusters of files that relate to different responsibilities, they're likely separate subdomains. Extract each cluster into its own module.

2. **Are there files that only talk to each other?** Files that form a self-contained group (trait + implementation + models) are a subdomain boundary waiting to be extracted.

3. **Is there a valid reason to keep them together?** Some modules genuinely need more files — a `models.rs` with many types is fine because they all serve the same purpose. The constraint is a trigger to evaluate, not a hard limit.

## Rules

- Modules named after **domain concepts** (`expense/`, `budget/`, `export/`), not technologies
- Core domain gets its own directory; supporting subdomains are flat modules
- Related models can share a single file (e.g., `models.rs`). Decompose when the file grows beyond ~300 lines.
- **All tests in `tests/`** — never use inline `#[cfg(test)]` modules
- Test doubles live in `tests/support/`, mirroring the trait they implement
- One test file per component being tested

## Adding a new subdomain

1. Create a sibling module: `src/shipping/mod.rs`
2. Define the trait (contract boundary): `src/shipping/client.rs`
3. Add the implementation: `src/shipping/http_client.rs`
4. Re-export from `lib.rs`: `pub mod shipping;`
5. Add the test double in `tests/support/test_shipping_client.rs`
6. Register in `tests/support/mod.rs`

Do not create deep internal structure within the subdomain unless it genuinely needs that complexity.
