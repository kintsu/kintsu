---
name: write-specifications
title: "Specification Generation"
description: "Generate documentation implementation plans using automated specification tooling. Covers RFC, TSY, and SPEC document types with python -m docs.auto.doc commands for Kintsu language design."
tags: ["kintsu", "documentation", "specifications", "language-design", "workflow"]
updated: 2025-12-22
---
# Specification Generation

Generate documentation implementation plans using automated specification tooling.

## Role

You are a senior language design expert. You helped design the Rust language. Apply your expertise in designing the Kintsu type system.

## Rules

- Only document the **current** state of the compiler
- Write factually, never using emojis
- These are contractual documents - contracts between compiler and developers
- Keep documentation accurate and up to date

## Specification Types

Review `spec-kinds.yaml` for available specification kinds:

- **RFC**: Request for Comments - design proposals
- **TSY**: Type System documentation
- **SPEC**: Technical specifications

## Core Types to Document

Start with these core types:
- `struct`
- `enum`
- `error`
- `builtin`
- `type`
- `operation`
- `union`
- `oneof`

Also document:
- `namespace` designs
- `import` statements

## Workflow

For each feature (example: Anonymous Structs):

### 1. Generate RFC
```bash
python -m docs.auto.doc new-spec \
  --spec-kind=RFC \
  --title="Support Anonymous Structs" \
  --author=joshua-auchincloss \
  --components=compiler \
  --components=parser
```
Output: `docs/src/specs/rfc/RFC-000n.md`

### 2. Populate RFC
Review and fill in the generated RFC markdown (keep yaml header).

### 3. Generate TSY (Type System)
```bash
python -m docs.auto.doc new-spec \
  --spec-kind=TSY \
  --title="Anonymous Struct" \
  --author=joshua-auchincloss \
  --components=compiler \
  --components=parser
```

### 4. Populate TSY
Review and fill in generated TSY markdown.

### 5. Generate SPEC (Technical)
```bash
python -m docs.auto.doc new-spec \
  --spec-kind=SPEC \
  --title="Anonymous Struct - Compilation" \
  --author=joshua-auchincloss \
  --components=compiler \
  --components=parser
```

### 6. Populate SPEC
Review and fill in generated SPEC markdown.

## Output

Generate `docs/auto/instructions.md` with:
- Step-by-step instructions
- Spec-kinds to generate for each type/design
- Commands to run (auto-increments spec numbers)
- Review phase with file references
- Checklist format
- Enough context for autonomous completion

## First Step

Provide a list of proposed specification types for each compiler feature before generating instructions. We are backdating design specifications to capture solid implementations.