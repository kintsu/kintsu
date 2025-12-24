---
name: code-review
title: "Code Review Guidelines"
description: "Senior-level code review process for clean, maintainable Rust code. Covers structure review, API stability, testing, function analysis, Rust conventions compliance, and specific rules for parser context cleanup."
tags: ["kintsu", "rust", "code-review", "code-quality", "workflow"]
updated: 2025-12-22
---
# Code Review Guidelines

Senior-level code review process for clean, maintainable Rust code.

## Role

You're a senior expert software engineer with extensive experience in planning and maintaining projects, ensuring clean code and best practices. You have a keen eye for detail and ability to refine code.

## Pre-Review Setup

1. Take a deep breath - feel zen and focused
2. Review all coding guidelines in `.github/instructions/*.md` and `.github/copilot-instructions.md`
3. Review code carefully

## Review Checklist

### Structure
- [ ] Final code is clean and maintainable
- [ ] Files > 300 lines: suggest new file structure
- [ ] Preserve all code in restructuring
- [ ] `kintsu-parser` only exports ast nodes, tokenize, and compilation functions

### API Stability
- [ ] Don't change public/private APIs unless removing dead code
- [ ] Update downstream uses in local packages when changing APIs

### Testing
- [ ] Tests still pass after changes

### Functions
- [ ] Analyze each function one-by-one
- [ ] Propose candidate list for unused functions to remove

### Documentation
- [ ] Review per self-documenting-code skill
- [ ] Clean code is self-documenting
- [ ] Keep documentation brief

### Rust Conventions
- [ ] All code adheres to rust-conventions skill principles

## Specific Rules

### Parser Context (`parser/src/ctx`)
- Be particularly critical
- Want clean, concise code
- No unused structs allowed

### Deep Field Access
If code uses deeply nested fields like `foo.value.value.bar`:
- Add helper function in original `Foo` definition
- Create `foo.bar()` that returns a reference
- Find original struct and implement accessor there

## Agent Mode

Apply beast-mode and quantum-thinking workflows for thorough analysis.

## Quality Standards

- No dead code
- No unused parameters or functions
- Efficient use of iterators
- Minimal allocations
- Type-safe APIs
- Private fields in structs
- Sealed traits where appropriate
- Code passes `cargo test`
