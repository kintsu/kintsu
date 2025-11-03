#[derive(serde::Deserialize, serde::Serialize)]
pub struct PackageUpload {
    // while we keep the manifest in our fs, we also add it here for easy access
    pub manifest: super::package::PackageManifest,
    pub package: super::lock::Lockfiles,
    pub source: kintsu_fs::memory::MemoryFileSystem,
}
