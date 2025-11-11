use secrecy::ExposeSecret;
use serde::{Serialize, de::DeserializeOwned};

use crate::*;

use crate::manager::StorageManager;

pub struct S3Storage<D> {
    pub(crate) client: aws_sdk_s3::Client,
    bucket_name: String,
    ph: std::marker::PhantomData<D>,
}

// static is ok in this context because the data is owned by the caller / we never have ownership
impl<D: 'static + Send + Sync + serde::Serialize + serde::de::DeserializeOwned> S3Storage<D> {
    pub async fn new(config: &Config) -> Self {
        let credentials = aws_sdk_s3::config::Credentials::new(
            config.access_key_id.expose_secret(),
            config.secret_access_key.expose_secret(),
            None,
            None,
            "kintsu-registry",
        );
        let sdk_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
            .endpoint_url(config.endpoint.clone())
            .credentials_provider(credentials)
            .force_path_style(true)
            .region(aws_sdk_s3::config::Region::new(config.region.clone()))
            .build();

        let client = aws_sdk_s3::Client::from_conf(s3_config);

        Self {
            client,
            bucket_name: config.bucket.clone(),
            ph: std::marker::PhantomData,
        }
    }

    pub async fn managed(config: &Config) -> StorageManager<D> {
        StorageManager::<D>::new(Arc::new(Self::new(config).await))
    }

    pub async fn put_and_get_checksum<T: Serialize>(
        &self,
        path: &str,
        data: &T,
    ) -> Result<Checksum, StorageError> {
        let data = serde_json::to_vec(data).map_err(|e| StorageError::StoreError(e.to_string()))?;
        let data = self.encode(data);

        let checksum = Checksum::hash(&data);

        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(path)
            .content_type("application/json")
            .checksum_sha256(checksum.value())
            .body(aws_sdk_s3::primitives::ByteStream::from(data))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to store object in S3: {:#?}", e);
                StorageError::StoreError(e.to_string())
            })?;

        Ok(checksum)
    }

    pub async fn get_and_verify<T: DeserializeOwned>(
        &self,
        path: &str,
        checksum: Checksum,
    ) -> Result<T, StorageError> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(path)
            .checksum_mode(aws_sdk_s3::types::ChecksumMode::Enabled)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to retrieve object from S3: {:#?}", e);
                StorageError::RetrievalError(e.to_string())
            })?;

        let data = resp
            .body
            .collect()
            .await
            .map_err(|e| StorageError::RetrievalError(e.to_string()))?
            .into_bytes();

        let found = Checksum::hash(&data);
        if found != checksum {
            return Err(StorageError::ChecksumMismatch {
                expected: checksum.value().to_string(),
                found: found.value().to_string(),
            });
        }

        let data = self.decode(data.to_vec());
        Ok(serde_json::from_slice(&data)?)
    }

    pub async fn presign(
        &self,
        path: &str,
        expires_in: chrono::Duration,
    ) -> Result<String, StorageError> {
        let presigner =
            aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in.to_std().unwrap())?;

        let req = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(path)
            .checksum_mode(aws_sdk_s3::types::ChecksumMode::Enabled)
            .presigned(presigner)
            .await
            .map_err(|e| StorageError::RetrievalError(e.to_string()))?;

        Ok(req.uri().to_string())
    }
}

impl<D: 'static + Send + Sync + serde::Serialize + serde::de::DeserializeOwned> PackageStorage<D>
    for S3Storage<D>
{
    fn put_source<'d>(
        &'d self,
        path: &'d str,
        data: &'d kintsu_fs::memory::MemoryFileSystem,
    ) -> LocalFuture<'d, Checksum> {
        Box::pin(async move { self.put_and_get_checksum(path, data).await })
    }

    fn put_declarations<'d>(
        &'d self,
        path: &'d str,
        data: &'d D,
    ) -> LocalFuture<'d, Checksum> {
        Box::pin(async move { self.put_and_get_checksum(path, data).await })
    }

    fn get_source<'d>(
        &'d self,
        path: &'d str,
        checksum: Checksum,
    ) -> LocalFuture<'d, kintsu_fs::memory::MemoryFileSystem> {
        Box::pin(async move { self.get_and_verify(path, checksum).await })
    }

    fn get_declarations<'d>(
        &'d self,
        path: &'d str,
        checksum: Checksum,
    ) -> LocalFuture<'d, D> {
        Box::pin(async move { self.get_and_verify(path, checksum).await })
    }
}

#[cfg(all(test, feature = "test"))]
mod test {
    use kintsu_fs::FileSystem;

    use super::*;
    use crate::PackageStorage;

    #[derive(serde::Deserialize, serde::Serialize)]
    struct TestDecl(String);

    unsafe impl Send for TestDecl {}
    unsafe impl Sync for TestDecl {}

    #[tokio::test]
    async fn test_put() {
        let ctx = crate::tst::TestS3Ctx::new().await;
        let s3 = super::S3Storage::<TestDecl>::new(&ctx.conf).await;

        let decl = TestDecl("declarations contents".to_string());
        let chksum = s3
            .put_and_get_checksum("foo.txt", &decl)
            .await
            .unwrap();

        let out = s3
            .get_and_verify::<TestDecl>("foo.txt", chksum)
            .await
            .unwrap();

        assert_eq!(out.0, "declarations contents".to_string());
    }

    #[tokio::test]
    async fn test_s3_trait_impl() {
        let ctx = crate::tst::TestS3Ctx::new().await;
        let s3 = super::S3Storage::<TestDecl>::new(&ctx.conf).await;

        let data = TestDecl("baz".to_string());
        let fs = kintsu_fs::memory! {
            "data-we-want-flat" => "bar".to_string(),
        };

        let package_name = "my-package";
        let version = "1.0.0";

        let stored = s3
            .store_package(package_name, version, &fs, &data)
            .await
            .unwrap();

        let _ = s3
            .get_and_verify::<TestDecl>(
                "m/my-package/1.0.0/declarations.json",
                stored.declarations_checksum.clone(),
            )
            .await
            .unwrap();

        let _ = s3
            .get_and_verify::<kintsu_fs::memory::MemoryFileSystem>(
                "m/my-package/1.0.0/source.json",
                stored.source_checksum.clone(),
            )
            .await
            .unwrap();

        let content = s3
            .retrieve_package(package_name, version, stored)
            .await
            .unwrap();

        assert_eq!(content.declarations.0, "baz");

        assert_eq!(
            content
                .fs
                .read_to_string_sync(&std::path::PathBuf::from("data-we-want-flat"))
                .unwrap(),
            "bar".to_string()
        );
    }
}
