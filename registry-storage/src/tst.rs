use testcontainers::{
    ContainerAsync, GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

pub struct TestS3Ctx {
    pub container: ContainerAsync<GenericImage>,
    pub conf: crate::Config,
    pub client: crate::s3::S3Storage<()>,
}

impl TestS3Ctx {
    const TAG: &'static str =
        "sha256:8e467b32af3ff83e70c70dddb0c36b5e611f46e89a3075db8770aea4f30b2fe3";

    pub async fn new() -> Self {
        let container = GenericImage::new("rustfs/rustfs", Self::TAG)
            .with_exposed_port(9000.tcp())
            .with_exposed_port(9001.tcp())
            .with_wait_for(WaitFor::message_on_stdout("localhost"))
            .with_env_var("RUSTFS_ADDRESS", "0.0.0.0:9000")
            .with_env_var("RUSTFS_CONSOLE_ADDRESS", "0.0.0.0:9001")
            .with_env_var("RUSTFS_CONSOLE_ENABLE", "true")
            .with_env_var("RUSTFS_EXTERNAL_ADDRESS", ":9000")
            .with_env_var("RUSTFS_CORS_ALLOWED_ORIGINS", "*")
            .with_env_var("RUSTFS_CONSOLE_CORS_ALLOWED_ORIGINS", "*")
            .with_env_var("RUSTFS_ACCESS_KEY", "rustfsadmin")
            .with_env_var("RUSTFS_SECRET_KEY", "rustfsadmin")
            .with_env_var("RUSTFS_LOG_LEVEL", "info")
            .start()
            .await
            .expect("Failed to start rustfs");

        let host_port = container
            .get_host_port_ipv4(9000)
            .await
            .unwrap();

        let conf = crate::Config {
            endpoint: format!("http://127.0.0.1:{host_port}"),
            bucket: "test-bucket".to_string(),
            region: "ca-central-1".to_string(),
            access_key_id: "rustfsadmin".into(),
            secret_access_key: "rustfsadmin".into(),
        };

        let client = crate::s3::S3Storage::<()>::new(&conf).await;

        client
            .client
            .create_bucket()
            .bucket(&conf.bucket)
            .send()
            .await
            .unwrap();

        Self {
            container,
            conf,
            client,
        }
    }

    pub async fn managed<
        D: serde::de::DeserializeOwned + serde::Serialize + Sync + Send + 'static,
    >(
        &self
    ) -> crate::manager::StorageManager<D> {
        crate::s3::S3Storage::<D>::managed(&self.conf).await
    }
}

#[tokio::test]
async fn ctx() {
    let _ = TestS3Ctx::new().await;
}
