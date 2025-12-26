//! CLI tests for KPR (Parsing) errors per ERR-0003.
//!
//! Parsing errors occur during syntactic analysis when the parser encounters
//! malformed syntax, missing tokens, or invalid constructs.
//! Most KPR errors require source spans per SPEC-0022.

use kintsu_fs::memory;
use kintsu_test_suite::cli_tests::{CliErrorTest, minimal_manifest};

/// KPR0006 / KFS4002: Missing lib.ks file
#[tokio::test]
async fn kpr_missing_lib_ks() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kpr-missing-lib"),
        "pkg/schema/other.ks" => r#"namespace pkg; struct Foo {};"#,
    };

    let result = CliErrorTest::new("kpr_missing_lib_ks")
        .name("Missing lib.ks File")
        .purpose("Verify error for missing schema/lib.ks")
        .expect_error("K") // May be KPR or KFS
        .requires_span(false) // No source span for missing file
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kpr_missing_lib_ks", result.stderr);
}

/// KPR0008 / KPR2008: Type definition in lib.ks
#[tokio::test]
async fn kpr_lib_ks_type_definition() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kpr-libks-typedef"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

struct User {
    name: str
};
"#,
    };

    let result = CliErrorTest::new("kpr_lib_ks_type_definition")
        .name("Type Definition in lib.ks")
        .purpose("Verify KPR2008 for type definitions in lib.ks (only namespace and use allowed)")
        .expect_error("KPR")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kpr_lib_ks_type_definition", result.stderr);
}

/// KPR0010 / KFS4002: Empty file list (no .ks files)
#[tokio::test]
async fn kpr_empty_file_list() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-kpr-empty"),
        "pkg/schema/.gitkeep" => "",
    };

    let result = CliErrorTest::new("kpr_empty_file_list")
        .name("Empty File List")
        .purpose("Verify error for no .ks files to compile")
        .expect_error("K") // May be KPR or KFS
        .requires_span(false)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("kpr_empty_file_list", result.stderr);
}
