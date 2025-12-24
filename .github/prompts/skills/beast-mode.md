---
name: beast-mode
title: "Beast Mode - Autonomous Problem Solving"
description: "Comprehensive workflow for autonomous, thorough problem-solving with Rust focus. Covers information gathering, codebase investigation, planning with todo lists, anti-patterns to avoid, debugging, memory safety for GUI contexts, and threading best practices."
tags: ["kintsu", "rust", "workflow", "autonomous", "debugging", "planning"]
updated: 2025-12-22
---
# Beast Mode - Autonomous Problem Solving

A comprehensive workflow for autonomous, thorough problem-solving with Rust focus.

## Core Principles

- Keep working until the problem is completely solved
- Iterate and verify - never end without solving
- Think thoroughly and plan extensively
- Test rigorously to catch edge cases

## Workflow

### 1. Information Gathering

- **Factsets First**: Query `search_skills`, `search_facts`, `search_resources` before external research
- Fetch all provided URLs recursively
- Research third-party packages and dependencies
- Verify understanding is up-to-date via documentation
- Register fetched URLs with `add_resources` for team knowledge

### 2. Problem Understanding

- Read issues carefully before coding
- Use `dbg!()` macro for exploration
- Check `rustdoc` and documentation tools

### 3. Codebase Investigation

- Explore `mod.rs`, `lib.rs`, and relevant modules
- Search for `fn`, `struct`, `enum`, `trait` items
- Use `cargo tree`, `cargo-expand`, `cargo doc --open`
- Identify root causes

### 4. Research

- **Factsets**: Check existing facts and resources before external search
- Search docs.rs, users.rust-lang.org, Reddit/r/rust
- Use Stack Overflow for common patterns
- Verify library usage is current
- **Capture**: `submit_facts` for new library patterns, `add_resources` for useful URLs

### 5. Planning

Create a todo list:
```markdown
- [ ] Step 1: Description
- [ ] Step 2: Description
- [x] Completed step
```

Check off steps as you complete them. Continue to next step after checking off.

### 6. Anti-Patterns to Avoid

- `.clone()` instead of borrowing - unnecessary allocations
- `.unwrap()`/`.expect()` overuse - fragile error handling
- `.collect()` too early - prevents lazy iteration
- `unsafe` without clear need - bypasses safety
- Over-abstracting with traits/generics
- Global mutable state - breaks testability
- Macros that hide logic - harder to debug
- Ignoring lifetime annotations
- Premature optimization

### 7. Implementation

- Read 1000 lines at a time for context
- Make small, testable, incremental changes
- Reapply patches if not applied correctly

### 8. Debugging

- Use `tracing`, `log`, or `dbg!()` to inspect state
- Find root cause, not symptoms
- Use `RUST_BACKTRACE=1` for stack traces
- Use `cargo-expand` for macro debugging
- Run `cargo fmt`, `cargo check`, `cargo clippy`
- **Factsets**: `submit_execution_logs` for successful debug commands

### 9. Memory Safety (GUI contexts)

- GUI must run on main thread
- Use `glib::Sender` or `glib::idle_add_local()` for thread communication
- Use `Rc`, `Arc`, `Weak` for reference counting
- Avoid circular references
- Use `RefCell`, `Mutex` for shared state

### 10. Threading

- Use `std::thread`, `tokio`, `async-std`, or `rayon` appropriately
- Share state with `Arc<Mutex<T>>` or `Arc<RwLock<T>>`
- Never violate thread-safety guarantees

## Communication

Be clear, concise, friendly yet professional:

- "Fetching documentation for `tokio::select!` to verify usage patterns."
- "Tests passed. Now validating with additional edge cases."
- "Using `thiserror` for ergonomic error handling."

## Factsets Integration

Use Factsets throughout the workflow:

| Phase | Factsets Action |
|-------|----------------|
| Start | `search_skills`, `search_facts`, `search_resources` |
| Research | `add_resources` for URLs, `submit_facts` for learnings |
| Implementation | `submit_execution_logs` for working commands |
| Completion | `create_skill` if reusable procedure, `submit_facts` for patterns |

**Never skip Factsets queries at the start of a task.**
