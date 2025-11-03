use kintsu_fs::memory;
use kintsu_test_suite::*;

#[tokio::test]
async fn compile_fail_missing_dependency() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/missing_dep_use.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_fail_missing_dependency",
        "Missing Dependency",
        "Verify compilation fails when a used dependency is not declared",
        false,
        vec![Tag::Validations],
    );

    let err = harness.compile_fail().await;
    let err_msg = format!("{:?}", err);
    assert!(
        err_msg.contains("external_pkg")
            || err_msg.contains("not found")
            || err_msg.contains("dependency"),
        "Error should mention missing dependency: {}",
        err_msg
    );
}

#[ignore = "currently failing"]
#[tokio::test]
async fn compile_fail_circular_dependency() {
    let fs = memory! {
        "pkg-a/schema.toml" => r#"[package]
name = "pkg-a"
version = "1.0.0"

[dependencies]
pkg-b = { path = "../pkg-b" }
"#,
        "pkg-a/schema/lib.ks" => "namespace pkg_a;\nnamespace foo { use pkg_b;\ntype Foo = i32;\ntype Bar = pkg_b::foo::Bar; };",
        "pkg-b/schema.toml" => r#"[package]
name = "pkg-b"
version = "1.0.0"

[dependencies]
pkg-a = { path = "../pkg-a" }
"#,
        "pkg-b/schema/lib.ks" => "namespace pkg_b;\nnamespace foo { use pkg_a;\ntype Foo = pkg_a::foo::Foo; };",
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_fail_circular_dependency",
        "Circular Dependency",
        "Detect and reject circular dependency chains",
        false,
        vec![Tag::Validations],
    );
    harness.set_root("pkg-a");

    let err = harness.compile_fail().await;

    // Verify error mentions circular dependency
    let err_msg = format!("{:?}", err);
    assert!(
        err_msg.contains("circular") || err_msg.contains("cycle"),
        "Error should mention circular dependency: {}",
        err_msg
    );
}

#[tokio::test]
async fn compile_fail_invalid_checksum() {
    let fs = memory! {
        "dep/schema.toml" => include_str!("../fragments/dep_manifest.toml"),
        "dep/schema/lib.ks" => include_str!("../fragments/dep_lib.ks"),
        "pkg/schema.toml" => r#"[package]
name = "pkg"
version = "1.0.0"

[dependencies]
dep = { path = "../dep" }
"#,
        "pkg/schema.lock" => r#"# This lockfile has an INCORRECT checksum
[[package]]
name = "dep"
version = "1.0.0"
source = { path = "../dep" }
checksum = "deadbeefdeadbeefdeadbeefdeadbeef"

[provides]
dep = ["dep"]
"#,
        "pkg/schema/lib.ks" => "namespace pkg;\nnamespace main { use dep;  type PkgData = dep::data::Data; };",
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_fail_invalid_checksum",
        "Invalid Lockfile Checksum",
        "Verify compilation detects modified dependencies and regenerates lockfile",
        true, // Should succeed but regenerate lockfile
        vec![Tag::Lockfile],
    );

    let _ = harness.compile_pass().await;
    let write_ops = harness.count_operations("Write");
    assert!(
        write_ops > 0,
        "Should have rewritten lockfile with correct checksum"
    );
}

#[ignore = "todo: fix the version conflict detection"]
#[tokio::test]
async fn compile_fail_version_conflict() {
    let fs = memory! {
        "lib/schema.toml" => r#"
            [package]
            name = "lib"
            version = "2.0.0"
        "#,
        "lib/schema/lib.ks" => "namespace lib; namespace bar { struct Item { id: u64 }; };",
        "pkg-a/schema.toml" => r#"
            [package]
            name = "pkg-a"
            version = "1.0.0"

            [dependencies]
            lib = { path = "../lib", version = "^1.0" }
        "#,
        "pkg-a/schema/lib.ks" => "namespace pkg_a; namespace foo { use lib;  type Foo = lib::bar::Item; };",
        "pkg-b/schema.toml" => r#"
            [package]
            name = "pkg-b"
            version = "1.0.0"

            [dependencies]
            lib = { path = "../lib", version = "^2.0" }
        "#,
        "pkg-b/schema/lib.ks" => "namespace pkg_b; namespace baz { use lib; type Baz = lib::bar::Item; };",
        "app/schema.toml" => r#"
            [package]
            name = "app"
            version = "1.0.0"

            [dependencies]
            pkg-a = { path = "../pkg-a" }
            pkg-b = { path = "../pkg-b" }
        "#,
        "app/schema/lib.ks" => "namespace app; namespace foo { use pkg_a; use pkg_b; type App = oneof pkg_a::foo::Foo | pkg_b::baz::Baz; };",
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_fail_version_conflict",
        "Version Conflict - Incompatible Requirements",
        "Detect when dependencies require incompatible versions",
        false,
        vec![Tag::Validations],
    );
    harness.set_root("app");

    let err = harness.compile_fail().await;
    let err_msg = format!("{:?}", err);
    assert!(
        err_msg.contains("version")
            || err_msg.contains("conflict")
            || err_msg.contains("incompatible"),
        "Error should mention version conflict: {}",
        err_msg
    );
}

#[tokio::test]
async fn compile_fail_missing_namespace() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/missing_namespace_use.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_fail_missing_namespace",
        "Missing Namespace File",
        "Verify error when imported namespace doesn't exist",
        false,
        vec![Tag::Validations],
    );

    let err = harness.compile_fail().await;
    let err_msg = format!("{:?}", err);
    assert!(
        err_msg.contains("missing_namespace") || err_msg.contains("not found"),
        "Error should mention missing namespace: {}",
        err_msg
    );
}

#[tokio::test]
async fn compile_fail_undefined_type() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/undefined_type.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_fail_undefined_type",
        "Type Reference to Undefined Type",
        "Catch references to non-existent types",
        false,
        vec![Tag::Validations],
    );

    let err = harness.compile_fail().await;
    let err_msg = format!("{:?}", err);
    assert!(
        err_msg.contains("UndefinedType")
            || err_msg.contains("not found")
            || err_msg.contains("undefined"),
        "Error should mention undefined type: {}",
        err_msg
    );
}

#[tokio::test]
async fn compile_fail_duplicate_type() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/duplicate_type.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_fail_duplicate_type",
        "Duplicate Type Definition",
        "Prevent conflicting type definitions in same namespace",
        false,
        vec![Tag::Validations],
    );

    let err = harness.compile_fail().await;
    let err_msg = format!("{:?}", err);
    assert!(
        err_msg.contains("duplicate")
            || err_msg.contains("Foo")
            || err_msg.contains("already defined"),
        "Error should mention duplicate type: {}",
        err_msg
    );
}

#[tokio::test]
async fn compile_fail_invalid_enum_discriminant() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/invalid_enum_discriminant.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_fail_invalid_enum_discriminant",
        "Invalid Enum Discriminant",
        "Reject invalid enum values",
        false,
        vec![Tag::Validations],
    );

    let _ = harness.compile_fail().await;
}

#[tokio::test]
async fn compile_fail_malformed_manifest() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/malformed_manifest.toml"),
        "pkg/schema/lib.ks" => "namespace pkg;",
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_fail_malformed_manifest",
        "Malformed Manifest",
        "Catch syntax errors in TOML manifests",
        false,
        vec![Tag::Soundness],
    );

    let err = harness.compile_fail().await;
    let err_msg = format!("{:?}", err);
    assert!(
        err_msg.contains("unclosed table"),
        "Error should mention TOML parse error: {}",
        err_msg
    );
}

#[tokio::test]
async fn compile_fail_wrong_import_syntax() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/wrong_import_syntax.ks"),
        "pkg/schema/internal.ks" => include_str!("../fragments/internal_namespace.ks"),
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_fail_wrong_import_syntax",
        "Import from Same Package with Wrong Path",
        "Verify imports within package use correct syntax",
        false,
        vec![Tag::Validations],
    );

    let err = harness.compile_fail().await;
    let err_msg = format!("{:?}", err);
    assert!(
        !err_msg.is_empty(),
        "Should produce an error for incorrect import syntax"
    );
}
