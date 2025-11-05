use crate::{
    Error, Result,
    models::org::Org,
    schema::{org, org_admin},
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

/// Import a GitHub organization into the registry
///
/// Creates both the org record and the org_admin relationship in a transaction.
/// Returns Conflict error if org already exists.
///
/// # Security
/// - Caller must validate admin permissions via GitHub API before calling this
/// - Creates atomic transaction for org + org_admin
/// - Checks for existing org by gh_id to prevent duplicates
pub async fn import_organization(
    conn: &mut AsyncPgConnection,
    gh_id: i32,
    org_name: &str,
    gh_avatar: &str,
    admin_user_id: i64,
) -> Result<Org> {
    conn.transaction(|conn| {
        Box::pin(async move {
            // Check if org already exists by GitHub ID
            let existing = org::table
                .filter(
                    org::name
                        .eq(org_name)
                        .or(org::gh_id.eq(gh_id)),
                )
                .first::<Org>(conn)
                .await
                .optional()?;

            if let Some(existing_org) = existing {
                return Err(Error::Conflict(format!(
                    "Organization '{}' (gh_id: {}) already imported",
                    existing_org.name, existing_org.gh_id
                )));
            }

            // Insert organization
            let new_org = diesel::insert_into(org::table)
                .values((
                    org::name.eq(org_name),
                    org::gh_id.eq(gh_id),
                    org::gh_avatar.eq(gh_avatar),
                ))
                .returning(Org::as_returning())
                .get_result(conn)
                .await?;

            // Create org_admin relationship
            diesel::insert_into(org_admin::table)
                .values((
                    org_admin::org_id.eq(new_org.id),
                    org_admin::user_id.eq(admin_user_id),
                    org_admin::revoked_at.eq(None::<chrono::DateTime<chrono::Utc>>),
                ))
                .execute(conn)
                .await?;

            Ok(new_org)
        })
    })
    .await
}
