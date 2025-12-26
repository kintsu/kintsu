//! CLI tests for KTE (Type Expression) errors per ERR-0010.
//!
//! Type expression errors occur when there are issues with type expression
//! operators like `Pick`, `Omit`, `Partial`, `Required`, `Extract`, `Exclude`.
//! All KTE errors require source spans per SPEC-0022.

use kintsu_fs::memory;
use kintsu_test_suite::cli_tests::{CliErrorTest, minimal_manifest};

/// KTE0001: Missing open bracket after operator
#[tokio::test]
async fn kte0001_missing_open_bracket() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kte0001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    id: u64,
    name: str,
    email: str
};

type PartialUser = Pick User, id;
"#,
    };

    let result = CliErrorTest::new("kte0001_missing_open_bracket")
        .name("Missing Open Bracket")
        .purpose("Verify KTE0001 for missing '[' after operator name")
        .expect_error("K") // May be KTE or KPR
        .requires_span(true) // Per ERR-0010: span required on operator keyword
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kte0001_missing_open_bracket", result.stderr);
}

/// KTE0002: Unclosed bracket in type expression
#[tokio::test]
async fn kte0002_unclosed_bracket() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kte0002"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    id: u64,
    name: str
};

type PartialUser = Pick[User, id | name;
"#,
    };

    let result = CliErrorTest::new("kte0002_unclosed_bracket")
        .name("Unclosed Bracket")
        .purpose("Verify KTE0002 for missing ']' to close operator")
        .expect_error("K") // May be KTE or KPR
        .requires_span(true) // Per ERR-0010: span required
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kte0002_unclosed_bracket", result.stderr);
}

/// KTE2001: Expected struct type (got enum)
#[tokio::test]
async fn kte2001_expected_struct_type() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kte2001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

enum Status {
    Active = 1,
    Inactive = 2
};

type PartialStatus = Pick[Status, Active];
"#,
    };

    let result = CliErrorTest::new("kte2001_expected_struct_type")
        .name("Expected Struct Type")
        .purpose("Verify KTE2001 for Pick/Omit on non-struct type")
        .expect_error("KTE")
        .requires_span(true) // Per ERR-0010: span required
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kte2001_expected_struct_type", result.stderr);
}

/// KTE1001: Unknown field in selector
#[tokio::test]
async fn kte1001_unknown_field() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kte1001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    id: u64,
    name: str
};

type PartialUser = Pick[User, id | nonexistent];
"#,
    };

    let result = CliErrorTest::new("kte1001_unknown_field")
        .name("Unknown Field in Selector")
        .purpose("Verify KTE1001 for unknown field name in Pick selector")
        .expect_error("KTE")
        .requires_span(true) // Per ERR-0010: span required
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kte1001_unknown_field", result.stderr);
}

/// KTE4001: Empty selector list
#[tokio::test]
async fn kte4001_empty_selector_list() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kte4001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    id: u64,
    name: str
};

type EmptyUser = Pick[User, ];
"#,
    };

    let result = CliErrorTest::new("kte4001_empty_selector_list")
        .name("Empty Selector List")
        .purpose("Verify KTE4001 for empty field selector list")
        .expect_error("K") // May be KTE or KPR
        .requires_span(true) // Per ERR-0010: span required
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kte4001_empty_selector_list", result.stderr);
}

/// KTE4002: No fields remain after Omit
#[tokio::test]
async fn kte4002_no_fields_remain() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kte4002"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    id: u64,
    name: str
};

type EmptyUser = Omit[User, id | name];
"#,
    };

    let result = CliErrorTest::new("kte4002_no_fields_remain")
        .name("No Fields Remain")
        .purpose("Verify KTE4002 when Omit removes all fields")
        .expect_error("KTE")
        .requires_span(true) // Per ERR-0010: span required
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kte4002_no_fields_remain", result.stderr);
}

/// KTE2002: Expected oneof type for Extract/Exclude
#[tokio::test]
async fn kte2002_expected_oneof_type() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kte2002"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    id: u64,
    name: str
};

type ExtractedUser = Extract[User, id];
"#,
    };

    let result = CliErrorTest::new("kte2002_expected_oneof_type")
        .name("Expected OneOf Type")
        .purpose("Verify KTE2002 for Extract on non-oneof type")
        .expect_error("KTE")
        .requires_span(true) // Per ERR-0010: span required
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kte2002_expected_oneof_type", result.stderr);
}
