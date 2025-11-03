use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, RwLock},
};

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

fn de_with_utf<'de, D>(
    deserializer: D
) -> std::result::Result<BTreeMap<PathBuf, Vec<u8>>, D::Error>
where
    D: serde::Deserializer<'de>, {
    let map: BTreeMap<PathBuf, String> = BTreeMap::deserialize(deserializer)?;
    Ok(map
        .into_iter()
        .map(|(k, v)| (k, v.into_bytes()))
        .collect())
}

fn ser_with_utf<S>(
    orig: &BTreeMap<PathBuf, Vec<u8>>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer, {
    let map: BTreeMap<PathBuf, String> = orig
        .iter()
        .map(|(k, v)| (k.clone(), String::from_utf8_lossy(v).into_owned()))
        .collect();
    map.serialize(serializer)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
struct FilesInner {
    #[serde(serialize_with = "ser_with_utf", deserialize_with = "de_with_utf")]
    map: BTreeMap<PathBuf, Vec<u8>>,
}

impl FilesInner {
    fn new() -> Self {
        Self::default()
    }
}

impl Deref for FilesInner {
    type Target = BTreeMap<PathBuf, Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for FilesInner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

#[derive(Clone, Debug)]
pub struct MemoryFileSystem {
    files: Arc<RwLock<FilesInner>>,
    #[cfg(fs_test)]
    operations: Arc<RwLock<Vec<FsOperation>>>,
}

impl serde::Serialize for MemoryFileSystem {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer, {
        let files = self.files.read().unwrap();
        files.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for MemoryFileSystem {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>, {
        let files_inner = FilesInner::deserialize(deserializer)?;
        Ok(Self {
            files: Arc::new(RwLock::new(files_inner)),
            #[cfg(fs_test)]
            operations: Arc::new(RwLock::new(Vec::new())),
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
            files: Arc::new(RwLock::new(FilesInner::new())),
            #[cfg(fs_test)]
            operations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_files(
        files: impl IntoIterator<Item = (impl Into<PathBuf>, impl Into<Vec<u8>>)>
    ) -> Self {
        let mut map = BTreeMap::new();
        for (path, contents) in files {
            map.insert(path.into(), contents.into());
        }
        Self {
            files: Arc::new(RwLock::new(FilesInner { map })),
            #[cfg(fs_test)]
            operations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add_file(
        &self,
        path: impl Into<PathBuf>,
        contents: impl AsRef<[u8]>,
    ) {
        let mut files = self.files.write().unwrap();
        files.insert(path.into(), contents.as_ref().to_vec());
    }

    pub fn remove_file(
        &self,
        path: &Path,
    ) -> bool {
        let mut files = self.files.write().unwrap();
        files.remove(path).is_some()
    }

    pub fn clear(&self) {
        let mut files = self.files.write().unwrap();
        files.clear();
        #[cfg(fs_test)]
        self.clear_operations();
    }

    pub fn list_files(&self) -> Vec<PathBuf> {
        let files = self.files.read().unwrap();
        files.keys().cloned().collect()
    }

    pub fn get_file_content(
        &self,
        path: &Path,
    ) -> Option<Vec<u8>> {
        let files = self.files.read().unwrap();
        files.get(&remove_relative(path)).cloned()
    }

    #[cfg(fs_test)]
    pub fn operations(&self) -> Vec<FsOperation> {
        let ops = self.operations.read().unwrap();
        ops.clone()
    }

    #[cfg(fs_test)]
    pub fn clear_operations(&self) {
        let mut ops = self.operations.write().unwrap();
        ops.clear();
    }

    #[cfg(fs_test)]
    fn track_operation(
        &self,
        op: FsOperation,
    ) {
        let mut ops = self.operations.write().unwrap();
        ops.push(op);
    }

    fn matches_glob(
        path: &Path,
        pattern: &str,
    ) -> bool {
        match glob::Pattern::new(
            &remove_relative(&PathBuf::from(pattern))
                .display()
                .to_string(),
        ) {
            Ok(pat) => pat.matches_path(&remove_relative(path)),
            Err(_) => false,
        }
    }

    #[cfg(fs_test)]
    pub fn debug_print_files(&self) {
        let files = self.files.read().unwrap();
        for (path, contents) in files.iter() {
            println!("File: {} ({} bytes)", path.display(), contents.len());
            println!("```\n{}\n```", String::from_utf8_lossy(contents));
        }
    }

    pub fn danger_write_to_physical(
        &self,
        root_path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let files = self.files.read().unwrap();
        for (path, contents) in files.iter() {
            let full_path = root_path.as_ref().join(path);
            if let Some(parent) = full_path.parent()
                && !parent.exists()
            {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(full_path, contents)?;
        }
        Ok(())
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
        let files = self.files.read().unwrap();
        let exists = files.contains_key(&remove_relative(path));
        #[cfg(fs_test)]
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
        let files = self.files.read().unwrap();
        let mut results = Vec::new();

        for path in files.keys() {
            let matches_include = include.is_empty()
                || include
                    .iter()
                    .any(|pattern| Self::matches_glob(path, pattern));

            let matches_exclude = exclude
                .iter()
                .any(|pattern| Self::matches_glob(path, pattern));

            if matches_include && !matches_exclude {
                results.push(path.clone());
            }
        }

        results.sort();

        #[cfg(fs_test)]
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

        #[cfg(fs_test)]
        let operations = self.operations.clone();

        let path = remove_relative(path);
        Box::pin(async move {
            let files = files.read().unwrap();
            let result = files.get(&path).cloned().ok_or_else(|| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("File not found: {}", path.display()),
                ))
            });

            #[cfg(fs_test)]
            if result.is_ok() {
                let mut ops = operations.write().unwrap();
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

        #[cfg(fs_test)]
        let operations = self.operations.clone();

        let path = remove_relative(path);
        Box::pin(async move {
            let files = files.read().unwrap();
            let bytes = files.get(&path).ok_or_else(|| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("File not found: {}", path.display()),
                ))
            })?;
            let result = String::from_utf8(bytes.clone()).map_err(|e| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid UTF-8: {}", e),
                ))
            });

            #[cfg(fs_test)]
            if result.is_ok() {
                let mut ops = operations.write().unwrap();
                ops.push(FsOperation::ReadToString { path: path.clone() });
            }
            result
        })
    }

    fn read_to_string_sync(
        &self,
        path: &Path,
    ) -> Result<String> {
        let files = self.files.read().unwrap();
        let path = remove_relative(path);
        let bytes = files.get(&path).ok_or_else(|| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path.display()),
            ))
        })?;
        let result = String::from_utf8(bytes.clone()).map_err(|e| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid UTF-8: {}", e),
            ))
        });

        #[cfg(fs_test)]
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

        #[cfg(fs_test)]
        let operations = self.operations.clone();
        #[cfg(fs_test)]
        let size = contents.len();

        Box::pin(async move {
            let mut files = files.write().unwrap();

            files.insert(path.clone(), contents);

            #[cfg(fs_test)]
            {
                let mut ops = operations.write().unwrap();
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
