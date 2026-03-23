---
name: inversion-of-control
description: Traits as contract boundaries for external dependencies. Covers trait design, trait objects for DI, and the production/test implementation pattern. Use when adding external dependencies or defining service boundaries.
version: 1.0.0
---

# Inversion of Control

## Pattern

Every external dependency (database, file I/O, network) sits behind a trait. The service depends on the trait, not the implementation.

```
Service → trait → production impl (SQLite, CSV, HTTP)
                → test impl (in-memory, recording)
```

## Defining a trait

```rust
pub trait ExpenseRepository {
    fn save_expense(&self, expense: &Expense) -> Result<Expense, RepositoryError>;
    fn find_expense_by_id(&self, id: i64) -> Result<Option<Expense>, RepositoryError>;
    fn find_all_expenses(&self) -> Result<Vec<Expense>, RepositoryError>;
    fn delete_expense(&self, id: i64) -> Result<bool, RepositoryError>;
}
```

- Methods take `&self` — enables shared references and trait objects
- Return `Result<T, BoundaryError>` — errors specific to this boundary
- Named after the domain concept (`ExpenseRepository`), not the technology
- **One method per operation** — not `execute(op: &str, params: Map)`

## Injecting via trait objects

```rust
pub struct ExpenseService {
    repository: Box<dyn ExpenseRepository>,
    export_client: Box<dyn ExportClient>,
}

impl ExpenseService {
    pub fn new(
        repository: Box<dyn ExpenseRepository>,
        export_client: Box<dyn ExportClient>,
    ) -> Self {
        Self { repository, export_client }
    }
}
```

- `Box<dyn Trait>` for owned trait objects — simple, explicit DI
- Constructor receives all dependencies — no global state, no service locator
- Production wiring in `main.rs`, test wiring in each test function

## Production wiring

```rust
// src/main.rs
fn main() -> anyhow::Result<()> {
    let repository = SqliteExpenseRepository::new("expenses.db")?;
    let export_client = CsvExportClient;
    let service = ExpenseService::new(Box::new(repository), Box::new(export_client));
    // ...
}
```

## Test wiring

```rust
// tests/expense_service_test.rs
fn create_service() -> ExpenseService {
    let recording = ExportRecording::new();
    ExpenseService::new(
        Box::new(TestExpenseRepository::new()),
        Box::new(TestExportClient::new(recording)),
    )
}
```

## When to use trait objects vs generics

| Approach | When |
|---|---|
| `Box<dyn Trait>` | Default — simple, explicit, matches DI pattern |
| Generics (`T: Trait`) | Performance-critical paths, or when you need `Sized` |

For reference projects and application code, trait objects are preferred for clarity.
