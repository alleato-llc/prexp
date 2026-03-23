---
name: adding-unit-tests
description: Adds unit tests for pure business logic in Rust. Tests live in the tests/ directory as separate files. No infrastructure, no test doubles — just plain structs and function calls. Use when testing algorithms, calculations, validation logic, or any function with no external dependencies.
version: 1.0.0
---

# Adding Unit Tests

Unit tests exercise pure computation — functions with no side effects, no I/O, no dependencies.

## When to use unit tests vs integration tests

| Component | Test type | Why |
|-----------|-----------|-----|
| Calculator / summarizer | Unit test | Pure math, no dependencies |
| Validation logic | Unit test | Pure logic |
| Parser / formatter | Unit test | Data transformation |
| Domain model methods | Unit test | Self-contained behavior |
| Service orchestrating repo + clients | Integration test | Has external dependencies |
| Repository (real SQLite) | Integration test | Requires database |

## Structure

Test files live in `tests/`, named `<component>_test.rs`:

```
tests/
└── budget_calculator_test.rs
```

## Example

```rust
// tests/budget_calculator_test.rs
use chrono::NaiveDate;
use {crate_name}::budget::calculator::BudgetCalculator;
use {crate_name}::expense::models::{Budget, Category, Expense};

fn expense(amount_cents: i64, category: Category, date: NaiveDate) -> Expense {
    Expense::new(amount_cents, category, "test".into(), date)
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

#[test]
fn summarize_groups_expenses_by_category() {
    let expenses = vec![
        expense(1000, Category::Food, date(2026, 3, 1)),
        expense(2000, Category::Food, date(2026, 3, 15)),
        expense(3000, Category::Transport, date(2026, 3, 10)),
    ];

    let summary = BudgetCalculator::summarize(&expenses, &[]);

    let food = summary.iter().find(|s| s.category == Category::Food).unwrap();
    assert_eq!(food.total_spent_cents, 3000);
}
```

## Conventions

- Helper functions at the top of the file for creating test data (`expense()`, `date()`)
- One `#[test]` function per behavior
- Test name describes the behavior: `summarize_groups_expenses_by_category`
- Use `assert_eq!` for values, `assert!(matches!(...))` for enum variants
- Test the **contract** (given inputs -> expected outputs), not the implementation
- Cover: normal cases, edge cases, boundary values, error conditions
- No test doubles needed — if you need them, use the `adding-integration-tests` skill instead
