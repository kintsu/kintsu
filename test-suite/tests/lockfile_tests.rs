use kintsu_fs::memory;
use kintsu_test_macros::compiler_test;
use kintsu_test_suite::*;

compiler_test! {
    id: test_lockfile_generation_from_scratch,
    name: "Lockfile Generation from Scratch",
    purpose: "Verify lockfile creation when none exists",
    expect_pass: true,
    tags: vec![Tag::Lockfile],
    root: "pkg",
    memory: || {
        memory! {
                "dep/schema.toml" => include_str!("../fragments/dep_manifest.toml"),
                "dep/schema/lib.ks" => include_str!("../fragments/dep_lib.ks"),
                "pkg/schema.toml" => r#"[package]
        name = "pkg"
        version = "1.0.0"

        [dependencies]
        dep = { path = "../dep" }
        "#,
                "pkg/schema/lib.ks" => "namespace pkg;\nuse dep;\nstruct Wrapper { data: Data }",
        }
    },
    assertions: |harness, _: CompileCtx| {
        // Lockfile should be created
        harness.assert_lockfile_written();
        harness.assert_lockfile_contains("dep");
        harness.assert_lockfile_contains("1.0.0");

        let lockfile = harness.read_lockfile().unwrap();
        assert!(
            lockfile.contains("checksum"),
            "Lockfile should contain checksum"
        );
    }
}

#[tokio::test]
async fn test_lockfile_02_unchanged_no_rewrite() {
    let fs = memory! {
        "dep/schema.toml" => include_str!("../fragments/dep_manifest.toml"),
        "dep/schema/lib.ks" => include_str!("../fragments/dep_lib.ks"),
        "pkg/schema.toml" => r#"[package]
name = "pkg"
version = "1.0.0"

[dependencies]
dep = { path = "../dep" }
"#,
        "pkg/schema/lib.ks" => "namespace pkg;\nuse dep;",
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "test_lockfile_02_unchanged_no_rewrite",
        "Lockfile Unchanged - No Rewrite",
        "Verify lockfile isn't rewritten when nothing changes",
        true,
        vec![Tag::Lockfile],
    );

    let _ = harness.compile_pass().await;
    let lockfile_content = harness.read_lockfile().unwrap();

    harness.add_text_file("pkg/schema.lock", &lockfile_content);
    harness.clear_operations();

    let _ = harness.compile_pass().await;

    let write_count = harness.count_operations("Write");
    assert_eq!(
        write_count, 0,
        "Lockfile should not be rewritten when unchanged",
    );
}

#[tokio::test]
async fn test_lockfile_03_dependency_added() {
    let fs = memory! {
        "dep-a/schema.toml" => r#"[package]
name = "dep-a"
version = "1.0.0"
"#,
        "dep-a/schema/lib.ks" => "namespace dep_a;\nstruct A { value: str }",
        "dep-b/schema.toml" => r#"[package]
name = "dep-b"
version = "1.0.0"
"#,
        "dep-b/schema/lib.ks" => "namespace dep_b;\nstruct B { count: i32 }",
        "pkg/schema.toml" => r#"[package]
name = "pkg"
version = "1.0.0"

[dependencies]
dep-a = { path = "../dep-a" }
"#,
        "pkg/schema/lib.ks" => "namespace pkg;\nuse dep_a;",
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "test_lockfile_03_dependency_added",
        "Lockfile Update - Dependency Added",
        "Test lockfile update when new dependency is added",
        true,
        vec![Tag::Lockfile],
    );

    let _ = harness.compile_pass().await;
    let old_lockfile = harness.read_lockfile().unwrap();

    // Add second dependency to manifest and usage
    harness.add_text_file(
        "pkg/schema.toml",
        r#"[package]
name = "pkg"
version = "1.0.0"

[dependencies]
dep-a = { path = "../dep-a" }
dep-b = { path = "../dep-b" }
"#,
    );
    harness.add_text_file(
        "pkg/schema.lib.ks",
        "namespace pkg;\nuse dep_a;\nuse dep_b;",
    );
    harness.add_text_file("pkg/schema.lock", &old_lockfile);

    let _ = harness.compile_pass().await;

    // New lockfile should contain both dependencies
    harness.assert_lockfile_contains("dep-a");
    harness.assert_lockfile_contains("dep-b");
}

compiler_test! {
    id: test_lockfile_04_version_pruning,
    name: "Lockfile Pruning - Version Management",
    purpose: "Test that version pruning removes lower compatible versions",
    expect_pass: true,
    tags: vec![Tag::Lockfile],
    root: "pkg",
    memory: || {
        memory! {
            "lib/schema.toml" => r#"[package]
name = "lib"
version = "1.5.0"
"#,
            "lib/schema/lib.ks" => "namespace lib;\nstruct Item { id: u64 }",
            "pkg/schema.toml" => r#"[package]
name = "pkg"
version = "1.0.0"

[dependencies]
lib = { path = "../lib", version = "^1.0" }
"#,
            "pkg/schema/lib.ks" => "namespace pkg;\nuse lib;",
        }
    },
    assertions: |harness, _: CompileCtx| {
        harness.assert_lockfile_written();
        let lockfile = harness.read_lockfile().unwrap();

        assert!(
            lockfile.contains("1.5.0"),
            "Lockfile should contain version 1.5.0"
        );

        let lib_count = lockfile.matches("name = \"lib\"").count();
        assert_eq!(lib_count, 1, "Should have exactly one lib entry");
    }
}

#[tokio::test]
async fn test_lockfile_05_nested_merging() {
    // Create single filesystem with all packages
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
        "test_lockfile_05_nested_merging",
        "Nested Lockfile Merging",
        "Test nested lockfile merging when a dependency has its own lockfile",
        true,
        vec![Tag::Lockfile],
    );

    // First compile middle (which depends on base)
    harness.set_root("middle");
    harness.compile_pass().await;
    let middle_lockfile = harness.read_lockfile().unwrap();

    // Add middle's lockfile to the filesystem
    harness.add_text_file("middle/schema.lock", &middle_lockfile);

    // Now compile top (which depends on middle)
    harness.set_root("top");
    harness.compile_pass().await;

    // Top's lockfile should include both middle and base
    harness.assert_lockfile_contains("middle");
    harness.assert_lockfile_contains("base");
}

#[tokio::test]
async fn test_operation_01_minimal_analysis() {
    let fs = memory! {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => "namespace pkg;\nstruct Item { value: str }",
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "test_operation_01_minimal_analysis",
        "Minimal Compilation - Operation Analysis",
        "Verify filesystem operations are minimal and reproducible",
        true,
        vec![Tag::Lockfile],
    );

    harness.clear_operations(); // Clear setup operations

    let _ = harness.compile_pass().await;

    harness.assert_operation_occurred("ExistsSync");
    harness.assert_operation_occurred("ReadToString");

    let write_count = harness.count_operations("Write");
    assert_eq!(
        write_count, 0,
        "Should not write anything for minimal project"
    );
}

#[tokio::test]
async fn test_operation_02_with_dependency_analysis() {
    let fs = memory! {
        "dep/schema.toml" => include_str!("../fragments/dep_manifest.toml"),
        "dep/schema/lib.ks" => include_str!("../fragments/dep_lib.ks"),
        "pkg/schema.toml" => r#"[package]
name = "pkg"
version = "1.0.0"

[dependencies]
dep = { path = "../dep" }
"#,
        "pkg/schema/lib.ks" => "namespace pkg;\nuse dep;",
    };

    let mut harness = TestHarness::with_metadata(
        fs,
        "test_operation_02_with_dependency_analysis",
        "With Dependency - Operation Analysis",
        "Track operations when loading dependencies",
        true,
        vec![Tag::Lockfile],
    );

    harness.clear_operations();

    let _ = harness.compile_pass().await;

    let read_count = harness.count_operations("ReadToString");
    assert!(
        read_count >= 3,
        "Should read at least 3 files (pkg manifest, dep manifest, sources). Got: {}",
        read_count
    );

    let write_count = harness.count_operations("Write");
    assert_eq!(
        write_count, 1,
        "Should write exactly one file (lockfile). Got: {}",
        write_count
    );

    let ops1 = harness.operations();

    harness.clear_operations();
    let _ = harness.compile_pass().await;
    let ops2 = harness.operations();

    assert!(
        !ops1.is_empty() && !ops2.is_empty(),
        "Both compilations should have operations"
    );
}
