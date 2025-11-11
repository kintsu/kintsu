#![allow(async_fn_in_trait)]

use std::{
    path::{Path, PathBuf},
    pin::Pin,
};
pub mod match_paths;
pub mod memory;
pub mod physical;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("glob error: {0}")]
    Glob(#[from] glob::GlobError),
    #[error("pattern error: {0}")]
    GlobPattern(#[from] glob::PatternError),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub trait FileSystem: Send + Sync {
    fn exists_sync(
        &self,
        path: &Path,
    ) -> bool;

    fn find_glob(
        &self,
        include: &[String],
        exclude: &[String],
    ) -> Result<Vec<PathBuf>>;

    fn read(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = crate::Result<Vec<u8>>> + Send + Sync>>;

    fn read_to_string(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + Sync>>;

    fn read_to_string_sync(
        &self,
        path: &Path,
    ) -> crate::Result<String>;

    fn write(
        &self,
        path: &Path,
        contents: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send + Sync>>;
}

impl<T: FileSystem + ?Sized> FileSystem for std::sync::Arc<T> {
    fn exists_sync(
        &self,
        path: &Path,
    ) -> bool {
        self.as_ref().exists_sync(path)
    }

    fn find_glob(
        &self,
        include: &[String],
        exclude: &[String],
    ) -> Result<Vec<PathBuf>> {
        self.as_ref().find_glob(include, exclude)
    }

    fn read(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = crate::Result<Vec<u8>>> + Send + Sync>> {
        self.as_ref().read(path)
    }

    fn read_to_string(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + Sync>> {
        self.as_ref().read_to_string(path)
    }

    fn read_to_string_sync(
        &self,
        path: &Path,
    ) -> crate::Result<String> {
        self.as_ref().read_to_string_sync(path)
    }

    fn write(
        &self,
        path: &Path,
        contents: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send + Sync>> {
        self.as_ref().write(path, contents)
    }
}
