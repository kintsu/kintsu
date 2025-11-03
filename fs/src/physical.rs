use std::{path::Path, pin::Pin};

pub struct Physical;

impl crate::FileSystem for Physical {
    fn exists_sync(
        &self,
        path: &Path,
    ) -> bool {
        std::fs::exists(path).unwrap_or_default()
    }

    fn find_glob(
        &self,
        include: &[String],
        exclude: &[String],
    ) -> crate::Result<Vec<std::path::PathBuf>> {
        crate::match_paths::match_paths(include, exclude)
    }

    fn read(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = crate::Result<Vec<u8>>> + Send + Sync>> {
        let path = path.to_path_buf();
        Box::pin(async move { Ok(tokio::fs::read(path).await?) })
    }

    fn read_to_string(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = crate::Result<String>> + Send + Sync>> {
        let path = path.to_path_buf();
        Box::pin(async move { Ok(tokio::fs::read_to_string(path).await?) })
    }

    fn read_to_string_sync(
        &self,
        path: &Path,
    ) -> crate::Result<String> {
        Ok(std::fs::read_to_string(path)?)
    }

    fn write(
        &self,
        path: &Path,
        contents: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send + Sync>> {
        let path = path.to_path_buf();
        Box::pin(async move { Ok(tokio::fs::write(path, contents).await?) })
    }
}
