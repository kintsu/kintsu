//! CLI tests that verify valid schemas compile without errors.
//!
//! These tests ensure the CLI correctly compiles valid schemas
//! and serves as a baseline for error detection.

use kintsu_fs::memory;
use kintsu_test_suite::cli_tests::{CliErrorTest, minimal_manifest};
use std::path::PathBuf;

/// Valid basic schema compiles successfully
#[tokio::test]
async fn success_basic_schema() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-success-basic"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    struct User {
        id: u64,
        name: str,
        email?: str
    };

    enum Status {
        Active = 1,
        Inactive = 2
    };
};
"#,
    };

    let result = CliErrorTest::new("success_basic_schema")
        .name("Basic Schema Success")
        .purpose("Verify valid basic schema compiles without errors")
        .expect_success()
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    assert!(result.passed, "Valid schema should compile successfully");
}

/// Valid schema with nested namespaces compiles successfully
#[tokio::test]
async fn success_nested_namespaces() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-success-nested"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    struct User {
        id: u64,
        name: str
    };
};
"#,
    };

    let result = CliErrorTest::new("success_nested_namespaces")
        .name("Nested Namespaces Success")
        .purpose("Verify valid schema with nested namespaces compiles successfully")
        .expect_success()
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    assert!(
        result.passed,
        "Valid nested namespace schema should compile successfully"
    );
}

/// Valid schema with operations and errors compiles successfully
#[tokio::test]
async fn success_operations_with_errors() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-success-ops"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    struct User {
        id: u64,
        name: str
    };

    error ApiError {
        NotFound { id: u64 },
        InvalidInput { message: str }
    };

    #[err(ApiError)]
    operation get_user(id: u64) -> User!;

    #[err(ApiError)]
    operation create_user(name: str) -> User!;
};
"#,
    };

    let result = CliErrorTest::new("success_operations_with_errors")
        .name("Operations with Errors Success")
        .purpose("Verify valid schema with operations and error types compiles successfully")
        .expect_success()
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    assert!(
        result.passed,
        "Valid operations schema should compile successfully"
    );
}

/// Valid schema with type expressions compiles successfully
#[tokio::test]
async fn success_type_expressions() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-success-typeexpr"),
        "pkg/schema/lib.ks" => r#"namespace pkg;
use types;
"#,
        "pkg/schema/types.ks" => r#"namespace types;

struct User {
    id: u64,
    name: str,
    email: str,
    avatar?: str
};

type UserBasic = Pick[User, id | name];
type UserContact = Pick[User, email];
type UserPublic = Omit[User, email];
"#,
    };

    let result = CliErrorTest::new("success_type_expressions")
        .name("Type Expressions Success")
        .purpose("Verify valid schema with type expressions compiles successfully")
        .expect_success()
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    assert!(
        result.passed,
        "Valid type expressions schema should compile successfully"
    );
}

/// Valid schema with oneof compiles successfully
#[tokio::test]
async fn success_oneof_schema() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-success-oneof"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    struct Success {
        data: str
    };

    struct Error {
        message: str,
        code: i32
    };

    type Response = oneof Success | Error;
};
"#,
    };

    let result = CliErrorTest::new("success_oneof_schema")
        .name("OneOf Schema Success")
        .purpose("Verify valid schema with oneof compiles successfully")
        .expect_success()
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    assert!(
        result.passed,
        "Valid oneof schema should compile successfully"
    );
}

/// Valid schema with multiple types compiles successfully
#[tokio::test]
async fn success_multi_type_schema() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-success-multitype"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    struct Id {
        value: u64
    };

    enum Status {
        Active = 1,
        Inactive = 2
    };

    struct User {
        id: Id,
        name: str,
        status: Status
    };
};
"#,
    };

    let result = CliErrorTest::new("success_multi_type_schema")
        .name("Multi-Type Schema Success")
        .purpose("Verify valid schema with multiple types compiles successfully")
        .expect_success()
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    assert!(
        result.passed,
        "Valid multi-type schema should compile successfully"
    );
}

/// Integration test - run CLI against actual filesystem
#[tokio::test]
async fn integration_check_command() {
    use kintsu_test_suite::cli_tests::run_check_command;

    let temp_dir = PathBuf::from("./tmp/cli_test_integration_check");

    // Create a minimal valid package
    std::fs::create_dir_all(temp_dir.join("schema")).ok();
    std::fs::write(
        temp_dir.join("schema.toml"),
        minimal_manifest("integration-test"),
    )
    .ok();
    std::fs::write(
        temp_dir.join("schema/lib.ks"),
        "namespace integration_test;\nnamespace types { struct Foo { value: str }; };",
    )
    .ok();

    let output = run_check_command(&temp_dir);

    // Clean up
    std::fs::remove_dir_all(&temp_dir).ok();

    // This should succeed
    assert!(
        output.success(),
        "Integration test failed:\nstdout: {}\nstderr: {}",
        output.stdout,
        output.stderr
    );
}
