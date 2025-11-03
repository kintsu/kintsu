use kintsu_fs::memory;
use kintsu_test_suite::*;

#[tokio::test]
async fn compile_minimal_reproducible() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/minimal_lib.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_minimal_reproducible",
        "Minimal Reproducible Example",
        "Verify basic compilation of a single-file project with no dependencies",
        true,
        vec![Tag::Smoke],
    );

    let ctx = harness.compile_pass().await;
    assert_eq!(ctx.type_registry().all_types().len(), 1);
}

#[tokio::test]
async fn compile_namespace_by_file() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/use_bar.ks"),
        "pkg/schema/bar.ks" => include_str!("../fragments/bar_enum.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_namespace_by_file",
        "Public Namespace by File",
        "Test importing a namespace defined in a sibling file",
        true,
        vec![Tag::Smoke, Tag::Namespace],
    );

    let ctx = harness.compile_pass().await;
    assert_eq!(ctx.type_registry().all_types().len(), 1);
}

#[tokio::test]
async fn compile_namespace_by_directory() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/use_bar.ks"),
        "pkg/schema/bar/test.ks" => include_str!("../fragments/bar_enum.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_namespace_by_directory",
        "Public Namespace by Directory",
        "Test importing a namespace defined via directory structure",
        true,
        vec![Tag::Smoke, Tag::Namespace],
    );

    let ctx = harness.compile_pass().await;
    assert_eq!(ctx.type_registry().all_types().len(), 1);
}

#[tokio::test]
async fn compile_external_path_dependency() {
    let fs = memory! {
        "abc-corp/schema.toml" => include_str!("../fragments/abc_corp_manifest.toml"),
        "abc-corp/schema/lib.ks" => include_str!("../fragments/abc_corp_lib.ks"),
        "pkg/schema.toml" => include_str!("../fragments/pkg_with_dep_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/use_foo.ks"),
        "pkg/schema/foo/test.ks" => include_str!("../fragments/bar_with_external_type.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_external_path_dependency",
        "Smoke Package with External Path Dependency",
        "Test compilation with a path-based external dependency",
        true,
        vec![Tag::Smoke, Tag::Dependencies],
    );

    let _ = harness.compile_pass().await;
    harness.assert_lockfile_contains("abc-corp");
}

#[tokio::test]
async fn compile_transitive_dependencies() {
    let fs = memory! {
        "base/schema.toml" => include_str!("../fragments/base_manifest.toml"),
        "base/schema/lib.ks" => include_str!("../fragments/base_lib.ks"),
        "middle/schema.toml" => include_str!("../fragments/middle_manifest.toml"),
        "middle/schema/lib.ks" => include_str!("../fragments/middle_lib.ks"),
        "top/schema.toml" => include_str!("../fragments/top_manifest.toml"),
        "top/schema/lib.ks" => include_str!("../fragments/top_lib.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_transitive_dependencies",
        "Nested Path Dependencies (Transitive)",
        "Test transitive dependency resolution",
        true,
        vec![Tag::Smoke, Tag::Dependencies],
    );
    harness.set_root("top");

    let _ = harness.compile_pass().await;
    harness.assert_lockfile_contains("middle");
    harness.assert_lockfile_contains("base");
}

#[tokio::test]
async fn compile_diamond_dependency() {
    let fs = memory! {
        "common/schema.toml" => include_str!("../fragments/common_manifest.toml"),
        "common/schema/lib.ks" => include_str!("../fragments/common_lib.ks"),
        "lib-a/schema.toml" => include_str!("../fragments/lib_a_manifest.toml"),
        "lib-a/schema/lib.ks" => include_str!("../fragments/lib_a_lib.ks"),
        "lib-b/schema.toml" => include_str!("../fragments/lib_b_manifest.toml"),
        "lib-b/schema/lib.ks" => include_str!("../fragments/lib_b_lib.ks"),
        "app/schema.toml" => include_str!("../fragments/app_manifest.toml"),
        "app/schema/lib.ks" => include_str!("../fragments/app_lib.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_diamond_dependency",
        "Diamond Dependency Resolution",
        "Test multiple dependencies with shared transitive dependency",
        true,
        vec![Tag::Smoke, Tag::Dependencies],
    );
    harness.set_root("app");

    let _ = harness.compile_pass().await;

    harness.assert_lockfile_contains("common");
    harness.assert_lockfile_contains("lib-a");
    harness.assert_lockfile_contains("lib-b");

    let lockfile = harness.read_lockfile().unwrap();
    let common_count = lockfile.matches("name = \"common\"").count();
    assert_eq!(
        common_count, 1,
        "Common should appear exactly once in lockfile"
    );
}

#[tokio::test]
async fn compile_valid_lockfile_checksum() {
    // First, compile to generate a valid lockfile
    let fs = memory! {
        "dep/schema.toml" => include_str!("../fragments/dep_manifest.toml"),
        "dep/schema/lib.ks" => include_str!("../fragments/dep_lib.ks"),
        "pkg/schema.toml" => r#"[package]
name = "pkg"
version = "1.0.0"

[dependencies]
dep = { path = "../dep" }
"#,
        "pkg/schema/lib.ks" => "namespace pkg;\nnamespace uses_dep {\n\tuse dep::data::Data;\n\tstruct Wrapper { data: Data };\n};",
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_valid_lockfile_checksum",
        "Lockfile Validation - Valid Checksum",
        "Test that valid lockfile checksums are accepted and don't trigger rewrites",
        true,
        vec![Tag::Smoke, Tag::Lockfile],
    );

    let _ = harness.compile_pass().await;
    let lockfile_content = harness
        .read_lockfile()
        .expect("Lockfile should exist");

    // Add the lockfile to the same filesystem and compile again
    harness.add_text_file("pkg/schema.lock", &lockfile_content);
    harness.clear_operations();

    let _ = harness.compile_pass().await;

    // Lockfile should not be rewritten (validated successfully)
    let write_count = harness.count_operations("Write");
    assert_eq!(
        write_count, 0,
        "Lockfile should not be rewritten when valid"
    );
}

#[tokio::test]
async fn compile_version_pruning() {
    let fs = memory! {
        "lib/schema.toml" => r#"[package]
name = "lib"
version = "1.2.3"
"#,
        "lib/schema/lib.ks" => "namespace lib;\nnamespace types {\n\tstruct Item { id: i32 };\n};",
        "pkg/schema.toml" => r#"[package]
name = "pkg"
version = "1.0.0"

[dependencies]
lib = { path = "../lib", version = "^1.0" }
"#,
        "pkg/schema/lib.ks" => "namespace pkg;\nnamespace foo {\nuse lib;\ntype Foo = lib::types::Item;\n};",
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_version_pruning",
        "Version Compatibility - Multiple Compatible Versions",
        "Test version pruning keeps highest compatible version",
        true,
        vec![Tag::Smoke, Tag::VersionResolution],
    );

    let _ = harness.compile_pass().await;
    harness.assert_lockfile_written();
    harness.assert_lockfile_contains("lib");
}

#[tokio::test]
async fn compile_string_enum() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/string_enum.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_string_enum",
        "Enum with String Values",
        "Test string-based enum compilation",
        true,
        vec![Tag::Smoke, Tag::Soundness],
    );

    let _ = harness.compile_pass().await;
    harness.assert_lockfile_written();
}

#[tokio::test]
async fn compile_oneof_mixed_types() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/oneof_mixed.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_oneof_mixed_types",
        "OneOf with Multiple Types",
        "Test union type compilation with mixed types",
        true,
        vec![Tag::Smoke, Tag::Soundness],
    );

    let _ = harness.compile_pass().await;
    harness.assert_lockfile_written();
}
