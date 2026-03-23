---
name: testing-boundaries
description: Creates trait-based test implementations that honor the contract at each external boundary. Covers RefCell for interior mutability and Rc for shared assertions. Use when adding a new trait boundary or testing service interactions.
version: 1.0.0
---

# Testing Boundaries

Every external dependency sits behind a contract boundary — a trait that defines what the dependency does, not how it does it (see `inversion-of-control`). This skill covers how to create test implementations that honor the contract, and how to verify your code uses the contract correctly.

## Shared principles

### 1. Fakes over mocks

Hand-written trait implementations with in-memory state. No mock libraries — the test double is a real struct with real behavior.

### 2. Contract fidelity

A test implementation must behave like the real implementation at the contract level:
- **Return realistic results** — auto-incrementing IDs that mimic database behavior
- **Respect error semantics** — return `Err(RepositoryError::...)` for failure cases, not `panic!`
- **Honor the trait contract** — `find_expense_by_id` returns `None` for unknown IDs, not an empty vec

### 3. Call capture

Record invocations so tests can assert on what was called. Use `RefCell<Vec<...>>` for interior mutability through `&self` trait methods. Use `Rc<Recording>` when the test double is consumed by a service but the test needs to inspect recordings afterward.

### 4. Configurable errors

Test doubles support error injection to test failure paths:

```rust
pub struct TestExpenseRepository {
    error_to_return: RefCell<Option<RepositoryError>>,
    // ...
}
```

### 5. Reset between tests

Each test function creates its own fresh test double — no shared mutable state. Rust's test runner executes tests in parallel, so shared state would cause races.

## Location

```
tests/support/
├── mod.rs
├── test_repository.rs      (implements ExpenseRepository)
└── test_export_client.rs    (implements ExportClient)
```

One file per trait. Register in `tests/support/mod.rs`.

## Basic pattern — RefCell for interior mutability

Trait methods take `&self`, but test doubles need to mutate internal state. Use `RefCell` for interior mutability:

```rust
use std::cell::RefCell;

pub struct TestExpenseRepository {
    expenses: RefCell<Vec<Expense>>,
    next_id: RefCell<i64>,
}

impl TestExpenseRepository {
    pub fn new() -> Self {
        Self {
            expenses: RefCell::new(Vec::new()),
            next_id: RefCell::new(1),
        }
    }
}

impl ExpenseRepository for TestExpenseRepository {
    fn save_expense(&self, expense: &Expense) -> Result<Expense, RepositoryError> {
        let mut id = self.next_id.borrow_mut();
        let saved = Expense { id: Some(*id), ..expense.clone() };
        *id += 1;
        self.expenses.borrow_mut().push(saved.clone());
        Ok(saved)
    }

    fn find_all_expenses(&self) -> Result<Vec<Expense>, RepositoryError> {
        Ok(self.expenses.borrow().clone())
    }
    // ... other methods
}
```

**Key decisions:**
- `RefCell` for interior mutability — trait methods take `&self`, but we need to mutate
- Auto-incrementing IDs — mimics database behavior (contract fidelity)
- Clone-based storage — simple, no lifetime complexity

## Shared recording pattern — Rc for assertion after service call

When you pass a test double to a service (via `Box<dyn Trait>`) but need to assert on its state afterward, use `Rc` to share ownership:

```rust
use std::rc::Rc;
use std::cell::RefCell;

pub struct ExportRecording {
    exports: RefCell<Vec<ExportRecord>>,
}

impl ExportRecording {
    pub fn new() -> Rc<Self> {  // Returns Rc, not Self
        Rc::new(Self { exports: RefCell::new(Vec::new()) })
    }

    pub fn export_count(&self) -> usize { ... }
    pub fn last_export(&self) -> Option<ExportRecord> { ... }
}

pub struct TestExportClient {
    recording: Rc<ExportRecording>,
}

impl TestExportClient {
    pub fn new(recording: Rc<ExportRecording>) -> Self {
        Self { recording }
    }
}

impl ExportClient for TestExportClient {
    fn export_expenses(&self, expenses: &[Expense], destination: &str) -> Result<(), ExportError> {
        self.recording.exports.borrow_mut().push(ExportRecord { ... });
        Ok(())
    }
}
```

Usage in tests:

```rust
#[test]
fn export_delegates_to_client() {
    let recording = ExportRecording::new();         // Rc<ExportRecording>
    let client = TestExportClient::new(recording.clone());  // clone Rc
    let service = ExpenseService::new(Box::new(repo), Box::new(client));

    service.export_expenses("out.csv").unwrap();

    assert_eq!(recording.export_count(), 1);        // assert via shared Rc
}
```

## When to use which pattern

| Pattern | When |
|---|---|
| **Simple RefCell** | Test double is queried directly (e.g., repository — service returns its data) |
| **Rc + Recording** | Test double is consumed by service, but test needs to inspect what happened |

## Conventions

- Test doubles live in `tests/support/`, one file per trait
- Register in `tests/support/mod.rs`
- Always implement the full trait — no partial implementations
- Use `RefCell` for interior mutability, `Rc` for shared ownership
- No `unsafe` code in test doubles
- Auto-incrementing IDs mimic database behavior
- Each test function creates its own fresh test double

## Checklist

When creating or reviewing test doubles, verify:

- [ ] Test double implements the full trait — no `unimplemented!()` stubs
- [ ] Interior mutability via `RefCell`, not `unsafe` or `Mutex`
- [ ] `Rc<Recording>` pattern used when test needs to assert on consumed double
- [ ] Auto-incrementing IDs for repository fakes (mimics database behavior)
- [ ] Error injection supported for failure path testing
- [ ] Each test creates its own fresh double — no shared state between tests
