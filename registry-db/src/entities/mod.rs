pub mod prelude;

pub(crate) mod api_key;
pub mod api_key_public;
pub mod downloads;
pub mod org;
pub mod org_invitation;
pub mod org_role;
pub mod package;
pub mod schema_role;
pub mod types;
pub mod user_favourite;
pub mod users;
pub mod version;

pub use prelude::*;

// Re-export ActiveModel types for tests
#[cfg(feature = "test")]
pub use {
    downloads::ActiveModel as DownloadsActiveModel, org::ActiveModel as OrgActiveModel,
    org_invitation::ActiveModel as OrgInvitationActiveModel,
    org_role::ActiveModel as OrgRoleActiveModel, package::ActiveModel as PackageActiveModel,
    schema_role::ActiveModel as SchemaRoleActiveModel,
    user_favourite::ActiveModel as UserFavouriteActiveModel, users::ActiveModel as UserActiveModel,
    version::ActiveModel as VersionActiveModel,
};
