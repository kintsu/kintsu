//! CLI tests for KMT (Metadata) errors per ERR-0008.
//!
//! Metadata errors occur when there are issues with version attributes,
//! error attributes, and other metadata declarations.
//! All KMT errors require source spans per SPEC-0022.

use kintsu_fs::memory;
use kintsu_test_suite::cli_tests::{CliErrorTest, minimal_manifest};

/// KMT3001: Version attribute conflict
#[tokio::test]
async fn kmt3001_version_conflict() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kmt3001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

#[version(1)]
#[version(2)]
struct User {
    name: str
};
"#,
    };

    let result = CliErrorTest::new("kmt3001_version_conflict")
        .name("Version Attribute Conflict")
        .purpose("Verify KMT3001 for duplicate version attributes on same item")
        .expect_error("KMT")
        .requires_span(true) // Per ERR-0008: span required on conflicting attribute
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kmt3001_version_conflict", result.stderr);
}

/// KMT3002: Duplicate meta attribute (err attribute)
#[tokio::test]
async fn kmt3002_duplicate_err_attribute() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kmt3002"),
        "pkg/schema/lib.ks" => r#"#![err(ApiError)]
#![err(OtherError)]
namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

error ApiError {
    NotFound { message: str }
};

error OtherError {
    Unknown
};
"#,
    };

    let result = CliErrorTest::new("kmt3002_duplicate_err_attribute")
        .name("Duplicate Err Attribute")
        .purpose("Verify KMT3002 for duplicate #![err(...)] attributes")
        .expect_error("KMT")
        .requires_span(true) // Per ERR-0008: span required on duplicate attribute
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kmt3002_duplicate_err_attribute", result.stderr);
}

/// KMT2001: Invalid version value (not a positive integer)
#[tokio::test]
async fn kmt2001_invalid_version_value() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kmt2001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

#[version(-1)]
struct User {
    name: str
};
"#,
    };

    let result = CliErrorTest::new("kmt2001_invalid_version_value")
        .name("Invalid Version Value")
        .purpose("Verify KMT2001 for non-positive integer in version attribute")
        .expect_error("K") // May be KMT or KLX/KPR for parsing
        .requires_span(true) // Per ERR-0008: span required on invalid value
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kmt2001_invalid_version_value", result.stderr);
}

/// KMT2002: Invalid error attribute (non-existent error type)
#[tokio::test]
async fn kmt2002_invalid_error_attribute() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kmt2002"),
        "pkg/schema/lib.ks" => r#"#![err(NonExistentError)]
namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    name: str
};
"#,
    };

    let result = CliErrorTest::new("kmt2002_invalid_error_attribute")
        .name("Invalid Error Attribute")
        .purpose("Verify KMT2002 for #![err(...)] referencing non-existent error type")
        .expect_error("K") // May be KMT or KTR for resolution
        .requires_span(true) // Per ERR-0008: span required on invalid reference
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kmt2002_invalid_error_attribute", result.stderr);
}
