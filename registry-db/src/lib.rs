#![recursion_limit = "512"]

pub mod engine;
pub mod entities;
pub(crate) mod tokens;

#[cfg(feature = "test")]
pub mod tst;

pub use tokens::MAX_TOKEN_HEADER_LENGTH;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    SeaOrm(#[from] sea_orm::DbErr),

    #[error("Database error: {0}")]
    TransactionError(Box<sea_orm::TransactionError<Self>>),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Resource conflict: {0}")]
    Conflict(String),

    #[error("Package version already exists: {package}@{version}")]
    PackageVersionExists { package: String, version: String },

    #[error("Storage error: {0}")]
    Storage(#[from] kintsu_registry_storage::StorageError),

    #[error("Manifest error: {0}")]
    InvalidManifest(#[from] kintsu_manifests::InvalidManifest),

    #[error("Manifest error: {0}")]
    Manifest(#[from] kintsu_manifests::Error),
}

impl<E> From<sea_orm::TransactionError<E>> for Error
where
    Error: From<E>,
{
    fn from(err: sea_orm::TransactionError<E>) -> Self {
        match err {
            sea_orm::TransactionError::Connection(e) => Error::SeaOrm(e),
            sea_orm::TransactionError::Transaction(e) => Error::from(e),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub type SeaOrmConnection = sea_orm::DatabaseConnection;

pub type DateTime = chrono::DateTime<chrono::Utc>;

pub type PackageStorage =
    kintsu_registry_storage::manager::StorageManager<kintsu_parser::declare::DeclarationVersion>;
