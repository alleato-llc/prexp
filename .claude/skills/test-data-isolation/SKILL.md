---
name: test-data-isolation
description: Ensures tests are independent with fresh state per test. Each test creates its own data and infrastructure — no shared mutable state between tests. Use when writing or reviewing tests. Use when writing or reviewing tests.
version: 1.0.0
---

# Test Data Isolation

Every test creates its own fresh state. No test depends on data from another test.

## Principles

### 1. Every test creates its own data

Tests must not read, reference, or depend on data created by other tests. Each test arranges its own inputs and asserts only on the outputs it produces.

```rust
// Bad — depends on data from another test
let expense = repo.find_expense_by_id(1).unwrap();

// Good — creates its own data, then queries it
let saved = repo.save_expense(&expense).unwrap();
let found = repo.find_expense_by_id(saved.id.unwrap()).unwrap();
```

### 2. Use unique identifiers

All IDs and descriptions that a test controls should be unique. This prevents accidental coupling between tests and ensures tests pass regardless of execution order.

```rust
// Bad — hardcoded descriptions risk collision
let expense = Expense::new(1500, Category::Food, "Lunch".into(), date(2026, 3, 1));

// Good — unique per test
let expense = Expense::new(1500, Category::Food, format!("Lunch-{}", Uuid::new_v4()), date(2026, 3, 1));
```

For simple unit tests where the value doesn't matter, reusing test fixture values is fine. Apply unique IDs when tests share infrastructure (real SQLite, shared file system).

### 3. Distinguish domain data from contextual data

**Domain data** is what the test is exercising — the entity being created, modified, or queried. Each test creates its own.

**Contextual data** is the prerequisite state that must exist for the domain operation to succeed. If the system requires contextual entities, create them per test so every test starts with a valid context.

## Strategies by test type

### Service tests — fresh test doubles per test

```rust
fn create_service() -> ExpenseService {
    let recording = ExportRecording::new();
    ExpenseService::new(
        Box::new(TestExpenseRepository::new()),  // Fresh empty repository
        Box::new(TestExportClient::new(recording)),
    )
}

#[test]
fn test_one() {
    let service = create_service();  // Own state
    // ...
}

#[test]
fn test_two() {
    let service = create_service();  // Own state, independent of test_one
    // ...
}
```

### Repository tests — fresh in-memory SQLite per test

```rust
fn create_repository() -> SqliteExpenseRepository {
    SqliteExpenseRepository::in_memory().unwrap()  // Fresh database, migrations auto-run
}

#[test]
fn test_one() {
    let repo = create_repository();  // Own database
    // ...
}
```

### File I/O tests — temp files per test

```rust
use tempfile::NamedTempFile;

#[test]
fn exports_to_csv() {
    let temp = NamedTempFile::new().unwrap();      // Auto-deleted when dropped
    let path = temp.path().to_str().unwrap();

    client.export_expenses(&expenses, path).unwrap();

    let content = fs::read_to_string(path).unwrap();
    // assert on content...
}
```

## Rules

- Each test function creates its own service/repository/client — never share mutable state
- In-memory SQLite gives each test a fresh database
- `tempfile::NamedTempFile` for file I/O tests — auto-cleanup on drop
- Helper functions (`create_service()`, `create_repository()`, `date()`) at the top of test files
- Test data builders for domain objects (`expense()`, `budget()`) — minimal required fields
- Rust's test runner executes tests in parallel by default — shared state would cause races

## What about database cleanup?

In-memory SQLite databases are created fresh per test via `SqliteExpenseRepository::in_memory()`, so cleanup is automatic. For service tests using test doubles, each test creates its own `TestExpenseRepository` instance. Because of this:

- In-memory databases are destroyed when the test function exits
- Unique descriptions/IDs mean no collisions even if databases were shared
- Test doubles start empty — no leftover state to clean up
- Rust's test runner executes tests in parallel — shared state would cause races

If your domain requires stricter isolation (e.g., persistent databases in integration tests), create a fresh `in_memory()` repository per test.

## Checklist

When writing or reviewing tests, verify:

- [ ] Each test creates its own data
- [ ] No test reads or references data created by another test
- [ ] Mutable infrastructure (database, files) is fresh per test
- [ ] Shared-ownership test doubles (Rc<Recording>) are created per test
- [ ] Assertions reference the test's own inputs/outputs — not hardcoded expected values
