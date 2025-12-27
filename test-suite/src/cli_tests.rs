use crate::{Tag, TestMetadata, TestReport, TestResult};
use kintsu_fs::memory::MemoryFileSystem;
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::Mutex,
};

// Global test results collector for JSONL output
static TEST_RESULTS: Mutex<Vec<CliTestResult>> = Mutex::new(Vec::new());

/// Result of a CLI error test.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CliTestResult {
    /// Test metadata
    pub metadata: TestMetadata,
    /// Exit code from CLI
    pub exit_code: i32,
    /// Whether test passed expectation
    pub passed: bool,
    /// Expected error code (if any)
    pub expected_error_code: Option<String>,
    /// Actual error code found (if any)
    pub actual_error_code: Option<String>,
    /// Whether a source span was expected per SPEC-0022
    pub expected_span: bool,
    /// Whether a source span was actually present in output
    pub has_source_span: bool,
    /// Whether span expectation was met
    pub span_matches_expectation: bool,
    /// Stdout content
    pub stdout: String,
    /// Stderr content (error message)
    pub stderr: String,
    /// Combined output for error_message field
    pub error_message: String,
}

impl CliTestResult {
    /// Write this result to the test-suite.jsonl file with type wrapper
    pub fn write_to_jsonl(&self) {
        let jsonl_path = PathBuf::from("./test-suite.jsonl");

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&jsonl_path)
        {
            let report = TestReport::cli_test(self);
            if let Ok(json) = serde_json::to_string(&report) {
                let _ = writeln!(file, "{}", json);
            }
        }
    }

    /// Convert to TestResult for compatibility
    pub fn to_test_result(
        &self,
        fs: MemoryFileSystem,
    ) -> TestResult {
        TestResult {
            fs,
            metadata: self.metadata.clone(),
            actual_pass: self.exit_code == 0,
            matches_expectation: self.passed,
            error_message: if self.stderr.is_empty() {
                None
            } else {
                Some(self.stderr.clone())
            },
        }
    }
}

/// Builder for CLI error tests.
#[derive(Debug, Clone)]
pub struct CliErrorTest {
    /// Test ID (used for snapshots and tmp directory)
    pub id: String,
    /// Human-readable test name
    pub name: String,
    /// Purpose/description of what this tests
    pub purpose: String,
    /// Expected error code prefix (e.g., "KTR", "KTR1002")
    pub expected_error_code: Option<String>,
    /// Whether the test expects the command to fail
    pub expect_failure: bool,
    /// Whether this expects a warning (success + code present)
    pub expect_warning: bool,
    /// Tags for categorization
    pub tags: Vec<Tag>,
    /// Memory filesystem with test files
    pub fs: MemoryFileSystem,
    /// Root package directory within the filesystem
    pub root: String,
    /// Whether this error should have source spans per SPEC-0022
    pub requires_span: bool,
}

impl CliErrorTest {
    /// Create a new CLI error test.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            purpose: String::new(),
            expected_error_code: None,
            expect_failure: true,
            expect_warning: false,
            tags: vec![Tag::Validations],
            fs: MemoryFileSystem::new(),
            root: "pkg".to_string(),
            requires_span: false,
        }
    }

    /// Set the test name.
    pub fn name(
        mut self,
        name: impl Into<String>,
    ) -> Self {
        self.name = name.into();
        self
    }

    /// Set the test purpose.
    pub fn purpose(
        mut self,
        purpose: impl Into<String>,
    ) -> Self {
        self.purpose = purpose.into();
        self
    }

    /// Set expected error code (prefix or full code).
    pub fn expect_error(
        mut self,
        code: impl Into<String>,
    ) -> Self {
        self.expected_error_code = Some(code.into());
        self.expect_failure = true;
        self.expect_warning = false;
        self
    }

    /// Expect a warning code (compilation succeeds but warning emitted).
    pub fn expect_warning(
        mut self,
        code: impl Into<String>,
    ) -> Self {
        self.expected_error_code = Some(code.into());
        self.expect_failure = false;
        self.expect_warning = true;
        self
    }

    /// Mark this as expecting success (no error).
    pub fn expect_success(mut self) -> Self {
        self.expected_error_code = None;
        self.expect_failure = false;
        self
    }

    /// Add tags for categorization.
    pub fn tags(
        mut self,
        tags: impl IntoIterator<Item = Tag>,
    ) -> Self {
        self.tags = tags.into_iter().collect();
        self
    }

    /// Set the memory filesystem.
    pub fn with_fs(
        mut self,
        fs: MemoryFileSystem,
    ) -> Self {
        self.fs = fs;
        self
    }

    /// Set the root package directory.
    pub fn root(
        mut self,
        root: impl Into<String>,
    ) -> Self {
        self.root = root.into();
        self
    }

    /// Indicate this error requires source spans per SPEC-0022.
    pub fn requires_span(
        mut self,
        requires: bool,
    ) -> Self {
        self.requires_span = requires;
        self
    }

    /// Add a file to the test filesystem.
    pub fn file(
        self,
        path: impl AsRef<Path>,
        content: impl AsRef<str>,
    ) -> Self {
        self.fs
            .add_file(path.as_ref(), content.as_ref().as_bytes());
        self
    }

    /// Add manifest file with standard content.
    pub fn with_manifest(
        self,
        name: &str,
    ) -> Self {
        let root = self.root.clone();
        let manifest = format!(
            r#"version = "v1"

[package]
name = "{}"
version = "1.0.0"
"#,
            name
        );
        self.file(format!("{}/schema.toml", root), manifest)
    }

    /// Add lib.ks with given content.
    pub fn with_lib_ks(
        self,
        content: impl Into<String>,
    ) -> Self {
        let root = self.root.clone();
        self.file(format!("{}/schema/lib.ks", root), content.into())
    }

    /// Run the test and return structured results.
    pub fn run(&self) -> CliTestResult {
        // Write files to temp directory
        let temp_dir = PathBuf::from(format!("./tmp/cli_test_{}", self.id));
        let _ = std::fs::remove_dir_all(&temp_dir);

        self.fs
            .danger_write_to_physical(&temp_dir)
            .expect("write test files to disk");

        let root_path = temp_dir.join(&self.root);
        let output = run_cli(&["check", "-d", &root_path.to_string_lossy()]);

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
        let combined = format!("{}\n{}", stdout, stderr);

        // Check for error code
        let actual_error_code = extract_error_code(&combined);

        // Check for source span (miette format with ╭─[file:line:col])
        let has_source_span = stderr.contains("╭─[") || stderr.contains("-->");

        // Check if span expectation is met
        let span_matches_expectation = if self.requires_span {
            has_source_span
        } else {
            true // If not required, always matches
        };

        // Determine if test passed
        let code_matches = match (&self.expected_error_code, &actual_error_code) {
            (Some(expected), Some(actual)) => actual.starts_with(expected),
            (None, None) => true,
            (None, Some(_)) => !self.expect_failure,
            (Some(_), None) => false,
        };

        let exit_matches = if self.expect_warning {
            exit_code == 0
        } else if self.expect_failure {
            exit_code != 0
        } else {
            exit_code == 0
        };

        let passed = code_matches && exit_matches;

        let result = CliTestResult {
            metadata: TestMetadata {
                id: self.id.clone(),
                name: self.name.clone(),
                purpose: self.purpose.clone(),
                expect_pass: !self.expect_failure,
                tags: self.tags.clone(),
            },
            exit_code,
            passed,
            expected_error_code: self.expected_error_code.clone(),
            actual_error_code,
            expected_span: self.requires_span,
            has_source_span,
            span_matches_expectation,
            stdout,
            stderr: stderr.clone(),
            error_message: if stderr.is_empty() {
                combined
            } else {
                stderr
            },
        };

        // Write to JSONL
        result.write_to_jsonl();

        result
    }

    /// Run and assert the test passes expectations.
    pub fn run_and_assert(&self) -> CliTestResult {
        let result = self.run();

        if self.expect_warning {
            assert!(
                result.exit_code == 0,
                "Expected warning (exit 0) but CLI failed with code {}.\nTest: {}\nstderr: {}",
                result.exit_code,
                self.id,
                result.stderr
            );
        } else if self.expect_failure {
            assert!(
                result.exit_code != 0,
                "Expected CLI to fail but it succeeded.\nTest: {}\nstdout: {}\nstderr: {}",
                self.id,
                result.stdout,
                result.stderr
            );
        } else {
            assert!(
                result.exit_code == 0,
                "Expected CLI to succeed but it failed with code {}.\nTest: {}\nstdout: {}\nstderr: {}",
                result.exit_code,
                self.id,
                result.stdout,
                result.stderr
            );
        }

        if let Some(expected) = &self.expected_error_code {
            assert!(
                result
                    .actual_error_code
                    .as_ref()
                    .is_some_and(|a| a.starts_with(expected)),
                "Expected error code starting with '{}', got {:?}.\nTest: {}\nstderr: {}",
                expected,
                result.actual_error_code,
                self.id,
                result.stderr
            );
        }

        // Enforce span requirements - this is a hard line per SPEC-0022
        if self.requires_span {
            assert!(
                result.has_source_span,
                "SPEC-0022 VIOLATION: Error {} requires source span but none was present.\n\
                Test: {}\n\
                Error code: {:?}\n\
                stderr:\n{}",
                self.expected_error_code
                    .as_deref()
                    .unwrap_or("(unknown)"),
                self.id,
                result.actual_error_code,
                result.stderr
            );
        }

        result
    }
}

/// Run the kintsu CLI with arguments.
fn run_cli(args: &[&str]) -> Output {
    let binary = find_kintsu_binary();

    Command::new(&binary)
        .args(args)
        .env("NO_COLOR", "1")
        .env("TERM", "dumb")
        .env("LOG_LEVEL", "off")
        .output()
        .expect("failed to execute kintsu command")
}

/// Find the kintsu binary.
fn find_kintsu_binary() -> PathBuf {
    for path in &[
        "../target/debug/kintsu",
        "../target/release/kintsu",
        "./target/debug/kintsu",
        "./target/release/kintsu",
    ] {
        let p = PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("kintsu")
}

/// Strip ANSI escape codes.
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Extract error code from output (e.g., "KTR1002").
fn extract_error_code(output: &str) -> Option<String> {
    // Look for pattern like "KXX####" at start of line
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('K') && trimmed.len() >= 4 {
            // Check if it looks like an error code
            let code: String = trimmed
                .chars()
                .take_while(|c| c.is_alphanumeric())
                .collect();
            if code.len() >= 4 && code.len() <= 8 {
                return Some(code);
            }
        }
    }
    None
}

/// Output from running the check command.
#[derive(Debug, Clone)]
pub struct CheckOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CheckOutput {
    /// Returns true if the command succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Run the kintsu check command in a directory.
pub fn run_check_command(dir: &Path) -> CheckOutput {
    let output = run_cli(&["check", "-d", &dir.to_string_lossy()]);

    CheckOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: strip_ansi_codes(&String::from_utf8_lossy(&output.stderr)),
    }
}

/// Minimal manifest helper.
pub fn minimal_manifest(name: &str) -> String {
    format!(
        r#"version = "v1"

[package]
name = "{}"
version = "1.0.0"
"#,
        name
    )
}

/// Manifest with dependencies helper.
pub fn manifest_with_deps(
    name: &str,
    deps: &[(&str, &str)],
) -> String {
    let mut manifest = format!(
        r#"version = "v1"

[package]
name = "{}"
version = "1.0.0"

[dependencies]
"#,
        name
    );
    for (dep_name, dep_path) in deps {
        manifest.push_str(&format!("{} = {{ path = \"{}\" }}\n", dep_name, dep_path));
    }
    manifest
}

/// Write all collected test results to JSONL file.
pub fn flush_results_to_jsonl() {
    let results = TEST_RESULTS.lock().unwrap();
    let jsonl_path = PathBuf::from("./test-suite.jsonl");

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&jsonl_path)
    {
        for result in results.iter() {
            if let Ok(json) = serde_json::to_string(result) {
                let _ = writeln!(file, "{}", json);
            }
        }
    }
}

// ============================================================================
// Test Generation Macros
// ============================================================================

/// Macro to define a CLI error test case.
///
/// # Example
/// ```ignore
/// cli_test! {
///     id: klx0001_unknown_char,
///     name: "Unknown Character",
///     purpose: "Verify KLX0001 for invalid characters",
///     domain: Lexical,
///     error_code: "KLX",
///     requires_span: true,
///     manifest: "test-pkg",
///     lib_ks: r#"namespace pkg; @invalid struct Foo {};"#,
/// }
/// ```
#[macro_export]
macro_rules! cli_test {
    (
        id: $id:ident,
        name: $name:expr,
        purpose: $purpose:expr,
        domain: $domain:ident,
        error_code: $code:expr,
        requires_span: $span:expr,
        manifest: $manifest:expr,
        lib_ks: $lib:expr
        $(,)?
    ) => {
        #[tokio::test]
        async fn $id() {
            use $crate::cli_tests::{CliErrorTest, minimal_manifest};
            use kintsu_fs::memory;

            let fs = memory! {
                "pkg/schema.toml" => minimal_manifest($manifest),
                "pkg/schema/lib.ks" => $lib,
            };

            let result = CliErrorTest::new(stringify!($id))
                .name($name)
                .purpose($purpose)
                .expect_error($code)
                .requires_span($span)
                .with_fs(fs)
                .root("pkg")
                .run_and_assert();

            insta::assert_snapshot!(stringify!($id), result.stderr);
        }
    };

    // Variant with custom files
    (
        id: $id:ident,
        name: $name:expr,
        purpose: $purpose:expr,
        domain: $domain:ident,
        error_code: $code:expr,
        requires_span: $span:expr,
        files: { $($path:expr => $content:expr),* $(,)? }
        $(,)?
    ) => {
        #[tokio::test]
        async fn $id() {
            use $crate::cli_tests::CliErrorTest;
            use kintsu_fs::memory;

            let fs = memory! {
                $($path => $content),*
            };

            let result = CliErrorTest::new(stringify!($id))
                .name($name)
                .purpose($purpose)
                .expect_error($code)
                .requires_span($span)
                .with_fs(fs)
                .root("pkg")
                .run_and_assert();

            insta::assert_snapshot!(stringify!($id), result.stderr);
        }
    };

    // Success test variant
    (
        id: $id:ident,
        name: $name:expr,
        purpose: $purpose:expr,
        expect_success: true,
        manifest: $manifest:expr,
        lib_ks: $lib:expr
        $(,)?
    ) => {
        #[tokio::test]
        async fn $id() {
            use $crate::cli_tests::{CliErrorTest, minimal_manifest};
            use kintsu_fs::memory;

            let fs = memory! {
                "pkg/schema.toml" => minimal_manifest($manifest),
                "pkg/schema/lib.ks" => $lib,
            };

            let result = CliErrorTest::new(stringify!($id))
                .name($name)
                .purpose($purpose)
                .expect_success()
                .with_fs(fs)
                .root("pkg")
                .run_and_assert();

            // No snapshot for success cases
            assert!(result.passed);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_error_code() {
        assert_eq!(
            extract_error_code("KTR1002\n  × error"),
            Some("KTR1002".into())
        );
        assert_eq!(extract_error_code("  KLX0001"), Some("KLX0001".into()));
        assert_eq!(extract_error_code("no error code here"), None);
    }

    #[test]
    fn test_strip_ansi() {
        let input = "\x1b[31merror\x1b[0m";
        assert_eq!(strip_ansi_codes(input), "error");
    }
}
