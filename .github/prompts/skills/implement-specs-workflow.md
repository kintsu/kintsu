---
name: implement-specs-workflow
title: "Specification Implementation Workflow"
description: "Workflow for implementing RFC-0016, RFC-0017, RFC-0018 specifications in Kintsu parser with context resaturation and verification steps."
tags: ["kintsu", "workflow", "implementation", "parser"]
updated: 2025-12-26
---
# Specification Implementation Workflow

When implementing Union Or, Variant Tagging, or Type Expressions in Kintsu.

## Context Resaturation

Before starting any phase:

```
search_facts with tags: ["kintsu", "<feature>", "parser"]
search_skills for related procedures
get_knowledge_context with feature-specific tags
```

## Key Files

| Component | Path |
|-----------|------|
| Tokens | parser/src/tokens/toks.rs |
| Type AST | parser/src/ast/ty.rs |
| Metadata | parser/src/ast/meta.rs |
| Resolver | parser/src/ctx/resolve/mod.rs |
| Test helpers | parser/src/tst.rs |
| Phase tests | parser/src/ctx/resolve/phase_tests.rs |

## Implementation Plan

Full plan at: implement-specs.md

## Build Verification

```bash
cargo build -p kintsu-parser
cargo test -p kintsu-parser
```

## After Each Phase

```
submit_facts for patterns discovered
submit_execution_logs for successful commands
```
