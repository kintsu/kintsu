---
name: rust-conventions
title: "Rust Coding Conventions"
description: "Idiomatic Rust practices and community standards for the Kintsu project. Covers ownership/borrowing, error handling with Result, API design with common traits, testing patterns, logging with tracing, and project organization."
tags: ["kintsu", "rust", "conventions", "code-quality", "api-design", "error-handling"]
updated: 2025-12-22
---
# Rust Coding Conventions

Idiomatic Rust practices and community standards for the Kintsu project.

## References

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [RFC 430 Naming Conventions](https://github.com/rust-lang/rfcs/blob/master/text/0430-finalizing-naming-conventions.md)

## General Instructions

- Prioritize readability, safety, maintainability
- Use strong typing and ownership system
- Break complex functions into smaller ones
- Handle errors with `Result<T, E>` and meaningful messages
- Follow RFC 430 naming conventions
- Verify with `cargo build -p {packageName}`
- Comments: brief, use `-` not em dashes, prefer `;`

## Patterns to Follow

### Struct Helpers
```rust
// If using foo.value.value.bar frequently:
impl Foo {
    fn bar(&self) -> &Bar {
        &self.value.value.bar
    }
}
```

### General Patterns
- Use `mod` and `pub` to encapsulate logic
- Handle errors with `?`, `match`, or `if let`
- Use `serde` for serialization, `thiserror` for errors
- Implement traits to abstract dependencies
- Structure async with `async/await` and `tokio`
- Prefer enums over flags for type safety
- Use `bon::Builder` for complex object creation
- Split binary (`main.rs`) and library (`lib.rs`) code
- Use `rayon` for data parallelism
- Prefer iterators over index-based loops
- Use `&str` over `String` for parameters when possible
- Prefer borrowing and zero-copy operations

### Ownership and Borrowing
- Prefer `&T` over cloning
- Use `&mut T` for modification
- Annotate lifetimes when compiler can't infer
- Use `Arc<T>` for thread-safe reference counting
- Use `RefCell<T>` (single-threaded) or `Mutex<T>`/`RwLock<T>` (multi-threaded) for interior mutability

## Patterns to Avoid

- `unwrap()`/`expect()` - prefer proper error handling (except in tests)
- `panic!` in library code - always return `Result`
- Global mutable state - use DI or thread-safe containers
- Deeply nested logic - refactor with functions
- `unsafe` without documentation
- Excessive `clone()` - use borrowing
- Premature `collect()` - keep iterators lazy
- Unnecessary allocations

## Error Handling

- `Result<T, E>` for recoverable errors
- `panic!` only for unrecoverable errors
- Prefer `?` over `unwrap()`/`expect()`
- Custom errors with `thiserror`
- `Option<T>` for optional values
- Meaningful error messages

## API Design

### Implement Common Traits
`Copy`, `Clone`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Hash`, `Debug`, `Display`, `Default`

Use: `From`, `AsRef`, `AsMut`, `FromIterator`, `Extend`

### Type Safety
- Newtypes for static distinctions
- Types convey meaning (avoid generic `bool` params)
- Only smart pointers implement `Deref`/`DerefMut`
- Type aliases for complex generic types

### Future Proofing
- Sealed traits to protect against downstream implementations
- Use `validator::Validate` for argument validation
- All public types implement `Debug`
- Accessor helpers instead of public fields
- Getters return references, don't clone

## Testing

- Unit tests in `#[cfg(test)]` modules
- Integration tests in `tests/` directory
- Use `#[test_case::test_case(..)]` for repetitive tests
- Examples use `?`, not `unwrap()`

## Logging

- Use `tracing::trace` where appropriate
- Only `tracing::info` where absolutely required
- Avoid field slop - single line displays
- Use `var.display()` for `ToTokens` types

## Quality Checklist

- [ ] RFC 430 naming conventions
- [ ] Common traits implemented
- [ ] `Result<T, E>` error handling
- [ ] Public items have rustdoc
- [ ] Comprehensive test coverage
- [ ] No `unsafe`, proper error handling
- [ ] No unused/dead code
- [ ] Efficient iterators, minimal allocations
- [ ] Predictable, type-safe APIs
- [ ] Private fields, sealed traits
- [ ] Passes `cargo test`
