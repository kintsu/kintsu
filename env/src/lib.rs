use std::path::PathBuf;
pub mod models;
pub(crate) mod schema;
use diesel_async::{AsyncConnection, sync_connection_wrapper::SyncConnectionWrapper};

pub struct Env {
    pub database_path: PathBuf,
    pub database: SyncConnectionWrapper<diesel::prelude::SqliteConnection>,
}

impl Env {
    pub async fn new() -> Self {
        let database_path = PathBuf::from("./dummy.sqlite");

        let database =
            SyncConnectionWrapper::establish(&format!("sqlite://{}", database_path.display()))
                .await
                .unwrap();
        Self {
            database_path,
            database,
        }
    }
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_conn() {
        let _ = super::Env::new().await;
    }
}
