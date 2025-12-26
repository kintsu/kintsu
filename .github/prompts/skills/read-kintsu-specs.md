---
name: read-kintsu-specs
title: "Read Kintsu Specifications"
description: "Procedure for reading Kintsu language specifications from local docs or https://docs.kintsu.dev. Covers RFC, SPEC, TSY, ERR document types with extraction patterns and Factsets registration."
tags: ["kintsu", "documentation", "specification", "reading", "workflow"]
updated: 2025-12-26
---

# Read Kintsu Specifications

Procedure for reading and understanding Kintsu language specifications.

## Specification Sources

### Local Documentation

Primary: `docs/src/content/specs/`

| Type | Path Pattern                               | Purpose                |
| ---- | ------------------------------------------ | ---------------------- |
| RFC  | `docs/src/content/specs/rfc/RFC-XXXX.md`   | Design proposals       |
| SPEC | `docs/src/content/specs/spec/SPEC-XXXX.md` | Compilation behavior   |
| TSY  | `docs/src/content/specs/tsy/TSY-XXXX.md`   | Type system rules      |
| ERR  | `docs/src/content/specs/err/ERR-XXXX.md`   | Error definitions      |
| AD   | `docs/src/content/specs/ad/`               | Architecture decisions |

### Remote Documentation

URL: `https://docs.kintsu.dev/specs/<type>/<ID>/`

Use remote when:

- Local docs unavailable
- User requests live version
- Cross-referencing published state

## Reading Workflow

### Step 1: Parse Spec ID

From ID format `<TYPE>-<NNNN>`:

- `RFC` - Request for Comments (design intent)
- `SPEC` - Compiler specification (implementation behavior)
- `TSY` - Type system rules (validation constraints)
- `ERR` - Error domain (error codes and messages)

### Step 2: Read Primary Spec

Use `read_file` for local, `fetch_webpage` for remote.

Extract and note:

- **Title and purpose** (first section)
- **Dependencies** (references to other specs)
- **Acceptance criteria** (AC-N items)
- **Error codes** (K-prefixed codes like KUN2001)

### Step 3: Build Dependency Graph

From spec references:

1. Identify prerequisite specs (must understand first)
2. Identify related error specs (ERR-XXXX)
3. Identify implementation specs (SPEC for RFC)

Read dependencies before primary spec content.

### Step 4: Register Resource

After reading:

```
add_resources:
  uri: docs/src/content/specs/<type>/<ID>.md
  type: file
  tags: ["kintsu", "specification", "<type-lowercase>", "<feature>"]
```

### Step 5: Submit Key Facts

Extract atomic facts immediately:

```
submit_facts:
  content: <one-sentence fact>
  tags: ["kintsu", "<feature>", "specification"]
  sourceType: documentation
```

## Specification Structure Patterns

### RFC Documents

| Section             | Extract                 |
| ------------------- | ----------------------- |
| Abstract            | One-line summary        |
| Motivation          | Problem statement       |
| Design              | Solution approach       |
| Syntax              | Grammar additions       |
| Semantics           | Behavior rules          |
| Error Handling      | Error conditions        |
| Acceptance Criteria | AC-N verification items |

### SPEC Documents

| Section    | Extract                       |
| ---------- | ----------------------------- |
| Scope      | What this specifies           |
| Phases     | Compilation pipeline stages   |
| Validation | Constraint rules              |
| Output     | Generated metadata structures |

### TSY Documents

| Section    | Extract                 |
| ---------- | ----------------------- |
| Rules      | Type constraints        |
| Validation | Per-feature rules       |
| Inference  | Type inference behavior |

### ERR Documents

| Section     | Extract                      |
| ----------- | ---------------------------- |
| Error Codes | K[Domain][Category][Seq]     |
| Messages    | User-facing text templates   |
| Spans       | Source location requirements |
| Help        | Actionable suggestions       |

## Cross-Reference Patterns

Specs reference each other via:

- Explicit dependency metadata
- Inline `[RFC-XXXX]` links
- Error domain references `ERR-XXXX`
- "See also" sections

Always follow cross-references to build complete understanding.

## Output

After reading specs, produce:

- Brief summary (defer to paths)
- Dependency tree
- Error domain mapping
- Implementation priority recommendation
