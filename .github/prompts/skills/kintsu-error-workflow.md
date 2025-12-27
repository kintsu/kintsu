---
name: kintsu-error-workflow
title: "Kintsu Error System Workflow"
description: "Workflow for implementing error codes, using ErrorBuilder, attaching spans/source, and testing error output in Kintsu"
tags: ["kintsu", "error-handling", "workflow", "testing"]
updated: 2025-12-27
---

# Error System Workflow

Workflow for implementing and validating error codes in Kintsu.

## Error Code Format

```
K[Domain][Category][Sequence]
```

- **Domain** (2 letters): LX, PR, NS, TY, TR, UN, MT, TG, TE, PK, RG, FS, IN, WS, CL
- **Category** (1 digit): 0=Syntax, 1=Resolution, 2=Validation, 3=Conflict, 4=Missing, 5=Cycle, 6=Compatibility
- **Sequence** (3 digits): 001-999

## Specification References

| Domain | ERR Spec | Description             |
| ------ | -------- | ----------------------- |
| KLX    | ERR-0002 | Lexical/tokenization    |
| KPR    | ERR-0003 | Parsing/AST             |
| KNS    | ERR-0004 | Namespace               |
| KTY    | ERR-0005 | Type definitions        |
| KTR    | ERR-0006 | Type resolution         |
| KUN    | ERR-0007 | Union operations        |
| KMT    | ERR-0008 | Metadata (version, err) |
| KTG    | ERR-0009 | Variant tagging         |
| KTE    | ERR-0010 | Type expressions        |

## Implementation Pattern

### 1. Error Definition (errors/src/domains/)

```rust
define_domain_errors!(TypeDefError => (TY, Type) for TypeDefErrors {
    #[error("duplicate field name: {name}")]
    #[diag(help = "rename one of the fields")]
    DuplicateField => (TY, Conflict, 3001) {
        name: String
    }
});
```

### 2. Error Builder Usage

```rust
use kintsu_errors::ErrorBuilder;

// With span (required for most errors)
TypeDefError::duplicate_field(field_name)
    .at(field.span())
    .build()
    .with_source_arc_if(&source_path, source_content.as_ref())

// For spanless errors (rare, must be justified)
TypeDefError::some_error(args)
    .unlocated()
    .build()
```

### 3. Source Context Propagation

```rust
// Attach source info at error creation site
fn validate_something(ctx: &NamespaceCtx, source_path: &Path) -> Result<(), Error> {
    let source_content = ctx.sources.get(source_path).cloned();

    // On error:
    Err(SomeError::new()
        .at(span)
        .build()
        .with_source_arc_if(source_path, source_content.as_ref()))
}
```

## Testing Workflow

### 1. Build CLI First

```bash
cargo build -p kintsu-cli
```

### 2. Run Tests

```bash
cargo nextest run -p kintsu-test-suite --no-fail-fast
```

### 3. Review Snapshots

```bash
cargo insta review
```

### 4. Check Specific Test

```bash
cargo nextest run -p kintsu-test-suite kmt3001_version_conflict
```

## Test File Pattern

Tests in `test-suite/tests/cli_k{domain}_tests.rs`:

```rust
#[tokio::test]
async fn kty3003_duplicate_field() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kty3003"),
        "pkg/schema/lib.ks" => "namespace pkg; use types;",
        "pkg/schema/types.ks" => r#"
namespace types;
struct User {
    name: str,
    name: i32  // duplicate
};
"#,
    };

    let result = CliErrorTest::new("kty3003_duplicate_field")
        .expect_error("KTY")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kty3003_duplicate_field", result.stderr);
}
```

## Documentation References

- **Design**: https://docs.kintsu.dev/specs/rfc/RFC-0023
- **Architecture**: https://docs.kintsu.dev/specs/spec/SPEC-0022
- **Local specs**: docs/src/content/specs/err/ERR-{NNNN}.md
