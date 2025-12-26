//! CLI tests for KUN (Union) errors per ERR-0007.
//!
//! Union errors occur when there are issues with union type operations,
//! field merging, and union-or composition.
//! All KUN errors require source spans per SPEC-0022.

use kintsu_fs::memory;
use kintsu_test_suite::cli_tests::{CliErrorTest, minimal_manifest};

/// KUN2001: Union operand must be struct type (found enum)
#[tokio::test]
async fn kun2001_union_operand_not_struct_enum() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kun2001-enum"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    id: u64,
    name: str
};

enum Status {
    Active = 1,
    Inactive = 2
};

type Combined = User & Status;
"#,
    };

    let result = CliErrorTest::new("kun2001_union_operand_not_struct_enum")
        .name("Union Operand Not Struct (Enum)")
        .purpose("Verify KUN2001 when union operand is an enum instead of struct")
        .expect_error("KUN")
        .requires_span(true) // Per ERR-0007: span required on the non-struct operand
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kun2001_union_operand_not_struct_enum", result.stderr);
}

/// KUN2001: Union operand must be struct type (found oneof)
#[tokio::test]
async fn kun2001_union_operand_not_struct_oneof() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kun2001-oneof"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct Base {
    id: u64
};

type Variant = oneof str | i32;

type Combined = Base & Variant;
"#,
    };

    let result = CliErrorTest::new("kun2001_union_operand_not_struct_oneof")
        .name("Union Operand Not Struct (OneOf)")
        .purpose("Verify KUN2001 when union operand is a oneof instead of struct")
        .expect_error("KUN")
        .requires_span(true) // Per ERR-0007: span required on the non-struct operand
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kun2001_union_operand_not_struct_oneof", result.stderr);
}

/// KUN3001: Union field conflict (different types)
/// Per ERR-0007: Severity=Warning, requires span on conflicting field
#[tokio::test]
async fn kun3001_union_field_conflict() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kun3001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct Base {
    version: str,
    name: str
};

struct Extended {
    version: i32,
    count: u64
};

type Combined = Base & Extended;
"#,
    };

    // Per ERR-0007: KUN3001 is a Warning with required span
    let result = CliErrorTest::new("kun3001_union_field_conflict")
        .name("Union Field Conflict")
        .purpose("Verify KUN3001 warning for field appearing with different types")
        .expect_error("KUN3001") // Warning code per ERR-0007
        .requires_span(true) // Per ERR-0007: span required on conflicting field
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kun3001_union_field_conflict", result.stderr);
}

/// KUN8001: Union field shadowed
/// Per ERR-0007: Severity=Warning, requires span on shadowed field
#[tokio::test]
async fn kun8001_union_field_shadowed() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kun8001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    id: u64,
    name: str
};

struct Permissions {
    id: str,
    role: str
};

type Admin = User & Permissions;
"#,
    };

    // Per ERR-0007: KUN8001 is a Warning with required span
    let result = CliErrorTest::new("kun8001_union_field_shadowed")
        .name("Union Field Shadowed")
        .purpose("Verify KUN8001 warning for field shadowed by earlier operand")
        .expect_error("KUN8001") // Warning code per ERR-0007
        .requires_span(true) // Per ERR-0007: span required on shadowed field
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kun8001_union_field_shadowed", result.stderr);
}
