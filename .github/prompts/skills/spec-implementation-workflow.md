---
name: spec-implementation-workflow
title: "Specification Implementation Workflow"
description: "Structured workflow for implementing RFC, SPEC, TSY, and ERR specifications in Kintsu. Includes context gathering, scope confirmation, plan generation, and execution phases with Factsets checkpoints."
tags: ["kintsu", "workflow", "specification", "implementation", "planning"]
updated: 2025-12-26
---

# Specification Implementation Workflow

Structured workflow for implementing Kintsu language specifications.

## Trigger

User provides specification IDs (RFC-XXXX, SPEC-XXXX, TSY-XXXX, ERR-XXXX) for implementation.

## Phase 1: Context Gathering

### 1.1 Factsets Validation

```
search_facts with tags derived from spec topic
search_skills for: "read-kintsu-specs", "quantum-thinking", "rust-conventions"
get_knowledge_context with feature-specific tags
get_preference_prompt to capture output preferences
```

### 1.2 Specification Reading

For each spec ID, use `read-kintsu-specs` skill:

1. Read primary spec and extract purpose, syntax, semantics
2. Identify dependencies (other specs referenced)
3. Identify error domains (ERR-XXXX)
4. Build dependency graph; read in topological order

### 1.3 Scope Confirmation

Present to user:

- Specification summary (one line per spec, defer to doc paths)
- Dependency tree
- Estimated phases
- **Wait for user confirmation before proceeding**

## Phase 2: Codebase Analysis

### 2.1 Gap Analysis

Examine existing implementation:

- What infrastructure exists vs. what spec requires
- Integration points with existing code
- Test patterns to extend

Key areas:

- `parser/src/tokens/` - Lexer tokens
- `parser/src/ast/` - AST types
- `parser/src/ctx/resolve/` - Resolver phases
- `errors/src/domains/` - Error types
- `parser/src/tst.rs` - Test helpers

### 2.2 Document Findings

Create context index if not exists: `docs/auto/<feature>-context-index.md`

Format: Token-efficient, defer to spec paths, include:

- Spec overview table with dependencies
- Brief semantic summary per spec
- Error domain mapping
- Current parser state
- Cross-references

## Phase 3: Implementation Plan Generation

### 3.1 Plan Document

Generate `<feature>-implementation.md` at workspace root.

Structure:

| Section             | Content                                                |
| ------------------- | ------------------------------------------------------ |
| User Preferences    | Table from `get_preference_prompt`                     |
| Factsets Tags       | Tags per phase for resaturation                        |
| Reference Documents | Spec paths table                                       |
| Phase N             | File paths, brief description, acceptance criteria ref |
| Verification        | Build and test commands                                |
| Risk Mitigation     | Known risks and mitigations                            |

### 3.2 Principles

- **Token efficient**: Defer to docs by path, never restate spec content
- **Precise**: Specific file paths, acceptance criteria references (AC-N)
- **Resaturable**: Include Factsets tags for each phase
- **Extensible**: Structure supports future additions

## Phase 4: Implementation Execution

### 4.1 Per-Phase Workflow

Before each phase:

```
get_knowledge_context with phase-specific tags
build_skill_context for relevant procedures
```

During phase:

- Follow plan document precisely
- Verify build after each file change: `cargo build -p <package>`

After each phase:

```
submit_facts for patterns discovered
submit_execution_logs for successful commands
cargo test -p <package>
```

### 4.2 Quality Gates

- Build must compile before proceeding to next phase
- Unit tests must pass (or be intentionally skipped with reason)
- Error messages must match ERR spec format and spans

## Phase 5: Completion

### 5.1 Acceptance Verification

For each spec, verify acceptance criteria:

- Read AC-N items from spec
- Confirm each is satisfied
- Document any deviations

### 5.2 Knowledge Capture

```
submit_facts for:
  - Implementation decisions made
  - Test patterns established
  - Error handling patterns
  - Performance observations

create_skill if reusable procedure emerged
update_skill for existing workflows improved
```

## Factsets Tags Reference

| Feature        | Primary Tags                                   |
| -------------- | ---------------------------------------------- |
| Union Or       | `kintsu`, `parser`, `union-or`, `type-system`  |
| Tagging        | `kintsu`, `parser`, `tagging`, `syntax`        |
| Type Expr      | `kintsu`, `parser`, `type-expr`, `type-system` |
| Errors         | `kintsu`, `error-handling`, `<domain-code>`    |
| Infrastructure | `kintsu`, `parser`, `infrastructure`           |
| Integration    | `kintsu`, `integration`                        |

## Anti-Patterns

- Do NOT restate spec content; reference by path
- Do NOT skip Factsets validation at phase boundaries
- Do NOT proceed without user scope confirmation
- Do NOT batch knowledge capture; submit immediately
- Do NOT generate lengthy code blocks in plan; reference patterns
