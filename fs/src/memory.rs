use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::{Error, FileSystem, Result};
use std::{
    ffi::OsString,
    path::{Component, MAIN_SEPARATOR},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FsOperation {
    Read {
        path: PathBuf,
    },
    ReadToString {
        path: PathBuf,
    },
    Write {
        path: PathBuf,
        size: usize,
    },
    FindGlob {
        include: Vec<String>,
        exclude: Vec<String>,
        found: usize,
    },
    ExistsSync {
        path: PathBuf,
        exists: bool,
    },
}

fn de_with_utf<'de, D>(deserializer: D) -> std::result::Result<HashMap<PathBuf, Bytes>, D::Error>
where
    D: serde::Deserializer<'de>, {
    let map: HashMap<PathBuf, String> = HashMap::deserialize(deserializer)?;
    Ok(map
        .into_iter()
        .map(|(k, v)| (k, Bytes::from(v.into_bytes())))
        .collect())
}

fn ser_with_utf<S>(
    orig: &DashMap<PathBuf, Bytes>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer, {
    let map: HashMap<PathBuf, String> = orig
        .iter()
        .map(|ref_multi| {
            (
                ref_multi.key().clone(),
                String::from_utf8_lossy(ref_multi.value().as_ref()).into_owned(),
            )
        })
        .collect();
    map.serialize(serializer)
}

#[allow(dead_code)]
#[cfg(feature = "api")]
#[derive(utoipa::ToSchema)]
struct MemoryFileSystemSerdeHelper {
    files: HashMap<String, String>,
}

#[cfg_attr(feature = "db", derive(sea_orm::prelude::FromJsonQueryResult))]
#[derive(Clone, Debug)]
pub struct MemoryFileSystem {
    files: Arc<DashMap<PathBuf, Bytes>>,

    pattern_cache: Arc<Mutex<HashMap<String, glob::Pattern>>>,

    #[cfg(feature = "fs-test")]
    operations: Arc<Mutex<Vec<FsOperation>>>,
}

impl PartialEq for MemoryFileSystem {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        let self_keys: Vec<_> = self
            .files
            .iter()
            .map(|entry| entry.key().clone())
            .collect();
        let other_keys: Vec<_> = other
            .files
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        if self_keys.len() != other_keys.len() {
            return false;
        }

        for key in &self_keys {
            let self_value = self.files.get(key);
            let other_value = other.files.get(key);

            match (self_value, other_value) {
                (Some(sv), Some(ov)) => {
                    if sv.value() != ov.value() {
                        return false;
                    }
                },
                _ => return false,
            }
        }

        true
    }
}

impl Eq for MemoryFileSystem {}

#[cfg(feature = "api")]
impl utoipa::PartialSchema for MemoryFileSystem {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        MemoryFileSystemSerdeHelper::schema()
    }
}

#[cfg(feature = "api")]
impl utoipa::ToSchema for MemoryFileSystem {}

impl serde::Serialize for MemoryFileSystem {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer, {
        ser_with_utf(&self.files, serializer)
    }
}

impl<'de> serde::Deserialize<'de> for MemoryFileSystem {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>, {
        let files_map = de_with_utf(deserializer)?;
        let files = Arc::new(DashMap::new());
        for (k, v) in files_map {
            files.insert(k, v);
        }
        Ok(Self {
            files,
            pattern_cache: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "fs-test")]
            operations: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

fn remove_relative(path: &Path) -> PathBuf {
    let mut prefix: Option<OsString> = None;
    let mut has_root = false;
    let mut stack: Vec<OsString> = Vec::new();

    for comp in path.components() {
        match comp {
            Component::Prefix(p) => prefix = Some(p.as_os_str().to_os_string()),
            Component::RootDir => has_root = true,
            Component::CurDir => {},
            Component::ParentDir => {
                if let Some(last) = stack.pop() {
                    let _ = last;
                } else if prefix.is_some() || has_root {
                } else {
                    stack.push(OsString::from(".."));
                }
            },
            Component::Normal(s) => stack.push(s.to_os_string()),
        }
    }

    let mut out = PathBuf::new();
    if let Some(p) = prefix {
        out.push(p);
    }
    if has_root {
        // push the root separator ("/" on unix, "\" on windows)
        out.push(MAIN_SEPARATOR.to_string());
    }
    for seg in stack {
        out.push(seg);
    }

    if out.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        out
    }
}

impl MemoryFileSystem {
    pub fn new() -> Self {
        Self {
            files: Arc::new(DashMap::new()),
            pattern_cache: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "fs-test")]
            operations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_files(
        files: impl IntoIterator<Item = (impl Into<PathBuf>, impl Into<Vec<u8>>)>
    ) -> Self {
        let map = DashMap::new();
        for (path, contents) in files {
            map.insert(path.into(), Bytes::from(contents.into()));
        }
        Self {
            files: Arc::new(map),
            pattern_cache: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "fs-test")]
            operations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_file(
        &self,
        path: impl Into<PathBuf>,
        contents: impl AsRef<[u8]>,
    ) {
        self.files
            .insert(path.into(), Bytes::from(contents.as_ref().to_vec()));
    }

    pub fn remove_file(
        &self,
        path: &Path,
    ) -> bool {
        self.files.remove(path).is_some()
    }

    pub fn clear(&self) {
        self.files.clear();
        #[cfg(feature = "fs-test")]
        self.clear_operations();
    }

    pub fn list_files(&self) -> Vec<PathBuf> {
        self.files
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn get_file_content(
        &self,
        path: &Path,
    ) -> Option<Vec<u8>> {
        self.files
            .get(&remove_relative(path))
            .map(|entry| entry.value().to_vec())
    }

    #[cfg(feature = "fs-test")]
    pub fn operations(&self) -> Vec<FsOperation> {
        let ops = self.operations.lock().unwrap();
        ops.clone()
    }

    #[cfg(feature = "fs-test")]
    pub fn clear_operations(&self) {
        let mut ops = self.operations.lock().unwrap();
        ops.clear()
    }

    #[cfg(feature = "fs-test")]
    fn track_operation(
        &self,
        op: FsOperation,
    ) {
        let mut ops = self.operations.lock().unwrap();
        ops.push(op);
    }

    #[cfg(feature = "fs-test")]
    pub fn debug_print_files(&self) {
        for entry in self.files.iter() {
            let (path, contents) = (entry.key(), entry.value());
            println!("File: {} ({} bytes)", path.display(), contents.len());
            println!("```\n{}\n```", String::from_utf8_lossy(contents.as_ref()));
        }
    }

    /// Writes all in-memory files to the physical filesystem at the specified root path.
    /// Creates directories as needed.
    pub fn danger_write_to_physical(
        &self,
        root_path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        for entry in self.files.iter() {
            let (path, contents) = (entry.key(), entry.value());
            let full_path = root_path.as_ref().join(path);
            if let Some(parent) = full_path.parent()
                && !parent.exists()
            {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(full_path, contents.as_ref())?;
        }
        Ok(())
    }

    pub fn merge(
        root_path: impl AsRef<Path>,
        many: Vec<MemoryFileSystem>,
    ) -> Self {
        let root_path = root_path.as_ref();
        let merged = MemoryFileSystem::new();
        for fs in many {
            for entry in fs.files.iter() {
                merged.files.insert(
                    root_path.join(entry.key()).to_path_buf(),
                    entry.value().clone(),
                );
            }
        }
        merged
    }

    pub async fn extract_from<Fs: crate::FileSystem + Send + Sync>(
        fs: &Fs,
        root_path: impl AsRef<Path>,
        include: &[impl std::fmt::Display],
        exclude: &[impl std::fmt::Display],
    ) -> std::result::Result<Self, Error> {
        let root_path = root_path.as_ref();
        let memory_fs = MemoryFileSystem::new();

        let include = include
            .iter()
            .map(|s| format!("{}{}", root_path.display(), s))
            .collect::<Vec<_>>();

        let exclude = exclude
            .iter()
            .map(|s| format!("{}{}", root_path.display(), s))
            .collect::<Vec<_>>();

        let all_files = fs.find_glob(&include, &exclude)?;

        for file_path in all_files {
            let relative_path = file_path
                .strip_prefix(root_path)
                .unwrap_or(&file_path);
            let content = fs.read(&file_path).await?;
            memory_fs
                .files
                .insert(relative_path.to_path_buf(), Bytes::from(content));
        }
        Ok(memory_fs)
    }
}

impl Default for MemoryFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for MemoryFileSystem {
    fn exists_sync(
        &self,
        path: &Path,
    ) -> bool {
        let exists = self
            .files
            .contains_key(&remove_relative(path));
        #[cfg(feature = "fs-test")]
        self.track_operation(FsOperation::ExistsSync {
            path: path.to_path_buf(),
            exists,
        });
        exists
    }

    fn find_glob(
        &self,
        include: &[String],
        exclude: &[String],
    ) -> Result<Vec<PathBuf>> {
        let mut pattern_cache = self.pattern_cache.lock().unwrap();

        let include_patterns: Result<Vec<glob::Pattern>> = include
            .iter()
            .map(|pattern| {
                if let Some(compiled) = pattern_cache.get(pattern) {
                    Ok(compiled.clone())
                } else {
                    let normalized = remove_relative(&PathBuf::from(pattern))
                        .display()
                        .to_string();
                    let compiled = glob::Pattern::new(&normalized)?;
                    pattern_cache.insert(pattern.clone(), compiled.clone());
                    Ok(compiled)
                }
            })
            .collect();

        let exclude_patterns: Result<Vec<glob::Pattern>> = exclude
            .iter()
            .map(|pattern| {
                if let Some(compiled) = pattern_cache.get(pattern) {
                    Ok(compiled.clone())
                } else {
                    let normalized = remove_relative(&PathBuf::from(pattern))
                        .display()
                        .to_string();
                    let compiled = glob::Pattern::new(&normalized)?;
                    pattern_cache.insert(pattern.clone(), compiled.clone());
                    Ok(compiled)
                }
            })
            .collect();

        drop(pattern_cache);

        let include_patterns = include_patterns?;
        let exclude_patterns = exclude_patterns?;

        let mut results = Vec::new();

        for entry in self.files.iter() {
            let path = entry.key();
            let normalized_path = remove_relative(path);

            let matches_include = include.is_empty()
                || include_patterns
                    .iter()
                    .any(|pattern| pattern.matches_path(&normalized_path));

            let matches_exclude = exclude_patterns
                .iter()
                .any(|pattern| pattern.matches_path(&normalized_path));

            if matches_include && !matches_exclude {
                results.push(path.clone());
            }
        }

        results.sort();

        #[cfg(feature = "fs-test")]
        self.track_operation(FsOperation::FindGlob {
            include: include.to_vec(),
            exclude: exclude.to_vec(),
            found: results.len(),
        });

        Ok(results)
    }

    fn read(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + Send + Sync>> {
        let files = self.files.clone();

        #[cfg(feature = "fs-test")]
        let operations = self.operations.clone();

        let path = remove_relative(path);
        Box::pin(async move {
            let result = files
                .get(&path)
                .map(|entry| entry.value().to_vec())
                .ok_or_else(|| {
                    Error::IoError(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("File not found: {}", path.display()),
                    ))
                });

            #[cfg(feature = "fs-test")]
            if result.is_ok() {
                let mut ops = operations.lock().unwrap();
                ops.push(FsOperation::Read { path: path.clone() });
            }

            result
        })
    }

    fn read_to_string(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + Sync>> {
        let files = self.files.clone();

        #[cfg(feature = "fs-test")]
        let operations = self.operations.clone();

        let path = remove_relative(path);
        Box::pin(async move {
            let bytes = files
                .get(&path)
                .map(|entry| entry.value().clone())
                .ok_or_else(|| {
                    Error::IoError(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("File not found: {}", path.display()),
                    ))
                })?;

            let result = String::from_utf8(bytes.to_vec()).map_err(|e| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid UTF-8: {}", e),
                ))
            });

            #[cfg(feature = "fs-test")]
            if result.is_ok() {
                let mut ops = operations.lock().unwrap();
                ops.push(FsOperation::ReadToString { path: path.clone() });
            }

            result
        })
    }

    fn read_to_string_sync(
        &self,
        path: &Path,
    ) -> Result<String> {
        let path = remove_relative(path);
        let bytes = self
            .files
            .get(&path)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("File not found: {}", path.display()),
                ))
            })?;

        let result = String::from_utf8(bytes.to_vec()).map_err(|e| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid UTF-8: {}", e),
            ))
        });

        #[cfg(feature = "fs-test")]
        if result.is_ok() {
            self.track_operation(FsOperation::ReadToString {
                path: path.to_path_buf(),
            });
        }

        result
    }

    fn write(
        &self,
        path: &Path,
        contents: Vec<u8>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + Sync>> {
        let files = self.files.clone();
        let path = remove_relative(path);

        #[cfg(feature = "fs-test")]
        let operations = self.operations.clone();
        #[cfg(feature = "fs-test")]
        let size = contents.len();

        Box::pin(async move {
            files.insert(path.clone(), Bytes::from(contents));

            #[cfg(feature = "fs-test")]
            {
                let mut ops = operations.lock().unwrap();
                ops.push(FsOperation::Write {
                    path: path.clone(),
                    size,
                });
            }

            Ok(())
        })
    }
}

/// ```
/// use kintsu_fs::memory;
///
/// let filesystem = memory! {
///     "dir/file.txt" => "Hello, World!",
///     "dir/data.json" => r#"{"key": "value"}"#,
///     "dir/bytes.bin" => b"binary data",
/// };
/// ```
#[macro_export]
macro_rules! memory {
    ($($path:expr => $contents:expr),* $(,)?) => {{
        let fs = $crate::memory::MemoryFileSystem::new();
        $(
            fs.add_file($path, $contents);
        )*
        fs
    }};
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_memory_fs_basic() {
        let fs = MemoryFileSystem::new();

        assert!(!fs.exists_sync("test.txt".as_ref()));

        fs.add_file("test.txt", b"Hello, World!");
        assert!(fs.exists_sync("test.txt".as_ref()));

        let content = fs
            .read_to_string_sync("test.txt".as_ref())
            .unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_memory_fs_macro() {
        let fs = memory! {
            "dir/file1.txt" => "Hello",
            "dir/file2.txt" => "World",
        };

        assert!(fs.exists_sync("dir/file1.txt".as_ref()));
        assert!(fs.exists_sync("dir/file2.txt".as_ref()));

        let content1 = fs
            .read_to_string_sync("dir/file1.txt".as_ref())
            .unwrap();
        assert_eq!(content1, "Hello");
    }

    #[test]
    fn test_find_glob() {
        let fs = memory! {
            "src/main.rs" => "fn main() {}",
            "src/lib.rs" => "pub fn hello() {}",
            "tests/test.rs" => "#[test] fn test() {}",
            "README.md" => "# Project",
        };

        let results = fs
            .find_glob(&["src/**/*.rs".to_string()], &[])
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .any(|p| p.to_str() == Some("src/main.rs"))
        );
        assert!(
            results
                .iter()
                .any(|p| p.to_str() == Some("src/lib.rs"))
        );

        let results = fs
            .find_glob(&["**/*.rs".to_string()], &["tests/**".to_string()])
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(
            !results
                .iter()
                .any(|p| p.to_str() == Some("tests/test.rs"))
        );
    }
}
