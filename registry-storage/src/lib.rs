use aws_sdk_s3::presigning::PresigningConfigError;
use std::sync::Arc;

pub mod manager;
pub mod s3;

#[cfg(feature = "test")]
pub mod tst;

#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("checksum mismatch: expected {expected}, found {found}")]
    ChecksumMismatch { expected: String, found: String },
    #[error("storage error: {0}")]
    StoreError(String),
    #[error("storage error: {0}")]
    RetrievalError(String),

    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("'{path}': {source}")]
    WithPath {
        path: String,
        #[source]
        source: Box<Self>,
    },

    #[error("presign error: {0}")]
    PresigningConfigError(#[from] PresigningConfigError),
}

impl StorageError {
    pub fn with_path(path: impl Into<String>) -> impl FnOnce(Self) -> Self {
        let path = path.into();
        move |source| {
            StorageError::WithPath {
                path,
                source: Box::new(source),
            }
        }
    }
}

/// AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY must be set in the environment
#[derive(serde::Deserialize, Debug)]
pub struct Config {
    #[serde(alias = "ENDPOINT")]
    pub endpoint: String,
    #[serde(alias = "BUCKET")]
    pub bucket: String,
    #[serde(alias = "REGION")]
    pub region: String,
    #[serde(alias = "ACCESS_KEY_ID")]
    pub access_key_id: secrecy::SecretString,
    #[serde(alias = "SECRET_ACCESS_KEY")]
    pub secret_access_key: secrecy::SecretString,
}

pub struct StorageIndex;

pub enum AssetType {
    Source,
    Declarations,
}

impl std::fmt::Display for AssetType {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            AssetType::Source => write!(f, "source"),
            AssetType::Declarations => write!(f, "declarations"),
        }
    }
}

impl StorageIndex {
    pub fn path_for_package(
        package_name: &str,
        version: &str,
        asset: AssetType,
    ) -> String {
        format!(
            "{}/{}/{}/{}.json",
            package_name.chars().next().unwrap(),
            package_name,
            version,
            asset
        )
    }

    pub fn path_for_source(
        package_name: &str,
        version: &str,
    ) -> String {
        Self::path_for_package(package_name, version, AssetType::Source)
    }

    pub fn path_for_declarations(
        package_name: &str,
        version: &str,
    ) -> String {
        Self::path_for_package(package_name, version, AssetType::Declarations)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksum(String);

impl From<String> for Checksum {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Checksum {
    pub fn hash(input: &[u8]) -> Self {
        let hash = sha256::digest(input);
        Self(hash)
    }

    pub fn verify(
        &self,
        input: &[u8],
    ) -> bool {
        let hash = Self::hash(input);
        &hash == self
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

pub type LocalFuture<'a, T = ()> =
    std::pin::Pin<Box<dyn Future<Output = Result<T, StorageError>> + Send + 'a>>;

pub struct BulkGetPackage {
    pub package_name: String,
    pub version: String,
    pub checksums: StoredPackageChecksum,
}

pub struct BulkGetPackageResponse<D> {
    pub package_name: String,
    pub version: String,
    pub content: StoredPackageContent<D>,
}

pub struct BulkGetSource {
    pub package_name: String,
    pub version: String,
    pub checksum: Checksum,
}

pub struct BulkGetSourceResponse {
    pub package_name: String,
    pub version: String,
    pub fs: kintsu_fs::memory::MemoryFileSystem,
}

pub struct StoredPackageChecksum {
    pub source_checksum: Checksum,
    pub declarations_checksum: Checksum,
}

impl StoredPackageChecksum {
    pub fn new(
        source_checksum: impl Into<Checksum>,
        declarations_checksum: impl Into<Checksum>,
    ) -> Self {
        Self {
            source_checksum: source_checksum.into(),
            declarations_checksum: declarations_checksum.into(),
        }
    }
}

pub struct StoredPackageContent<D> {
    pub fs: kintsu_fs::memory::MemoryFileSystem,
    pub declarations: D,
}

pub trait PackageStorage<D: Send + Sync + serde::Serialize + serde::de::DeserializeOwned>:
    Send + Sync {
    fn path_for_source(
        &self,
        package_name: &str,
        version: &str,
    ) -> String {
        StorageIndex::path_for_source(package_name, version)
    }

    fn path_for_declarations(
        &self,
        package_name: &str,
        version: &str,
    ) -> String {
        StorageIndex::path_for_declarations(package_name, version)
    }

    fn checksum(
        &self,
        bytes: &[u8],
    ) -> Checksum {
        Checksum::hash(bytes)
    }

    fn encode(
        &self,
        data: Vec<u8>,
    ) -> Vec<u8> {
        data
    }

    fn decode(
        &self,
        data: Vec<u8>,
    ) -> Vec<u8> {
        data
    }

    fn put_source<'d>(
        &'d self,
        path: &'d str,
        data: &'d kintsu_fs::memory::MemoryFileSystem,
    ) -> LocalFuture<'d, Checksum>;

    fn put_declarations<'d>(
        &'d self,
        path: &'d str,
        data: &'d D,
    ) -> LocalFuture<'d, Checksum>;

    fn get_source<'d>(
        &'d self,
        path: &'d str,
        checksum: Checksum,
    ) -> LocalFuture<'d, kintsu_fs::memory::MemoryFileSystem>;

    fn get_sources<'d>(
        &'d self,
        sources: Vec<BulkGetSource>,
    ) -> LocalFuture<'d, Vec<BulkGetSourceResponse>> {
        let futures = sources
            .into_iter()
            .map(
                |BulkGetSource {
                     package_name,
                     version,
                     checksum,
                 }| {
                    async move {
                        let fs = self
                            .retrieve_source(&package_name, &version, checksum)
                            .await?;

                        Ok::<_, crate::StorageError>(BulkGetSourceResponse {
                            package_name,
                            version,
                            fs,
                        })
                    }
                },
            )
            .collect::<Vec<_>>();

        Box::pin(async move { Ok(futures_util::future::try_join_all(futures).await?) })
    }

    fn get_declarations<'d>(
        &'d self,
        path: &'d str,
        checksum: Checksum,
    ) -> LocalFuture<'d, D>;

    fn store_package<'p>(
        &'p self,
        package_name: &'p str,
        version: &'p str,
        fs: &'p kintsu_fs::memory::MemoryFileSystem,
        declarations: &'p D,
    ) -> LocalFuture<'p, StoredPackageChecksum> {
        let source_path = self.path_for_source(package_name, version);
        let declarations_path = self.path_for_declarations(package_name, version);

        Box::pin(async move {
            let (source_checksum, declarations_checksum) = tokio::join!(
                self.put_source(&source_path, fs),
                self.put_declarations(&declarations_path, declarations),
            );

            let source_checksum = source_checksum.map_err(StorageError::with_path(&source_path))?;

            let declarations_checksum =
                declarations_checksum.map_err(StorageError::with_path(&declarations_path))?;

            Ok(StoredPackageChecksum {
                source_checksum,
                declarations_checksum,
            })
        })
    }

    fn retrieve_package<'p>(
        &'p self,
        package_name: &'p str,
        version: &'p str,
        stored: StoredPackageChecksum,
    ) -> LocalFuture<'p, StoredPackageContent<D>> {
        let source_path = self.path_for_source(package_name, version);
        let declarations_path = self.path_for_declarations(package_name, version);

        Box::pin(async move {
            let (fs, declarations) = tokio::join!(
                self.get_source(&source_path, stored.source_checksum,),
                self.get_declarations(&declarations_path, stored.declarations_checksum),
            );

            let fs = fs.map_err(StorageError::with_path(&source_path))?;

            let declarations = declarations.map_err(StorageError::with_path(&declarations_path))?;

            Ok(StoredPackageContent { fs, declarations })
        })
    }

    fn retrieve_source<'a>(
        &'a self,
        package_name: &'a str,
        version: &'a str,
        expected_checksum: Checksum,
    ) -> LocalFuture<'a, kintsu_fs::memory::MemoryFileSystem> {
        let source_path = self.path_for_source(package_name, version);

        Box::pin(async move {
            let fs = self
                .get_source(&source_path, expected_checksum)
                .await
                .map_err(StorageError::with_path(&source_path))?;

            Ok(fs)
        })
    }

    fn retrieve_declarations<'a>(
        &'a self,
        package_name: &'a str,
        version: &'a str,
        expected_checksum: Checksum,
    ) -> LocalFuture<'a, D> {
        let declarations_path = self.path_for_declarations(package_name, version);

        Box::pin(async move {
            let declarations = self
                .get_declarations(&declarations_path, expected_checksum)
                .await
                .map_err(StorageError::with_path(&declarations_path))?;

            Ok(declarations)
        })
    }

    fn retrieve_packages<'a>(
        &'a self,
        packages: Vec<BulkGetPackage>,
    ) -> LocalFuture<'a, Vec<BulkGetPackageResponse<D>>> {
        let futures = packages
            .into_iter()
            .map(
                |BulkGetPackage {
                     package_name,
                     version,
                     checksums,
                 }| {
                    async move {
                        let content = self
                            .retrieve_package(&package_name, &version, checksums)
                            .await?;

                        Ok::<_, crate::StorageError>(BulkGetPackageResponse::<D> {
                            package_name,
                            version,
                            content,
                        })
                    }
                },
            )
            .collect::<Vec<_>>();

        Box::pin(async move { Ok(futures_util::future::try_join_all(futures).await?) })
    }
}
