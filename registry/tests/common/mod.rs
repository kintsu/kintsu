//! Test infrastructure for registry integration tests
//!
//! Provides TestRegistryCtx which composes TestDbCtx and TestS3Ctx
//! along with fluent builders for making HTTP requests.

mod ctx;
mod request;
mod response;

pub use ctx::*;
pub use request::*;
pub use response::*;
