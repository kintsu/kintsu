//! CLI tests for KTG (Tagging) errors per ERR-0009.
//!
//! Tagging errors occur when there are issues with variant tagging
//! in `oneof` and `error` types.
//! All KTG errors require source spans per SPEC-0022.

use kintsu_fs::memory;
use kintsu_test_suite::cli_tests::{CliErrorTest, minimal_manifest};

/// KTG2001: Tag parameter invalid type (expected string literal)
#[tokio::test]
async fn ktg2001_tag_parameter_invalid_type() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-ktg2001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct Payload { data: str };

#[tag(name = 42)]
type Result = oneof
    | Success(Payload)
    | Failure(str);
"#,
    };

    let result = CliErrorTest::new("ktg2001_tag_parameter_invalid_type")
        .name("Tag Parameter Invalid Type")
        .purpose("Verify KTG2001 for tag parameter not being a string literal")
        .expect_error("K") // May be KTG or KPR for parsing
        .requires_span(true) // Per ERR-0009: span required on invalid parameter
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("ktg2001_tag_parameter_invalid_type", result.stderr);
}

/// KTG3001: Multiple tagging styles
/// NOTE: Per TSY-0013, duplicate #[tag(...)] attributes on same type should trigger this error
#[tokio::test]
async fn ktg3001_multiple_tag_styles() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-ktg3001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct Payload { data: str };

#[tag(external)]
#[tag(name = "kind")]
oneof Result {
    Success(Payload),
    Failure(str)
};
"#,
    };

    let result = CliErrorTest::new("ktg3001_multiple_tag_styles")
        .name("Multiple Tagging Styles")
        .purpose("Verify KTG3001 for specifying multiple tagging styles")
        .expect_error("KTG")
        .requires_span(true) // Per ERR-0009: span required on tag attribute
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("ktg3001_multiple_tag_styles", result.stderr);
}

/// KTG2002: Tag on non-variant type (struct)
#[tokio::test]
async fn ktg2002_tag_on_struct() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-ktg2002"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

#[tag(external)]
struct User {
    name: str
};
"#,
    };

    let result = CliErrorTest::new("ktg2002_tag_on_struct")
        .name("Tag on Non-Variant Type")
        .purpose("Verify KTG2002 for tag attribute on struct (invalid)")
        .expect_error("KTG")
        .requires_span(true) // Per ERR-0009: span required on tag attribute
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("ktg2002_tag_on_struct", result.stderr);
}

/// KTG3002: Internal tag field conflict
/// Per TSY-0013: internal tagging uses #[tag(name = "field_name")]
#[tokio::test]
async fn ktg3002_internal_tag_field_conflict() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-ktg3002"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct Success {
    type: str,
    data: str
};

struct Failure {
    message: str
};

#[tag(name = "type")]
oneof Result {
    Success(Success),
    Failure(Failure)
};
"#,
    };

    let result = CliErrorTest::new("ktg3002_internal_tag_field_conflict")
        .name("Internal Tag Field Conflict")
        .purpose("Verify KTG3002 when internal tag name conflicts with variant field")
        .expect_error("KTG")
        .requires_span(true) // Per ERR-0009: span required on conflicting field
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("ktg3002_internal_tag_field_conflict", result.stderr);
}

/// KTG3004: Untagged duplicate type
/// Per TSY-0013: untagged oneofs cannot have duplicate variant types
#[tokio::test]
async fn ktg3004_untagged_duplicate_type() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-ktg3004"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

#[tag(untagged)]
oneof StringOrString {
    First(str),
    Second(str)
};
"#,
    };

    let result = CliErrorTest::new("ktg3004_untagged_duplicate_type")
        .name("Untagged Duplicate Type")
        .purpose("Verify KTG3004 for untagged oneof with duplicate variant types")
        .expect_error("KTG")
        .requires_span(true) // Per ERR-0009: span required on duplicate variant
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("ktg3004_untagged_duplicate_type", result.stderr);
}

/// KTG2003: Internal tag requires struct variants
/// Per TSY-0013: internal tagging uses #[tag(name = "field_name")] and requires struct variants
#[tokio::test]
async fn ktg2003_internal_tag_requires_struct() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-ktg2003"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

#[tag(name = "type")]
oneof Result {
    Success(str),
    Failure(i32)
};
"#,
    };

    let result = CliErrorTest::new("ktg2003_internal_tag_requires_struct")
        .name("Internal Tag Requires Struct")
        .purpose("Verify KTG2003 when internal tagging used with non-struct variants")
        .expect_error("KTG")
        .requires_span(true) // Per ERR-0009: span required on non-struct variant
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("ktg2003_internal_tag_requires_struct", result.stderr);
}
