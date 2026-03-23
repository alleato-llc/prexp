---
name: adding-integration-tests
description: Adds integration tests that exercise services with test doubles or real infrastructure (SQLite). Tests live in the tests/ directory as separate files. Use when testing services, repositories, or any component with external dependencies.
version: 1.0.0
---

# Adding Integration Tests

Integration tests exercise a component with its dependencies — either real infrastructure (SQLite) or test doubles (in-memory implementations).

## Two flavors

| Flavor | Dependencies | Example |
|---|---|---|
| **Service tests** | Test doubles (in-memory) | `expense_service_test.rs` |
| **Repository tests** | Real SQLite (in-memory DB) | `sqlite_repository_test.rs` |

## Service tests (with test doubles)

Test the service layer using in-memory test doubles from `tests/support/`.

```rust
// tests/expense_service_test.rs
mod support;

use rust_cli_reference::expense::service::ExpenseService;
use support::test_repository::TestExpenseRepository;
use support::test_export_client::{ExportRecording, TestExportClient};

fn create_service() -> ExpenseService {
    let recording = ExportRecording::new();
    ExpenseService::new(
        Box::new(TestExpenseRepository::new()),
        Box::new(TestExportClient::new(recording)),
    )
}

#[test]
fn add_expense_persists_and_returns_with_id() {
    let service = create_service();

    let expense = service
        .add_expense(1500, Category::Food, "Lunch".into(), date(2026, 3, 1))
        .unwrap();

    assert!(expense.id.is_some());
    assert_eq!(expense.amount_cents, 1500);
}
```

### Asserting on test double recordings

When you need to assert that a service called a test double correctly, use the `Rc<Recording>` pattern:

```rust
#[test]
fn export_delegates_to_client() {
    let recording = ExportRecording::new();  // Rc<ExportRecording>
    let service = ExpenseService::new(
        Box::new(TestExpenseRepository::new()),
        Box::new(TestExportClient::new(recording.clone())),  // clone the Rc
    );

    service.add_expense(1000, Category::Food, "Coffee".into(), date(2026, 3, 1)).unwrap();
    service.export_expenses("out.csv").unwrap();

    assert_eq!(recording.export_count(), 1);  // assert via the shared Rc
}
```

## Repository tests (real SQLite)

Test the repository against a real SQLite database. Each test gets a fresh in-memory database.

```rust
// tests/sqlite_repository_test.rs
use rust_cli_reference::expense::sqlite_repository::SqliteExpenseRepository;

fn create_repository() -> SqliteExpenseRepository {
    SqliteExpenseRepository::in_memory().unwrap()
}

#[test]
fn save_and_find_expense_by_id() {
    let repo = create_repository();
    let expense = Expense::new(1500, Category::Food, "Lunch".into(), date(2026, 3, 1));

    let saved = repo.save_expense(&expense).unwrap();
    let found = repo.find_expense_by_id(saved.id.unwrap()).unwrap().unwrap();

    assert_eq!(found.amount_cents, 1500);
}
```

## What to assert on

Assert on **observable side effects**, not internal implementation:

1. **Returned values** — the direct result of the operation
2. **Error variants** — `assert!(matches!(result, Err(ExpenseError::NotFound(_))))`
3. **Recording state** — verify external calls happened with correct data via `Rc<Recording>`
4. **Repository state** — fetch and verify persisted values
5. **Negative assertions** — verify side effects did NOT happen on failure paths

### Derive expected values from inputs

Do not hardcode expected values when they can be computed from the test inputs:

```rust
// Bad — magic number
assert_eq!(summary.total_spent_cents, 3000);

// Good — derived from inputs, documents the business rule
let expected_total: i64 = expenses.iter().map(|e| e.amount_cents).sum();
assert_eq!(summary.total_spent_cents, expected_total);
```

## Conventions

- Test file name: `<component>_test.rs`
- `mod support;` at the top to import test doubles
- Helper function `create_service()` / `create_repository()` for setup
- Each test creates its own fresh state — no shared mutable state between tests
- Assert on observable outcomes: returned values, error variants, recording state
- Test both success and failure paths
- Verify that failure paths do NOT trigger downstream side effects
