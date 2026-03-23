---
name: component-design
description: Design rules for services, repositories, clients, and calculators in Rust CLI/TUI applications. Defines responsibility boundaries, composition patterns, method sizing, and when to decompose. Use when creating or reviewing services, repositories, or client implementations.
version: 1.0.0
---

# Component Design

## Shared rules

These apply to all component types.

### Single responsibility first, then size

A struct should be responsible for **one thing**. File size is a secondary signal — only evaluate it after confirming the struct is properly decomposed.

If a file exceeds **300–500 lines**, evaluate whether it's doing too much. Ask:

1. Does this struct have more than one reason to change?
2. Can the methods be grouped into clusters that serve different purposes?
3. Would extracting a cluster into its own struct make both clearer?

If the answer to all three is no — the struct is genuinely one cohesive responsibility that happens to be large — that's fine. The constraint is a trigger to evaluate, not a hard limit.

### Method size

Most methods should naturally land at **20–30 lines** when they're doing one thing well.

**Up to ~50 lines** is acceptable for orchestration methods that coordinate a sequence of steps — each step is a clear block, and extracting them into private functions would just scatter the narrative.

**Over 50 lines** is the trigger to evaluate. Ask:
- Is this method doing more than one thing?
- Are there blocks of code that could be named (extracted into a function) to clarify the flow?
- Is there duplicated logic that could be shared?

### Method composition

Structure methods at **one level of abstraction**. A method should either coordinate high-level steps or implement low-level details — not both.

```rust
// Bad — mixes orchestration with implementation details
pub fn add_expense(&self, amount: i64, category: Category, desc: String, date: NaiveDate) -> Result<Expense, ExpenseError> {
    if amount <= 0 { return Err(ExpenseError::InvalidAmount); }
    if desc.is_empty() { return Err(ExpenseError::EmptyDescription); }
    let expense = Expense::new(amount, category, desc, date);
    let saved = self.repository.save_expense(&expense)?;
    let all = self.repository.find_all_expenses()?;
    // ... 30 more lines of inline computation
    Ok(saved)
}

// Good — orchestration at one level, details pushed down
pub fn add_expense(&self, amount: i64, category: Category, desc: String, date: NaiveDate) -> Result<Expense, ExpenseError> {
    self.validate_expense(amount, &desc)?;
    let expense = Expense::new(amount, category, desc, date);
    let saved = self.repository.save_expense(&expense)?;
    Ok(saved)
}
```

When an orchestration method is long but each step is clear and sequential, it's fine to keep it as one method. Extract when:
- A block of code needs a name to explain what it does
- The same logic appears in multiple places
- The method mixes abstraction levels

### Composition over inheritance

Rust doesn't have inheritance, but the principle still applies: prefer composing structs with collaborators over building trait hierarchies for code reuse.

```rust
// Good — shared logic in a collaborator
pub struct ExpenseService {
    repository: Box<dyn ExpenseRepository>,
    validator: ExpenseValidator,  // shared validation logic
}

// Avoid — trait with default methods as a form of inheritance
pub trait Validatable {
    fn validate(&self) -> Result<(), ValidationError> { ... }
}
```

## Component types

| Component | Responsibility | Dependencies | Error handling |
|---|---|---|---|
| **Service** | Orchestrate workflows, validate input | Trait objects (`Box<dyn Trait>`) | Returns `Result<T, DomainError>` |
| **Repository** | Persistence boundary (trait) | None (trait) / Database (impl) | Returns `Result<T, RepositoryError>` |
| **Client** | External service boundary (trait) | None (trait) / I/O (impl) | Returns `Result<T, ClientError>` |
| **Calculator** | Pure computation | None | Returns plain values or `Result` |
| **App (TUI)** | Render loop, input handling | Service | Uses `anyhow::Result` |

## Service pattern

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

    pub fn add_expense(&self, ...) -> Result<Expense, ExpenseError> {
        // 1. Validate input
        // 2. Create domain object
        // 3. Persist via repository
        // 4. Return result
    }
}
```

- Constructor takes trait objects — production wires real impls, tests wire test doubles
- One public method per use case
- Validates at the boundary, then delegates to infrastructure

## Repository pattern

```rust
// Trait — the contract
pub trait ExpenseRepository {
    fn save_expense(&self, expense: &Expense) -> Result<Expense, RepositoryError>;
    fn find_expense_by_id(&self, id: i64) -> Result<Option<Expense>, RepositoryError>;
    fn find_all_expenses(&self) -> Result<Vec<Expense>, RepositoryError>;
    fn delete_expense(&self, id: i64) -> Result<bool, RepositoryError>;
}

// Production impl — real SQLite
pub struct SqliteExpenseRepository { conn: Connection }

// Test impl — in-memory HashMap/Vec with RefCell
pub struct TestExpenseRepository { expenses: RefCell<Vec<Expense>> }
```

- Trait methods use `&self` (not `&mut self`) — enables shared references
- Production impls use database connections (interior mutability built in)
- Test impls use `RefCell` for interior mutability through `&self`
- **No business logic** — a repository stores and retrieves data, nothing more

## Client pattern

```rust
pub trait ExportClient {
    fn export_expenses(&self, expenses: &[Expense], dest: &str) -> Result<(), ExportError>;
}
```

- **One method per operation** — not `execute(operation: &str, params: &Map)`
- **Method parameters are domain concepts** — not library-specific types
- **Named after the domain concept** (`ExportClient`), not the technology (`CsvWriter`)

## Calculator pattern

```rust
pub struct BudgetCalculator;

impl BudgetCalculator {
    pub fn summarize(expenses: &[Expense], budgets: &[Budget]) -> Vec<CategorySummary> {
        // Pure computation — no side effects, no dependencies
    }
}
```

- No constructor, no state — just associated functions
- Takes data in, returns data out
- Ideal for unit testing with plain structs

## Sizing guidelines

- **Methods**: Most 20–30 lines; orchestration up to ~50
- **Files**: Evaluate at 300–500 lines whether decomposition is needed
- **Modules**: 5–8 files per module directory; exceeding triggers evaluation
