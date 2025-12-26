//! CLI tests for KTR (Type Resolution) errors per ERR-0006.
//!
//! Type resolution errors occur when types cannot be resolved,
//! such as undefined types, unresolved paths, or circular dependencies.
//! All KTR errors require source spans per SPEC-0022.

use kintsu_fs::memory;
use kintsu_test_suite::cli_tests::{CliErrorTest, minimal_manifest};

/// KTR1002: Undefined type
#[tokio::test]
async fn ktr1002_undefined_type() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-ktr1002"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    struct Foo {
        bar: UndefinedType
    };
};
"#,
    };

    let result = CliErrorTest::new("ktr1002_undefined_type")
        .name("Undefined Type")
        .purpose("Verify KTR1002 for undefined type name")
        .expect_error("KTR")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    // The error should mention the undefined type
    assert!(result.stderr.contains("UndefinedType"));

    insta::assert_snapshot!("ktr1002_undefined_type", result.stderr);
}

/// KTR1002: Undefined type in separate file
#[tokio::test]
async fn ktr1002_undefined_type_separate_file() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-ktr1002b"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct Handler {
    user: Usr
};
"#,
    };

    let result = CliErrorTest::new("ktr1002_undefined_type_separate_file")
        .name("Undefined Type (typo)")
        .purpose("Verify KTR1002 for undefined type name (typo)")
        .expect_error("KTR")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("ktr1002_undefined_type_separate_file", result.stderr);
}

/// KTR5001: Circular type alias
#[tokio::test]
async fn ktr5001_circular_alias() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-ktr5001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    type A = B;
    type B = C;
    type C = A;
};
"#,
    };

    let result = CliErrorTest::new("ktr5001_circular_alias")
        .name("Circular Type Alias")
        .purpose("Verify KTR5001 for circular type alias chain")
        .expect_error("KTR")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("ktr5001_circular_alias", result.stderr);
}

/// KTR5001/KTY5001: Circular struct dependency
#[tokio::test]
async fn ktr_circular_struct_dependency() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-ktr-circular"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct A {
    b: B
};

struct B {
    a: A
};
"#,
    };

    let result = CliErrorTest::new("ktr_circular_struct_dependency")
        .name("Circular Struct Dependency")
        .purpose("Verify KTR/KTY error for circular struct references")
        .expect_error("K") // May be KTR or KTY
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("ktr_circular_struct_dependency", result.stderr);
}

/// KTR: Undefined import - using a type that doesn't exist in imported namespace
#[tokio::test]
async fn ktr_undefined_import() {
    let fs = memory! {
        "dep/schema.toml" => r#"version = "v1"
[package]
name = "dep"
version = "1.0.0"
"#,
        "dep/schema/lib.ks" => "namespace dep;\nnamespace types { struct RealType { value: str }; };",
        "pkg/schema.toml" => r#"version = "v1"
[package]
name = "test-pkg"
version = "1.0.0"

[dependencies]
dep = { path = "../dep" }
"#,
        "pkg/schema/lib.ks" => r#"namespace pkg;
use dep;

namespace types {
    struct Foo {
        item: dep::types::NonExistentType
    };
};
"#,
    };

    let result = CliErrorTest::new("ktr_undefined_import")
        .name("Undefined Import")
        .purpose("Verify KTR error when referencing non-existent type from import")
        .expect_error("K") // May be KTR or KNS
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("ktr_undefined_import", result.stderr);
}
