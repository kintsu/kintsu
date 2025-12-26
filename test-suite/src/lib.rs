use kintsu_fs::{FileSystem, memory::MemoryFileSystem};
use kintsu_manifests::{config::NewForNamed, lock::Lockfiles};
pub use kintsu_parser::ctx::CompileCtx;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

pub mod cli_tests;
pub mod many;

pub use cli_tests::*;

/// Type of test report for JSONL output.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestReportType {
    /// CLI error test result
    CliTest,
    /// Compile/harness test result
    CompileTest,
}

/// Wrapper for JSONL output with type tag.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TestReport<T> {
    /// Type of test report
    #[serde(rename = "type")]
    pub report_type: TestReportType,
    /// The test result content
    pub test: T,
}

impl<T> TestReport<T> {
    pub fn cli_test(test: T) -> Self {
        Self {
            report_type: TestReportType::CliTest,
            test,
        }
    }

    pub fn compile_test(test: T) -> Self {
        Self {
            report_type: TestReportType::CompileTest,
            test,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum Tag {
    /// Tests that focus on basic functionality and correctness.
    Smoke,

    /// Tests that focus on type system soundness.
    Soundness,

    /// Tests that focus on various aspects of schema and code generation.
    Validations,

    /// Tests that focus on lockfile generation and related functionality.
    Lockfile,

    /// Tests that focus on testing the file system operations.
    FileOperation,

    /// Tests that focus on import resolution and related features.
    Imports,

    /// Tests that focus on namespace handling and related features.
    Namespace,

    /// Tests that focus on dependency resolution and management.
    Dependencies,

    /// Tests that focus on version resolution and related features.
    VersionResolution,

    // types
    //
    /// Tests that focus on operation type resolution.
    Operation,
    /// Tests that focus on error type resolution.
    Error,
    /// Tests that focus on one-of type resolution.
    OneOf,
    /// Tests that focus on union type resolution.
    Union,
    /// Tests that focus on type alias resolution.
    TypeAlias,
    /// Tests that focus on struct type resolution.
    Struct,
    /// Tests that focus on enum type resolution.
    Enum,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, bon::Builder)]
#[serde(rename_all = "snake_case")]
pub struct TestMetadata {
    #[builder(into)]
    pub id: String,

    #[builder(into)]
    pub name: String,

    #[builder(into)]
    pub purpose: String,
    pub expect_pass: bool,
    pub tags: Vec<Tag>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, bon::Builder)]
#[serde(rename_all = "snake_case")]
pub struct TestResult {
    pub fs: MemoryFileSystem,
    pub metadata: TestMetadata,
    pub actual_pass: bool,
    pub matches_expectation: bool,
    pub error_message: Option<String>,
}

impl TestResult {
    /// Write this result to the test-suite.jsonl file with type wrapper
    pub fn write_to_jsonl(&self) {
        use std::{fs::OpenOptions, io::Write, path::PathBuf};

        let jsonl_path = PathBuf::from("./test-suite.jsonl");

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&jsonl_path)
        {
            let report = TestReport::compile_test(self);
            if let Ok(json) = serde_json::to_string(&report) {
                let _ = writeln!(file, "{}", json);
            }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, bon::Builder)]
pub struct TestHarness {
    pub fs: MemoryFileSystem,

    #[builder(into)]
    pub root: String,

    pub metadata: TestMetadata,

    pub result: Option<TestResult>,
}

impl TestHarness {
    pub fn with_metadata(
        fs: MemoryFileSystem,
        id: impl Into<String>,
        name: impl Into<String>,
        purpose: impl Into<String>,
        expect_pass: bool,
        tags: Vec<Tag>,
    ) -> Self {
        kintsu_testing::logging();

        Self {
            fs,
            root: "pkg".to_string(),
            metadata: TestMetadata {
                id: id.into(),
                name: name.into(),
                purpose: purpose.into(),
                expect_pass,
                tags,
            },
            result: None,
        }
    }

    pub fn with_root(
        mut self,
        root: impl Into<String>,
    ) -> Self {
        self.root = root.into();
        self
    }

    pub fn set_root(
        &mut self,
        root: impl Into<String>,
    ) {
        self.root = root.into();
    }

    pub fn add_file(
        &mut self,
        path: impl AsRef<Path>,
        content: impl AsRef<[u8]>,
    ) {
        self.fs
            .add_file(path.as_ref(), content.as_ref());
    }

    pub fn add_text_file(
        &mut self,
        path: impl AsRef<Path>,
        content: impl AsRef<str>,
    ) {
        self.add_file(path, content.as_ref().as_bytes());
    }

    fn handle_meta(
        &mut self,
        result: &Result<CompileCtx, Error>,
    ) {
        let (actual_pass, error_message) = match &result {
            Ok(_) => (true, None),
            Err(e) => (false, Some(format!("{:#?}", e))),
        };

        self.result = Some(TestResult {
            fs: self.fs.clone(),
            metadata: self.metadata.clone(),
            actual_pass,
            matches_expectation: actual_pass == self.metadata.expect_pass,
            error_message,
        });
    }

    fn id(&self) -> &str {
        &self.metadata.id
    }

    fn name(&self) -> &str {
        &self.metadata.name
    }

    fn purpose(&self) -> &str {
        &self.metadata.purpose
    }

    fn expect_pass(&self) -> bool {
        self.metadata.expect_pass
    }

    #[tracing::instrument(skip(self), fields(
        test_id = self.id() ,
        test_name = self.name(),
        test_purpose = self.purpose(),
        test_expect_pass = self.expect_pass(),
        root_package = self.root
    ))]
    pub async fn compile_pass(&mut self) -> CompileCtx {
        self.fs
            .danger_write_to_physical(format!("./tmp/{}", self.id()))
            .unwrap();

        let result = compile_pass(Arc::new(self.fs.clone()), &self.root).await;

        self.handle_meta(&result);

        match result {
            Ok(ctx) => {
                let decl = ctx
                    .emit_declarations()
                    .await
                    .expect("emit declarations");

                self.add_file(
                    "declarations.json",
                    serde_json::to_string(&decl).expect("serialize declarations"),
                );
                self.fs
                    .danger_write_to_physical(format!("./tmp/{}", self.id()))
                    .unwrap();

                ctx
            },
            Err(e) => {
                self.fs.debug_print_files();
                panic!(
                    "Expected compilation to succeed, but got error: {:?}",
                    e.to_report(None, None, None)
                )
            },
        }
    }

    #[tracing::instrument(skip(self), fields(
        test_id = self.id() ,
        test_name = self.name(),
        test_purpose = self.purpose(),
        test_expect_pass = self.expect_pass(),
        root_package = self.root
    ))]
    pub async fn compile_fail(&mut self) -> kintsu_parser::Error {
        self.fs
            .danger_write_to_physical(format!("./tmp/{}", self.id()))
            .unwrap();

        let result = CompileCtx::with_fs(Arc::new(self.fs.clone()), self.root.clone()).await;

        self.handle_meta(&result);

        match result {
            Err(err) => err,
            Ok(..) => {
                self.fs.debug_print_files();
                panic!("Expected compilation to fail, but it succeeded")
            },
        }
    }

    pub fn result(&self) -> Option<&TestResult> {
        self.result.as_ref()
    }

    pub fn operations(&self) -> Vec<kintsu_fs::memory::FsOperation> {
        self.fs.operations()
    }

    pub fn clear_operations(&mut self) {
        self.fs.clear_operations();
    }

    pub fn lockfile_path(&self) -> PathBuf {
        let p = PathBuf::from(format!("{}/{}", self.root, Lockfiles::NAME));
        tracing::info!("Expect lockfile path: {}", p.display());
        p
    }

    pub fn lockfile_exists(&self) -> bool {
        let lockfile_path = self.lockfile_path();

        self.fs.exists_sync(&lockfile_path)
    }

    pub fn read_lockfile(&self) -> Option<String> {
        let lockfile_path = self.lockfile_path();
        self.fs
            .get_file_content(&lockfile_path)
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    pub fn assert_lockfile_written(&self) {
        assert!(
            self.lockfile_exists(),
            "Expected lockfile to be written, but it doesn't exist"
        );
    }

    pub fn assert_no_lockfile(&self) {
        assert!(
            !self.lockfile_exists(),
            "Expected no lockfile, but one exists"
        );
    }

    pub fn assert_lockfile_contains(
        &self,
        package_name: &str,
    ) {
        let content = self
            .read_lockfile()
            .expect("Lockfile should exist to check contents");
        // ok this is a pretty shaky test method tbh but as-is the lockfile api needs stability b/f static tests
        assert!(
            content.contains(package_name),
            "Expected lockfile to contain '{}', but it doesn't.\nLockfile:\n{}",
            package_name,
            content
        );
    }

    #[allow(unused)]
    pub fn assert_operation_occurred(
        &self,
        op_type: &str,
    ) {
        // todo!("Implement assert_operation_occurred");
    }

    pub fn count_operations(
        &self,
        op_type: &str,
    ) -> usize {
        self.operations()
            .iter()
            .filter(|op| format!("{:?}", op).contains(op_type))
            .count()
    }
}

impl TestMetadata {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        tags: Vec<Tag>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            purpose: String::new(),
            expect_pass: true,
            tags,
        }
    }

    pub fn purpose(
        mut self,
        purpose: impl Into<String>,
    ) -> Self {
        self.purpose = purpose.into();
        self
    }

    pub fn expect_fail(mut self) -> Self {
        self.expect_pass = false;
        self
    }

    pub fn tag(
        mut self,
        tags: Vec<Tag>,
    ) -> Self {
        self.tags = tags;
        self
    }
}

pub async fn compile_pass(
    fs: Arc<dyn FileSystem>,
    root: &str,
) -> Result<CompileCtx, kintsu_parser::Error> {
    kintsu_testing::logging();
    match CompileCtx::with_fs(fs, root).await {
        Ok(ctx) => {
            ctx.finalize().await?;
            ctx.emit_declarations().await?;
            Ok(ctx)
        },
        Err(e) => Err(e),
    }
}

pub async fn compile_fail(
    fs: Arc<dyn FileSystem>,
    root: &str,
) -> Result<kintsu_parser::Error, ()> {
    kintsu_testing::logging();
    match CompileCtx::with_fs(fs, root).await {
        Ok(_) => Err(()),
        Err(e) => Ok(e),
    }
}

pub use kintsu_fs::memory::{FsOperation, MemoryFileSystem as MemFs};
pub use kintsu_parser::Error;

static TESTS: OnceLock<Mutex<TestCollector>> = OnceLock::new();

impl Drop for TestHarness {
    fn drop(&mut self) {
        TESTS
            .get_or_init(|| Mutex::new(TestCollector { tests: vec![] }))
            .lock()
            .unwrap()
            .tests
            .push(self.result.clone().unwrap())
    }
}

struct TestCollector {
    tests: Vec<TestResult>,
}

impl TestCollector {
    pub fn write_report(
        &self,
        path: &Path,
    ) {
        serde_jsonlines::append_json_lines(path, &self.tests).unwrap();
    }
}

#[ctor::dtor]
fn dtor() {
    TESTS
        .get_or_init(|| Mutex::new(TestCollector { tests: vec![] }))
        .lock()
        .unwrap()
        .write_report(&PathBuf::from("./test-suite.jsonl"));
}
