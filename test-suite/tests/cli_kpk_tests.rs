//! CLI tests for KPK (Package) errors per ERR-0011.
//!
//! Package errors occur when there are issues with the manifest (schema.toml),
//! lockfile, or dependency resolution.
//! Span requirements vary for KPK errors per SPEC-0022.

use kintsu_fs::memory;
use kintsu_test_suite::cli_tests::CliErrorTest;

/// KPK0001 / KPK9001: Invalid TOML in manifest
#[tokio::test]
async fn kpk_manifest_parse_error() {
    let fs = memory! {
        "pkg/schema.toml" => r#"
version = "v1"
[package
name = "broken"
"#,
        "pkg/schema/lib.ks" => "namespace pkg;",
    };

    let result = CliErrorTest::new("kpk_manifest_parse_error")
        .name("Manifest Parse Error")
        .purpose("Verify KPK error for invalid TOML in schema.toml")
        .expect_error("KPK")
        .requires_span(false) // Manifest errors may not have source span
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kpk_manifest_parse_error", result.stderr);
}

/// KPK4001 / KPK9001: Missing manifest
#[tokio::test]
async fn kpk_manifest_not_found() {
    let fs = memory! {
        "pkg/schema/lib.ks" => "namespace pkg;",
    };

    let result = CliErrorTest::new("kpk_manifest_not_found")
        .name("Manifest Not Found")
        .purpose("Verify KPK error for missing schema.toml")
        .expect_error("KPK")
        .requires_span(false)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kpk_manifest_not_found", result.stderr);
}

/// KPK3001 / KPK9001: Duplicate dependency (TOML duplicate key)
#[tokio::test]
async fn kpk_duplicate_dependency() {
    let fs = memory! {
        "pkg/schema.toml" => r#"version = "v1"

[package]
name = "test-pkg"
version = "1.0.0"

[dependencies]
common = { path = "../common" }
common = { path = "../other" }
"#,
        "pkg/schema/lib.ks" => "namespace pkg;",
    };

    let result = CliErrorTest::new("kpk_duplicate_dependency")
        .name("Duplicate Dependency")
        .purpose("Verify KPK error for same dependency listed twice (TOML duplicate key)")
        .expect_error("KPK")
        .requires_span(false)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kpk_duplicate_dependency", result.stderr);
}
