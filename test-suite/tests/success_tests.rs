use kintsu_fs::memory;
use kintsu_test_macros::compiler_test;
use kintsu_test_suite::*;

compiler_test! {
    id: compile_minimal_reproducible,
    name: "Minimal Reproducible Example",
    purpose: "Verify basic compilation of a single-file project with no dependencies",
    expect_pass: true,
    tags: vec![Tag::Smoke],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/minimal_lib.ks"),
        }
    },
    assertions: |_, ctx: CompileCtx| {
        assert_eq!(ctx.type_registry().all_types().len(), 1);
    }
}

compiler_test! {
    id: compile_empty_package,
    name: "Empty Package Compilation",
    purpose: "Verify compilation of a package with no types defined",
    expect_pass: true,
    tags: vec![Tag::Smoke],
    root: "empty_pkg",
    memory: || {
        memory! {
            "empty_pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "empty_pkg/schema/lib.ks" => "namespace pkg;",
        }
    },
    assertions: |_, ctx: CompileCtx| {
        assert_eq!(ctx.type_registry().all_types().len(), 0);
    }
}

compiler_test! {
    id: compile_namespace_by_file,
    name: "Public Namespace by File",
    purpose: "Test importing a namespace defined in a sibling file",
    expect_pass: true,
    tags: vec![Tag::Smoke, Tag::Namespace],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/use_bar.ks"),
            "pkg/schema/bar.ks" => include_str!("../fragments/bar_enum.ks"),
        }
    },
    assertions: |_, ctx: CompileCtx| {
        assert_eq!(ctx.type_registry().all_types().len(), 1);
    }
}

compiler_test! {
    id: compile_namespace_by_directory,
    name: "Public Namespace by Directory",
    purpose: "Test importing a namespace defined via directory structure",
    expect_pass: true,
    tags: vec![Tag::Smoke, Tag::Namespace],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/use_bar.ks"),
            "pkg/schema/bar/test.ks" => include_str!("../fragments/bar_enum.ks"),
        }
    },
    assertions: |_, ctx: CompileCtx| {
        assert_eq!(ctx.type_registry().all_types().len(), 1);
    }
}

compiler_test! {
    id: compile_external_path_dependency,
    name: "Smoke Package with External Path Dependency",
    purpose: "Test compilation with a path-based external dependency",
    expect_pass: true,
    tags: vec![Tag::Smoke, Tag::Dependencies],
    root: "pkg",
    memory: || {
        memory! {
            "abc-corp/schema.toml" => include_str!("../fragments/abc_corp_manifest.toml"),
            "abc-corp/schema/lib.ks" => include_str!("../fragments/abc_corp_lib.ks"),
            "pkg/schema.toml" => include_str!("../fragments/pkg_with_dep_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/use_foo.ks"),
            "pkg/schema/foo/test.ks" => include_str!("../fragments/bar_with_external_type.ks"),
        }
    },
    assertions: |harness, _: CompileCtx| {
        harness.assert_lockfile_contains("abc-corp");
    }
}

compiler_test! {
    id: compile_transitive_dependencies,
    name: "Nested Path Dependencies (Transitive)",
    purpose: "Test transitive dependency resolution",
    expect_pass: true,
    tags: vec![Tag::Smoke, Tag::Dependencies],
    root: "top",
    memory: || {
        memory! {
            "base/schema.toml" => include_str!("../fragments/base_manifest.toml"),
            "base/schema/lib.ks" => include_str!("../fragments/base_lib.ks"),
            "middle/schema.toml" => include_str!("../fragments/middle_manifest.toml"),
            "middle/schema/lib.ks" => include_str!("../fragments/middle_lib.ks"),
            "top/schema.toml" => include_str!("../fragments/top_manifest.toml"),
            "top/schema/lib.ks" => include_str!("../fragments/top_lib.ks"),
        }
    },
    assertions: |harness, _: CompileCtx| {
        harness.assert_lockfile_contains("middle");
        harness.assert_lockfile_contains("base");
    }
}

compiler_test! {
    id: compile_diamond_dependency,
    name: "Diamond Dependency Resolution",
    purpose: "Test multiple dependencies with shared transitive dependency",
    expect_pass: true,
    tags: vec![Tag::Smoke, Tag::Dependencies],
    root: "app",
    memory: || {

        memory! {
            "common/schema.toml" => include_str!("../fragments/common_manifest.toml"),
            "common/schema/lib.ks" => include_str!("../fragments/common_lib.ks"),
            "lib-a/schema.toml" => include_str!("../fragments/lib_a_manifest.toml"),
            "lib-a/schema/lib.ks" => include_str!("../fragments/lib_a_lib.ks"),
            "lib-b/schema.toml" => include_str!("../fragments/lib_b_manifest.toml"),
            "lib-b/schema/lib.ks" => include_str!("../fragments/lib_b_lib.ks"),
            "app/schema.toml" => include_str!("../fragments/app_manifest.toml"),
            "app/schema/lib.ks" => include_str!("../fragments/app_lib.ks"),
        }
    },
    assertions: |harness, _: CompileCtx| {
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

compiler_test! {
    id: compile_version_pruning,
    name: "Version Compatibility - Multiple Compatible Versions",
    purpose: "Test version pruning keeps highest compatible version",
    expect_pass: true,
    tags: vec![Tag::Smoke, Tag::VersionResolution],
    root: "pkg",
    memory: || {
        memory! {
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
        }
    },
    assertions: |harness, _: CompileCtx| {
        harness.assert_lockfile_written();
        harness.assert_lockfile_contains("lib");
    }
}

compiler_test! {
    id: compile_string_enum,
    name: "Enum with String Values",
    purpose: "Test string-based enum compilation",
    expect_pass: true,
    tags: vec![Tag::Smoke, Tag::Soundness],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/string_enum.ks"),
        }
    },
    assertions: |harness, _: CompileCtx| {
        harness.assert_lockfile_written();
    }
}

compiler_test! {
    id: compile_oneof_mixed_types,
    name: "OneOf with Multiple Types",
    purpose: "Test union type compilation with mixed types",
    expect_pass: true,
    tags: vec![Tag::Smoke, Tag::Soundness],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/oneof_mixed.ks"),
        }
    },
    assertions: |harness, _: CompileCtx| {
        harness.assert_lockfile_written();
    }
}
