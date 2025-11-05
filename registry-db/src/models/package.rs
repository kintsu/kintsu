use crate::{
    Error, Result,
    models::{api_key::ApiKey, scopes::Permission, version::Version},
    schema::{downloads, package, schema_admin, version},
};
use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

#[derive(Debug, Identifiable, AsChangeset, HasQuery, Serialize, utoipa::ToSchema, Clone)]
#[diesel(table_name = package)]
pub struct Package {
    pub id: i64,
    pub name: String,
}

impl Package {
    pub async fn get_package_download_count(
        conn: &mut AsyncPgConnection,
        package_name: &str,
    ) -> Result<i64> {
        let count: Option<i64> = downloads::table
            .inner_join(version::table)
            .inner_join(package::table.on(version::package.eq(package::id)))
            .filter(package::name.eq(package_name))
            .select(diesel::dsl::sum(downloads::count))
            .first(conn)
            .await?;

        Ok(count.unwrap_or(0))
    }

    pub async fn package_download_history(
        conn: &mut AsyncPgConnection,
        package_name: &str,
    ) -> Result<Vec<DownloadHistory>> {
        let results = diesel::sql_query(
            r#"
            SELECT
                d.day,
                v.qualified_version as version,
                d.count
            FROM downloads d
            INNER JOIN version v ON d.version = v.id
            INNER JOIN package p ON v.package = p.id
            WHERE p.name = $1
            AND d.day >= CURRENT_DATE - INTERVAL '90 days'
            ORDER BY d.day DESC, v.id DESC
            "#,
        )
        .bind::<diesel::sql_types::Text, _>(package_name)
        .load::<DownloadHistory>(conn)
        .await?;

        Ok(results)
    }

    pub async fn user_admins(
        conn: &mut AsyncPgConnection,
        package_id: i64,
    ) -> Result<Vec<i64>> {
        let admins = schema_admin::table
            .select(schema_admin::user_id)
            .filter(schema_admin::user_id.is_not_null())
            .filter(schema_admin::package.eq(package_id))
            .filter(schema_admin::revoked_at.is_null())
            .load::<Option<i64>>(conn)
            .await?
            .into_iter()
            .map(Option::unwrap)
            .collect();

        Ok(admins)
    }

    pub async fn org_admins(
        conn: &mut AsyncPgConnection,
        package_id: i64,
    ) -> Result<Vec<i64>> {
        let admins = schema_admin::table
            .select(schema_admin::org_id)
            .filter(schema_admin::org_id.is_not_null())
            .filter(schema_admin::package.eq(package_id))
            .filter(schema_admin::revoked_at.is_null())
            .load::<Option<i64>>(conn)
            .await?
            .into_iter()
            .map(Option::unwrap)
            .collect();

        Ok(admins)
    }

    pub async fn must_be_owner(
        conn: &mut AsyncPgConnection,
        api_key: &ApiKey,
        package_id: i64,
    ) -> Result<()> {
        let key_owner_id = api_key.owner_id();

        let is_admin = match key_owner_id {
            super::OwnerId::User(requestor_id) => {
                Package::user_admins(conn, package_id)
                    .await?
                    .into_iter()
                    .any(|user| user == requestor_id)
            },
            super::OwnerId::Org(requestor_id) => {
                Package::org_admins(conn, package_id)
                    .await?
                    .into_iter()
                    .any(|org| org == requestor_id)
            },
        };

        if !is_admin {
            return Err(Error::Unauthorized("Not package owner".into()));
        }

        Ok(())
    }

    pub async fn yank_version(
        conn: &mut AsyncPgConnection,
        api_key: &ApiKey,
        package_name: &str,
        version_str: &str,
    ) -> Result<()> {
        api_key.must_have_permission_for_package(package_name, &Permission::PublishPackage)?;

        let pkg = package::table
            .filter(package::name.eq(package_name))
            .first::<Package>(conn)
            .await
            .map_err(|e| {
                match e {
                    diesel::result::Error::NotFound => {
                        Error::NotFound(format!("Package '{}' not found", package_name))
                    },
                    e => Error::Database(e),
                }
            })?;

        Self::must_be_owner(conn, api_key, pkg.id).await?;

        let updated = diesel::update(version::table)
            .filter(version::package.eq(pkg.id))
            .filter(version::qualified_version.eq(version_str))
            .set(version::yanked_at.eq(Some(Utc::now())))
            .execute(conn)
            .await?;

        if updated == 0 {
            return Err(Error::NotFound(format!(
                "Version '{}' not found",
                version_str
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, diesel::QueryableByName, utoipa::ToSchema)]
pub struct DownloadHistory {
    #[diesel(sql_type = diesel::sql_types::Date)]
    pub day: NaiveDate,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub version: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub count: i32,
}

pub struct PackageUploadRequest<'a> {
    pub package_name: &'a str,
    pub version: &'a kintsu_manifests::version::Version,
    pub checksum: &'a str,
    pub description: Option<&'a str>,
    pub homepage: Option<&'a str>,
    pub license: &'a str,
    pub readme: &'a str,
    pub repository: &'a str,
    pub keywords: Vec<&'a str>,
}

impl PackageUploadRequest<'_> {
    pub async fn process(
        &self,
        conn: &mut AsyncPgConnection,
        api_key: &ApiKey,
    ) -> Result<Version> {
        conn.transaction(|conn| Box::pin(async move { self.process_raw(conn, api_key).await }))
            .await
    }

    async fn process_raw(
        &self,
        conn: &mut AsyncPgConnection,
        api_key: &super::api_key::ApiKey,
    ) -> Result<super::version::Version> {
        api_key.must_have_permission_for_package(self.package_name, &Permission::PublishPackage)?;

        let key_owner_id = api_key.owner_id();

        let pkg = package::table
            .filter(package::name.eq(self.package_name))
            .first::<Package>(conn)
            .await
            .optional()?;

        let package_id = if let Some(pkg) = pkg {
            Package::must_be_owner(conn, api_key, pkg.id).await?;

            pkg.id
        } else {
            let new_pkg = diesel::insert_into(package::table)
                .values(package::name.eq(self.package_name))
                .get_result::<Package>(conn)
                .await?;

            diesel::insert_into(schema_admin::table)
                .values((
                    schema_admin::package.eq(new_pkg.id),
                    schema_admin::user_id.eq(key_owner_id.user_id()),
                    schema_admin::org_id.eq(key_owner_id.org_id()),
                ))
                .execute(conn)
                .await?;

            new_pkg.id
        };

        let new_version = diesel::insert_into(version::table)
            .values((
                version::package.eq(package_id),
                version::qualified_version.eq(self.version.to_string()),
                version::checksum.eq(self.checksum),
                version::description.eq(self.description),
                version::homepage.eq(self.homepage),
                version::license.eq(self.license),
                version::readme.eq(self.readme),
                version::repository.eq(self.repository),
                version::keywords.eq(self.keywords.clone()),
                version::publishing_user_id.eq(key_owner_id.user_id()),
                version::publishing_org_id.eq(key_owner_id.org_id()),
            ))
            .get_result::<Version>(conn)
            .await?;

        Ok(new_version)
    }
}
