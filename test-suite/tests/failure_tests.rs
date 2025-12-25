use kintsu_fs::memory;
use kintsu_test_macros::compiler_test;
use kintsu_test_suite::*;

compiler_test! {
    id: compile_fail_missing_dependency,
    name: "Missing Dependency",
    purpose: "Verify compilation fails when a used dependency is not declared",
    expect_pass: false,
    tags: vec![Tag::Validations],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/missing_dep_use.ks"),
        }
    },
    assertions: |_, err: kintsu_parser::Error| {
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("external_pkg")
                || err_msg.contains("not found")
                || err_msg.contains("dependency"),
            "Error should mention missing dependency: {}",
            err_msg
        );
    }
}

compiler_test! {
    ignore: "currently failing",
    id: compile_fail_circular_dependency,
    name: "Circular Dependency",
    purpose: "Detect and reject circular dependency chains",
    expect_pass: false,
    tags: vec![Tag::Validations],
    root: "pkg-a",
    memory: || {
        memory! {
            "pkg-a/schema.toml" => r#"version = "v1"
[package]
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
        }
    },
    assertions: |_, err: kintsu_parser::Error| {
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("circular") || err_msg.contains("cycle"),
            "Error should mention circular dependency: {}",
            err_msg
        );
    }
}

compiler_test! {
    id: compile_fail_invalid_checksum,
    name: "Invalid Lockfile Checksum",
    purpose: "Verify compilation detects modified dependencies and regenerates lockfile",
    expect_pass: true,
    tags: vec![Tag::Lockfile],
    root: "pkg",
    memory: || {
        memory! {
            "dep/schema.toml" => include_str!("../fragments/dep_manifest.toml"),
            "dep/schema/lib.ks" => include_str!("../fragments/dep_lib.ks"),
            "pkg/schema.toml" => r#"version = "v1"
[package]
name = "pkg"
version = "1.0.0"

[dependencies]
dep = { path = "../dep" }
"#,
            "pkg/schema.lock.toml" => r#"# This lockfile has an INCORRECT checksum
version = "v1"

[root]
name = "dep"
version = "0.1.0"
source = { path = "../dep", type = "path" }
checksum = "abcfoobarwrongchecksum"


[packages]

"#,
            "pkg/schema/lib.ks" => "namespace pkg;\nnamespace main { use dep;  type PkgData = dep::data::Data; };",
        }
    },
    assertions: |harness, _: CompileCtx| {
        let write_ops = harness.count_operations("Write");
        assert!(
            write_ops > 0,
            "Should have rewritten lockfile with correct checksum"
        );
    }
}

compiler_test! {
    ignore: "todo: fix the version conflict detection",
    id: compile_fail_version_conflict,
    name: "Version Conflict - Incompatible Requirements",
    purpose: "Detect when dependencies require incompatible versions",
    expect_pass: false,
    tags: vec![Tag::Validations],
    root: "app",
    memory: || {
        memory! {
            "lib/schema.toml" => r#"version = "v1"
            [package]
            name = "lib"
            version = "2.0.0"
        "#,
            "lib/schema/lib.ks" => "namespace lib; namespace bar { struct Item { id: u64 }; };",
            "pkg-a/schema.toml" => r#"version = "v1"
            [package]
            name = "pkg-a"
            version = "1.0.0"

            [dependencies]
            lib = { path = "../lib", version = "^1.0" }
        "#,
            "pkg-a/schema/lib.ks" => "namespace pkg_a; namespace foo { use lib;  type Foo = lib::bar::Item; };",
            "pkg-b/schema.toml" => r#"version = "v1"
            [package]
            name = "pkg-b"
            version = "1.0.0"

            [dependencies]
            lib = { path = "../lib", version = "^2.0" }
        "#,
            "pkg-b/schema/lib.ks" => "namespace pkg_b; namespace baz { use lib; type Baz = lib::bar::Item; };",
            "app/schema.toml" => r#"version = "v1"
            [package]
            name = "app"
            version = "1.0.0"

            [dependencies]
            pkg-a = { path = "../pkg-a" }
            pkg-b = { path = "../pkg-b" }
        "#,
            "app/schema/lib.ks" => "namespace app; namespace foo { use pkg_a; use pkg_b; type App = oneof pkg_a::foo::Foo | pkg_b::baz::Baz; };",
        }
    },
    assertions: |_, err: kintsu_parser::Error| {
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("version")
                || err_msg.contains("conflict")
                || err_msg.contains("incompatible"),
            "Error should mention version conflict: {}",
            err_msg
        );
    }
}

compiler_test! {
    id: compile_fail_missing_namespace,
    name: "Missing Namespace File",
    purpose: "Verify error when imported namespace doesn't exist",
    expect_pass: false,
    tags: vec![Tag::Validations],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/missing_namespace_use.ks"),
        }
    },
    assertions: |_, err: kintsu_parser::Error| {
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("missing_namespace") || err_msg.contains("not found"),
            "Error should mention missing namespace: {}",
            err_msg
        );
    }
}

compiler_test! {
    id: compile_fail_undefined_type,
    name: "Type Reference to Undefined Type",
    purpose: "Catch references to non-existent types",
    expect_pass: false,
    tags: vec![Tag::Validations],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/undefined_type.ks"),
        }
    },
    assertions: |_, err: kintsu_parser::Error| {
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("UndefinedType")
                || err_msg.contains("not found")
                || err_msg.contains("undefined"),
            "Error should mention undefined type: {}",
            err_msg
        );
    }
}

compiler_test! {
    id: compile_fail_duplicate_type,
    name: "Duplicate Type Definition",
    purpose: "Prevent conflicting type definitions in same namespace",
    expect_pass: false,
    tags: vec![Tag::Validations],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/duplicate_type.ks"),
        }
    },
    assertions: |_, err: kintsu_parser::Error| {
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("duplicate")
                || err_msg.contains("Foo")
                || err_msg.contains("already defined"),
            "Error should mention duplicate type: {}",
            err_msg
        );
    }
}

compiler_test! {
    id: compile_fail_invalid_enum_discriminant,
    name: "Invalid Enum Discriminant",
    purpose: "Reject invalid enum values",
    expect_pass: false,
    tags: vec![Tag::Validations],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/invalid_enum_discriminant.ks"),
        }
    },
    assertions: |_, err: kintsu_parser::Error| {
        // Just verify compilation failed as expected
    }
}

compiler_test! {
    id: compile_fail_malformed_manifest,
    name: "Malformed Manifest",
    purpose: "Catch syntax errors in TOML manifests",
    expect_pass: false,
    tags: vec![Tag::Soundness],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/malformed_manifest.toml"),
            "pkg/schema/lib.ks" => "namespace pkg;",
        }
    },
    assertions: |_, err: kintsu_parser::Error| {
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("unclosed table"),
            "Error should mention TOML parse error: {}",
            err_msg
        );
    }
}

compiler_test! {
    id: compile_fail_wrong_import_syntax,
    name: "Import from Same Package with Wrong Path",
    purpose: "Verify imports within package use correct syntax",
    expect_pass: false,
    tags: vec![Tag::Validations],
    root: "pkg",
    memory: || {
        memory! {
            "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
            "pkg/schema/lib.ks" => include_str!("../fragments/wrong_import_syntax.ks"),
            "pkg/schema/internal.ks" => include_str!("../fragments/internal_namespace.ks"),
        }
    },
    assertions: |_, err: kintsu_parser::Error| {
        let err_msg = format!("{:?}", err);
        assert!(
            !err_msg.is_empty(),
            "Should produce an error for incorrect import syntax"
        );
    }
}
