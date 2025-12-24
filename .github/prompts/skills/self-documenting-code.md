---
name: self-documenting-code
title: "Self-Documenting Code"
description: "Guidelines for writing code that speaks for itself. Explains when comments are necessary (WHY not WHAT), annotation prefixes (todo, fixme, hack), anti-patterns to avoid, and Rust doc comment best practices."
tags: ["kintsu", "rust", "comments", "documentation", "code-quality"]
updated: 2025-12-22
---
# Self-Documenting Code

Write code that speaks for itself. Comment only when necessary to explain WHY, not WHAT.

## Core Principle

We do not need comments most of the time.

## Avoid These Comment Types

### Obvious Comments
```rust
let counter = 0; // Initialize counter to zero - BAD
counter += 1;    // Increment counter - BAD
```

### Redundant Comments
```rust
fn get_user_name() -> &str {
    self.name // Return the user's name - BAD
}
```

### Outdated Comments
```rust
// Calculate tax at 5% rate - BAD (actually 8%)
let tax = price * 0.08;
```

## Write These Comment Types

### Complex Business Logic
```rust
// Apply progressive tax brackets: 10% up to 10k, 20% above
let tax = calculate_progressive_tax(income, &[0.1, 0.2], &[10000]);
```

### Non-obvious Algorithms
```rust
// Using Floyd-Warshall for all-pairs shortest paths
// because we need distances between all nodes
```

### Regex Patterns
```rust
// Match email format: username@domain.extension
let email_pattern = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$";
```

### API Constraints
```rust
// GitHub API rate limit: 5000 requests/hour for authenticated users
rate_limiter.wait().await;
```

## Decision Framework

Before writing a comment, ask:
1. Is the code self-explanatory? -> No comment needed
2. Would a better name eliminate the need? -> Refactor instead
3. Does this explain WHY, not WHAT? -> Good comment
4. Will this help future maintainers? -> Good comment

## Annotations

Use these prefixes for special comments:
- `todo:` - Work to be done
- `fixme:` - Known bug to fix
- `hack:` - Workaround for external issue
- `note:` - Important implementation note
- `warning:` - Caution about usage
- `perf:` - Performance consideration
- `sec:` - Security consideration

## Anti-Patterns

### Dead Code Comments
```rust
// const old_function = || { ... }; // BAD - delete don't comment
```

### Changelog Comments
```rust
// Modified by John on 2023-01-15 - BAD - use git
```

### Divider Comments
```rust
// ===================================== // BAD
// UTILITY FUNCTIONS
// ===================================== // BAD
```

## Rust Doc Comments

Prefer doc comments (`///`) over inline comments for public APIs:

```rust
/// Adds two optional numbers.
///
/// ## Control Flow
/// - if a and b -> `a + b`
/// - if a -> `a`
/// - if b -> `b`
/// - otherwise, return 0
fn add_if(a: Option<i32>, b: Option<i32>) -> i32 {
    match (a, b) {
        (Some(a), Some(b)) => a + b,
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => 0
    }
}
```

## Style Rules

- Public docs: proper casing, markdown format
- Private comments (`//`, `/* */`): lowercase
- Use markdown in doc comments - Rust converts automatically
- Never use emojis in comments

## Quality Checklist

- [ ] Explains WHY, not WHAT
- [ ] Grammatically correct and clear
- [ ] Will remain accurate as code evolves
- [ ] Adds genuine value
- [ ] Placed above the code it describes
- [ ] Professional language, no emojis

**Remember: The best comment is the one you don't need because the code is self-documenting.**
