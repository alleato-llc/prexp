---
name: error-handling
description: Error handling patterns for Rust applications. Uses thiserror for library error types and anyhow for application-level errors. Defines error propagation strategy across layers. Use when adding error types, handling failures, or reviewing error propagation.
version: 1.0.0
---

# Error Handling

## Two-tier strategy

| Layer | Crate | Pattern |
|---|---|---|
| Library code (`src/lib.rs` tree) | `thiserror` | Typed error enums per domain boundary |
| Application code (`main.rs`, TUI) | `anyhow` | `anyhow::Result` for convenience |

## Error type design

One error enum per domain boundary. Use `#[from]` for automatic conversion between layers.

```rust
// src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum ExpenseError {
    #[error("expense not found: {0}")]
    NotFound(i64),
    #[error("invalid amount: must be positive")]
    InvalidAmount,
    #[error("description cannot be empty")]
    EmptyDescription,
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Export(#[from] ExportError),
}

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

## Propagation rules

- Services return `Result<T, ExpenseError>` — the domain error type
- Repository methods return `Result<T, RepositoryError>` — automatically converts to `ExpenseError` via `?`
- Client methods return `Result<T, ExportError>` — same auto-conversion
- Application code (`main.rs`) uses `anyhow::Result` — any error type converts via `?`

```rust
// In service — ? auto-converts RepositoryError to ExpenseError
pub fn add_expense(&self, ...) -> Result<Expense, ExpenseError> {
    let saved = self.repository.save_expense(&expense)?;  // RepositoryError -> ExpenseError
    Ok(saved)
}

// In main.rs — anyhow wraps everything
fn main() -> anyhow::Result<()> {
    let repo = SqliteExpenseRepository::new("expenses.db")?;  // RepositoryError -> anyhow
    Ok(())
}
```

## Rules

- **No panics in library code** — always return `Result`
- Use `#[error(transparent)]` when wrapping errors from lower layers
- Use `#[error("message: {0}")]` for domain-specific error variants
- One error enum per boundary, not one per module
- Boundary exceptions (e.g., I/O errors from a client) convert automatically via `#[from]` — external error types must not propagate past the service layer as raw types
- Server errors: log details at the application boundary, return generic message (never leak internals)
- Client errors: include the domain error message in the response
- Test assertions use `matches!` for error variants: `assert!(matches!(result, Err(ExpenseError::NotFound(_))))`

## Context enrichment

Use `.context()` / `.with_context()` (from `anyhow`) in **application code** to add human-readable context to errors as they propagate up the call stack. Do not use these in library code — library code should return typed errors that speak for themselves.

```rust
// In main.rs or TUI code — add context for the user
let repo = SqliteExpenseRepository::new(db_path)
    .context("failed to open expense database")?;

// Lazy context when formatting is expensive
let expenses = service.list_expenses()
    .with_context(|| format!("failed to load expenses from {}", db_path))?;
```

## No-panic enforcement

| Code location | Rule |
|---|---|
| Library code (`src/lib.rs` tree) | No `.unwrap()` — return `Result` instead |
| Infallible operations | Use `.expect("reason")` when the compiler can't prove it but the logic guarantees success |
| Tests | `.unwrap()` is fine — panics are test failures |

```rust
// Good — infallible but compiler doesn't know it
let start = NaiveDate::from_ymd_opt(year, month, 1)
    .expect("valid year/month produces a valid first-of-month date");

// Bad — bare unwrap hides the invariant
let start = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
```

## Web framework integration

If adding a web API (axum/actix), implement `IntoResponse` for domain error types — the Rust equivalent of Spring's `@ControllerAdvice`:

```rust
impl IntoResponse for ExpenseError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ExpenseError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ExpenseError::InvalidAmount | ExpenseError::EmptyDescription => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            _ => {
                tracing::error!("internal error: {self:#}");
                (StatusCode::INTERNAL_SERVER_ERROR, "An internal error occurred".into())
            }
        };
        (status, Json(json!({ "message": message, "status": status.as_u16() }))).into_response()
    }
}
```
