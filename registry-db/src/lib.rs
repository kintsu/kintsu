#![recursion_limit = "512"]

pub mod handlers;
pub mod models;
pub(crate) mod schema;
pub(crate) mod tokens;

use diesel_async::{AsyncPgConnection, pooled_connection::bb8::Pool};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

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
}

pub type Result<T> = std::result::Result<T, Error>;

pub type AsyncConnectionPool = Pool<AsyncPgConnection>;
