# prexp documentation

The `docs/` folder holds the **shared** material — the common design, the parity
model, and the native-API reference — that both the Rust and Swift ecosystems
honor. Ecosystem-specific docs live under each ecosystem.

## Design

- **[ARCHITECTURE.md](ARCHITECTURE.md)** — the common design: two independent
  implementations (no FFI), the core → front-ends layering, the `ProcessSource`
  seam, and the ecosystem-first monorepo. **Start here.**
- **[MIGRATION.md](MIGRATION.md)** — the decision record + milestone history for
  the modular-monolith refactor and the Swift port. The "why the layout is this way."

## Cross-cutting reference

- **[PARITY.md](PARITY.md)** — how the two implementations are kept honest:
  `scripts/parity.py`, a live cross-implementation diff (and why not a Gherkin spec).
- **[FFI.md](FFI.md)** — the native macOS APIs (libproc / Mach / sysctl / IOKit)
  both sides reimplement, the exact arithmetic, and the gotchas.
- **[TESTING.md](TESTING.md)** — the testing strategy across both ecosystems.

## Process

- **[../CONTRIBUTING.md](../CONTRIBUTING.md)** — how to build, test, and land a
  change across the ecosystems.
- **[../SECURITY.md](../SECURITY.md)** — what prexp can access and how to report
  issues.

## Per-ecosystem docs

The shared docs above cover the common design. For how a specific implementation is
built and structured:

- **Rust** — [../rust/README.md](../rust/README.md) and, for agents,
  [../rust/CLAUDE.md](../rust/CLAUDE.md).
- **Swift** — [../swift/README.md](../swift/README.md) and
  [../swift/CLAUDE.md](../swift/CLAUDE.md).
