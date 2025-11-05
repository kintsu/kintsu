use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use serde::Serialize;

use crate::{
    Error, Result,
    models::{Entity, package::Package, user::User},
    schema::{downloads, package, users, version},
};
use chrono::Utc;
use diesel_async::RunQueryDsl;

#[derive(Debug, Identifiable, Associations, HasQuery, Serialize, Clone, utoipa::ToSchema)]
#[diesel(
    table_name = version,
    check_for_backend(diesel::pg::Pg),
    primary_key(id),
    belongs_to(crate::models::package::Package, foreign_key = package),
    belongs_to(crate::models::org::Org, foreign_key = publishing_org_id),
    belongs_to(crate::models::user::User, foreign_key = publishing_user_id)
)]
pub struct Version {
    pub id: i64,
    pub package: i64,
    #[diesel(deserialize_as = String)]
    #[schema(value_type = String)]
    pub qualified_version: kintsu_manifests::version::Version,
    pub checksum: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: String,
    pub readme: String,
    pub repository: String,
    // #[diesel(deserialize_as = Keywords)]
    pub keywords: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub yanked_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publishing_org_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publishing_user_id: Option<i64>,
}

impl Version {
    pub async fn increment_download_count(
        conn: &mut AsyncPgConnection,
        version_id: i64,
    ) -> crate::Result<()> {
        let today = Utc::now().date_naive();

        diesel::insert_into(downloads::table)
            .values((
                downloads::version.eq(version_id),
                downloads::day.eq(today),
                downloads::count.eq(1),
            ))
            .on_conflict((downloads::version, downloads::day))
            .do_update()
            .set(downloads::count.eq(downloads::count + 1))
            .execute(conn)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct QualifiedPackageVersion {
    pub package: super::package::Package,
    pub version: Version,
    pub publisher: super::Entity,
}

pub struct LatestVersions {
    pub latest_version: kintsu_manifests::version::Version,
    pub latest_stable: Option<kintsu_manifests::version::Version>,
}

impl Version {
    pub async fn get_latest_versions(
        conn: &mut AsyncPgConnection,
        package_id: i64,
    ) -> Result<LatestVersions> {
        let mut all_versions: Vec<kintsu_manifests::version::Version> = version::table
            .select(version::qualified_version)
            .filter(version::package.eq(package_id))
            .filter(version::yanked_at.is_null())
            .load::<String>(conn)
            .await?
            .into_iter()
            .map(|v_str: String| {
                // Safe to unwrap since these versions are already validated on insert
                kintsu_manifests::version::Version::parse(&v_str).unwrap()
            })
            .collect();

        if all_versions.is_empty() {
            return Err(Error::NotFound("No versions found for package".into()));
        }

        // descending sort
        all_versions.sort_by(|a, b| b.cmp(&a));

        let latest_stable = all_versions.iter().find(|v| v.is_stable());

        Ok(LatestVersions {
            latest_version: all_versions[0].clone(),
            latest_stable: latest_stable.cloned(),
        })
    }

    pub async fn by_id(
        conn: &mut AsyncPgConnection,
        version_id: i64,
    ) -> Result<Version> {
        let ver = version::table
            .filter(version::id.eq(version_id))
            .first::<Version>(conn)
            .await
            .map_err(|e| {
                match e {
                    diesel::result::Error::NotFound => {
                        Error::NotFound(format!("Version with ID '{}' not found", version_id))
                    },
                    e => Error::Database(e),
                }
            })?;

        Ok(ver)
    }

    pub async fn get_package_version(
        conn: &mut AsyncPgConnection,
        package_name: &str,
        version_str: &str,
    ) -> Result<QualifiedPackageVersion> {
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

        let version = if version_str == "latest" {
            let latest_version = Self::get_latest_versions(conn, pkg.id).await?;
            latest_version
                .latest_stable
                .unwrap_or(latest_version.latest_version)
        } else {
            kintsu_manifests::version::Version::parse(version_str).map_err(|version_err| {
                Error::Validation(format!(
                    "Version '{}' is not a valid version: {}",
                    version_str, version_err
                ))
            })?
        };

        let ver = Version::query()
            .filter(version::package.eq(pkg.id))
            .filter(version::qualified_version.eq(version.to_string()))
            .first(conn)
            .await
            .map_err(|e| {
                match e {
                    diesel::result::Error::NotFound => {
                        Error::NotFound(format!("Version '{}' not found", version_str))
                    },
                    e => Error::Database(e),
                }
            })?;

        let publisher = if let Some(user_id) = ver.publishing_user_id {
            let user = users::table
                .filter(users::id.eq(user_id))
                .first::<User>(conn)
                .await?;
            Entity::User(user)
        } else if let Some(org_id) = ver.publishing_org_id {
            let org = crate::schema::org::table
                .filter(crate::schema::org::id.eq(org_id))
                .first::<crate::models::org::Org>(conn)
                .await?;
            Entity::Org(org)
        } else {
            return Err(Error::Database(diesel::result::Error::NotFound));
        };

        Ok(QualifiedPackageVersion {
            package: pkg,
            version: ver,
            publisher,
        })
    }
}
