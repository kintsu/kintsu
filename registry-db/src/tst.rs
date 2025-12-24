use sea_orm::ConnectionTrait;
use testcontainers::ContainerAsync;
use testcontainers_modules::{postgres, testcontainers::runners::AsyncRunner};

pub struct TestDbCtx {
    #[allow(unused)] // kept alive
    container: ContainerAsync<postgres::Postgres>,

    pub db_url: String,
    pub conn: sea_orm::DatabaseConnection,
}

impl TestDbCtx {
    pub async fn new() -> Self {
        const UP: &[&'static str] = &[include_str!("../migrations/0001_registry/up.sql")];

        let container = postgres::Postgres::default()
            .pull_image()
            .await
            .unwrap()
            .start()
            .await
            .unwrap();

        let host_port = container
            .get_host_port_ipv4(5432)
            .await
            .unwrap();

        let connection_string =
            &format!("postgres://postgres:postgres@127.0.0.1:{host_port}/postgres");

        let conn = sea_orm::Database::connect(connection_string)
            .await
            .unwrap();

        for up in UP {
            conn.execute_unprepared(*up).await.unwrap();
        }

        Self {
            db_url: connection_string.to_string(),
            container,
            conn,
        }
    }
}

#[tokio::test]
async fn ctx() {
    let _ = TestDbCtx::new().await;
}
