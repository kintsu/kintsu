use crate::schema::{api_key, org};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

#[derive(Debug, Identifiable, AsChangeset, HasQuery, Serialize, Clone, utoipa::ToSchema)]
#[diesel(table_name = org)]
pub struct Org {
    pub id: i64,
    pub name: String,
    pub gh_id: i32,
    pub gh_avatar: String,
}

#[derive(Debug, Serialize, Clone, utoipa::ToSchema)]
pub struct OrgWithAdmin {
    #[serde(flatten)]
    pub org: Org,
    pub user_is_admin: bool,
}

impl Org {
    pub async fn by_id(
        conn: &mut AsyncPgConnection,
        org_id: i64,
    ) -> crate::Result<Option<Self>> {
        use crate::schema::org::dsl::*;

        let org_opt = org
            .filter(id.eq(org_id))
            .first::<Self>(conn)
            .await
            .optional()?;

        Ok(org_opt)
    }

    pub async fn by_name(
        conn: &mut AsyncPgConnection,
        org_name: &str,
    ) -> crate::Result<Option<Self>> {
        use crate::schema::org::dsl::*;

        let org_opt = org
            .filter(name.eq(org_name))
            .first::<Self>(conn)
            .await
            .optional()?;

        Ok(org_opt)
    }

    pub async fn exists(
        conn: &mut AsyncPgConnection,
        org_name: &str,
    ) -> crate::Result<bool> {
        use crate::schema::org::dsl::*;

        let count: i64 = org
            .filter(name.eq(org_name))
            .count()
            .get_result(conn)
            .await?;

        Ok(count > 0)
    }

    pub async fn exists_bulk(
        conn: &mut AsyncPgConnection,
        org_names: &[&str],
    ) -> crate::Result<Vec<String>> {
        use crate::schema::org::dsl::*;

        let existing_orgs = org
            .filter(name.eq_any(org_names))
            .select(name)
            .load::<String>(conn)
            .await?;

        Ok(existing_orgs)
    }

    /// **WARNING**: This returns all API keys for the org, including _revoked_ keys.
    pub async fn tokens(
        conn: &mut AsyncPgConnection,
        org_id: i64,
    ) -> crate::Result<Vec<super::api_key::ApiKey>> {
        api_key::table
            .filter(api_key::org_id.eq(org_id))
            .order(api_key::id.desc())
            .select(super::api_key::ApiKey::as_select())
            .load(conn)
            .await
            .map_err(Into::into)
    }

    pub async fn is_user_admin(
        &self,
        conn: &mut AsyncPgConnection,
        user_id: i64,
    ) -> crate::Result<bool> {
        use crate::schema::org_admin;

        let count: i64 = org_admin::table
            .filter(org_admin::org_id.eq(self.id))
            .filter(org_admin::user_id.eq(user_id))
            .filter(org_admin::revoked_at.is_null())
            .count()
            .get_result(conn)
            .await?;

        Ok(count > 0)
    }

    pub async fn must_be_admin(
        &self,
        conn: &mut AsyncPgConnection,
        user_id: i64,
    ) -> crate::Result<()> {
        if !self.is_user_admin(conn, user_id).await? {
            return Err(crate::Error::Unauthorized(
                "User is not an admin of the organization".into(),
            ));
        }
        Ok(())
    }
}
