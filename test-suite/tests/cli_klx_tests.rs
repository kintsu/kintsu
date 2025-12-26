//! CLI tests for KLX (Lexical) errors per ERR-0002.
//!
//! Lexical errors occur during tokenization when the lexer encounters
//! invalid characters, malformed literals, or other character-level issues.
//! All KLX errors require source spans per SPEC-0022.

use kintsu_fs::memory;
use kintsu_test_suite::cli_tests::{CliErrorTest, minimal_manifest};

/// KLX0001: Unknown character in source
#[tokio::test]
async fn klx0001_unknown_character() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-klx0001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    struct User {
        name§: str
    };
};
"#,
    };

    let result = CliErrorTest::new("klx0001_unknown_character")
        .name("Unknown Character")
        .purpose("Verify KLX error for invalid characters in source")
        .expect_error("KLX")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("klx0001_unknown_character", result.stderr);
}

/// KLX0002: Invalid integer literal (overflow)
#[tokio::test]
async fn klx0002_invalid_integer_literal() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-klx0002"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    enum TooBig {
        Overflow = 99999999999999999999999999
    };
};
"#,
    };

    let result = CliErrorTest::new("klx0002_invalid_integer_literal")
        .name("Invalid Integer Literal")
        .purpose("Verify KLX error for integer overflow")
        .expect_error("KLX")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("klx0002_invalid_integer_literal", result.stderr);
}

/// KLX0005: Unterminated string literal
#[tokio::test]
async fn klx0005_unterminated_string() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-klx0005"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    #[doc = "This is unterminated
    struct User {
        name: str
    };
};
"#,
    };

    let result = CliErrorTest::new("klx0005_unterminated_string")
        .name("Unterminated String")
        .purpose("Verify KLX error for unterminated string literals")
        .expect_error("KLX")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("klx0005_unterminated_string", result.stderr);
}

/// KLX9001: General lexing error - missing colon triggers lexer error
#[tokio::test]
async fn klx9001_syntax_missing_colon() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-klx9001"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    struct Foo {
        value str
    };
};
"#,
    };

    let result = CliErrorTest::new("klx9001_syntax_missing_colon")
        .name("Syntax Error - Missing Colon")
        .purpose("Verify KLX9001 for missing colon in field definition")
        .expect_error("KLX")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("klx9001_syntax_missing_colon", result.stderr);
}

/// KLX9001: General lexing error - unexpected token
#[tokio::test]
async fn klx9001_unexpected_token() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-klx9001b"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    struct User name: str };
};
"#,
    };

    let result = CliErrorTest::new("klx9001_unexpected_token")
        .name("Unexpected Token")
        .purpose("Verify KLX9001 for unexpected token (missing brace)")
        .expect_error("KLX")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("klx9001_unexpected_token", result.stderr);
}

/// KLX9001: General lexing error - unexpected EOF
#[tokio::test]
async fn klx9001_unexpected_eof() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-klx9001c"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

namespace types {
    struct User {
        name: str,
"#,
    };

    let result = CliErrorTest::new("klx9001_unexpected_eof")
        .name("Unexpected End of File")
        .purpose("Verify KLX9001 for file ending before complete declaration")
        .expect_error("KLX")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("klx9001_unexpected_eof", result.stderr);
}

/// KLX9001: Special characters that aren't part of the grammar
#[tokio::test]
async fn klx9001_special_char() {
    let fs = memory! {
        "pkg/schema.toml" => minimal_manifest("test-klx9001d"),
        "pkg/schema/lib.ks" => r#"namespace pkg;

struct User {
    name: str,
    email# str
};
"#,
    };

    let result = CliErrorTest::new("klx9001_special_char")
        .name("Special Character in Field")
        .purpose("Verify KLX error for special character (#) in field definition")
        .expect_error("KLX")
        .requires_span(true)
        .with_fs(fs)
        .root("pkg")
        .run_and_assert();

    insta::assert_snapshot!("klx9001_special_char", result.stderr);
}
