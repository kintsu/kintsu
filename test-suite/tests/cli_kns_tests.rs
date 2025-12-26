//! CLI tests for KNS (Namespace) errors per ERR-0004.
//!
//! Namespace errors occur when there are issues with namespace declarations,
//! namespace resolution, or namespace conflicts.
//! Most KNS errors require source spans per SPEC-0022.

use kintsu_fs::memory;
use kintsu_test_suite::cli_tests::{CliErrorTest, minimal_manifest};

/// KNS1001: No namespace declaration in file
#[tokio::test]
async fn kns1001_no_namespace_declaration() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kns1001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"
struct User {
    name: str
};
"#,
    };

    let result = CliErrorTest::new("kns1001_no_namespace_declaration")
        .name("Missing Namespace in Non-lib File")
        .purpose("Verify KNS1001 for files without namespace declaration")
        .expect_error("KNS")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kns1001_no_namespace_declaration", result.stderr);
}

/// KNS3001: Multiple namespace declarations in one file
#[tokio::test]
async fn kns3001_multiple_namespaces() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kns3001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;
namespace models;

struct User {
    name: str
};
"#,
    };

    let result = CliErrorTest::new("kns3001_multiple_namespaces")
        .name("Multiple Namespace Declarations")
        .purpose("Verify KNS3001 for multiple namespace declarations in one file")
        .expect_error("KNS")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kns3001_multiple_namespaces", result.stderr);
}

/// KNS4001: Use path not found
#[tokio::test]
async fn kns4001_use_path_not_found() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kns4001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use models;
"#,
    };

    let result = CliErrorTest::new("kns4001_use_path_not_found")
        .name("Use Path Not Found")
        .purpose("Verify KNS4001 for use statement with no corresponding file/directory")
        .expect_error("KNS")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kns4001_use_path_not_found", result.stderr);
}

/// KPR0008/KPR0009: Missing namespace declaration in lib.ks triggers struct definition error
/// Per ERR-0003: KPR0008 requires span (the invalid item), KPR0009 requires span (start of file)
#[tokio::test]
async fn kns_missing_namespace_lib_ks() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kns-missing-ns"),
        "pkg/schema/lib.ks" => r#"
struct Foo {
    value: str
};
"#,
    };

    let result = CliErrorTest::new("kns_missing_namespace_lib_ks")
        .name("Missing Namespace in lib.ks")
        .purpose("Verify error when namespace declaration is missing in lib.ks")
        .expect_error("KPR") // KPR0008 or KPR0009 per ERR-0003
        .requires_span(true) // Both KPR0008 and KPR0009 require spans per ERR-0003
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kns_missing_namespace_lib_ks", result.stderr);
}

/// KNS: Missing dependency (use statement for non-existent namespace)
#[tokio::test]
async fn kns_missing_dependency() {
    let fs = memory! {
        "pkg/schema.toml" => r#"version = "v1"

[package]
name = "test-pkg"
version = "1.0.0"

[dependencies]
missing-dep = { path = "../missing-dep" }
"#,
        "pkg/schema/lib.ks" => "namespace pkg; use missing_dep;",
    };

    let result = CliErrorTest::new("kns_missing_dependency")
        .name("Missing Dependency")
        .purpose("Verify KNS error when a path dependency doesn't exist")
        .expect_error("K") // May be KNS or KPK
        .requires_span(false)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kns_missing_dependency", result.stderr);
}
