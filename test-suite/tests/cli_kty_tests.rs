//! CLI tests for KTY (Type Definition) errors per ERR-0005.
//!
//! Type definition errors occur when there are issues with type declarations,
//! such as duplicate types, duplicate fields, or circular dependencies.
//! All KTY errors require source spans per SPEC-0022.

use kintsu_fs::memory;
use kintsu_test_suite::cli_tests::{CliErrorTest, minimal_manifest};

/// KTY3001: Duplicate type identifier
#[tokio::test]
async fn kty3001_duplicate_type_ident() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kty3001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    name: str
};

struct User {
    id: u64
};
"#,
    };

    let result = CliErrorTest::new("kty3001_duplicate_type_ident")
        .name("Duplicate Type Identifier")
        .purpose("Verify KTY3001 for same type name declared twice")
        .expect_error("KTY")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kty3001_duplicate_type_ident", result.stderr);
}

/// KTY3001: Duplicate type in nested namespace
#[tokio::test]
async fn kty3001_duplicate_type_nested() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kty3001b"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    struct Foo {
        value: str
    };

    struct Foo {
        count: i32
    };
};
"#,
    };

    let result = CliErrorTest::new("kty3001_duplicate_type_nested")
        .name("Duplicate Type in Nested Namespace")
        .purpose("Verify KTY3001 for duplicate type definitions in same namespace")
        .expect_error("KTY")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    // Should mention the duplicate type name
    assert!(result.stderr.contains("Foo"));

    insta::assert_snapshot!("kty3001_duplicate_type_nested", result.stderr);
}

/// KTY3003: Duplicate field name
/// Per ERR-0005: requires span on duplicate field
#[tokio::test]
async fn kty3003_duplicate_field() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kty3003"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    id: u64,
    id: str
};
"#,
    };

    // Per ERR-0005: KTY3003 requires span on duplicate field
    let result = CliErrorTest::new("kty3003_duplicate_field")
        .name("Duplicate Field Name")
        .purpose("Verify KTY3003 for same field name twice in struct")
        .expect_error("KTY3003")
        .requires_span(true) // Per ERR-0005: span required
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kty3003_duplicate_field", result.stderr);
}

/// KTY2001: Missing error type on fallible operation
#[tokio::test]
async fn kty2001_missing_error_type() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kty2001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    name: str
};

operation create_user(input: str) -> User!;
"#,
    };

    let result = CliErrorTest::new("kty2001_missing_error_type")
        .name("Missing Error Type")
        .purpose("Verify KTY2001 for fallible operation without #[err(...)]")
        .expect_error("KTY")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kty2001_missing_error_type", result.stderr);
}
