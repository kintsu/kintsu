use super::Entity;
use crate::{
    Error, Result,
    entities::{org::Entity as OrgEntity, *},
};
use chrono::Utc;
use kintsu_manifests::version::{VersionExt, VersionSerde, parse_version};
use sea_orm::{
    ColumnTrait, EntityTrait, ExprTrait, Order, QueryFilter, QueryOrder, QuerySelect, Set,
    prelude::Expr,
    sea_query::{OnConflict, SimpleExpr},
};
use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct QualifiedPackageVersion {
    pub package: Package,
    pub version: Version,
    pub publisher: Entity,
}

impl From<(Version, Option<Package>, Option<User>, Option<Org>)> for QualifiedPackageVersion {
    fn from(tuple: (Version, Option<Package>, Option<User>, Option<Org>)) -> Self {
        let (version, package, user, org) = tuple;
        QualifiedPackageVersion {
            package: package.expect("postgres constraint ensures package exists"),
            version,
            publisher: user
                .map(Entity::User)
                .or(org.map(Entity::Org))
                .expect("postgres constraint ensures publisher exists"),
        }
    }
}

impl QualifiedPackageVersion {
    pub fn from_iter_tuple<
        I: IntoIterator<Item = (Version, Option<Package>, Option<User>, Option<Org>)>,
    >(
        tuple: I
    ) -> Vec<Self> {
        tuple.into_iter().map(Self::from).collect()
    }
}

pub struct LatestVersions {
    pub latest_version: VersionSerde,
    pub latest_stable: Option<VersionSerde>,
}

impl Version {
    pub async fn by_id(
        db: &sea_orm::DatabaseConnection,
        version_id: i64,
    ) -> Result<Self> {
        VersionEntity::find_by_id(version_id)
            .one(db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Version with ID '{}' not found", version_id)))
    }

    pub async fn by_name_and_version(
        db: &sea_orm::DatabaseConnection,
        package_name: &str,
        version_str: &str,
    ) -> Result<Self> {
        let ver = VersionEntity::find()
            .inner_join(PackageEntity)
            .filter(PackageColumn::Name.eq(package_name))
            .filter(VersionColumn::QualifiedVersion.eq(version_str))
            .one(db)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "Version '{}' for package '{}' not found",
                    version_str, package_name
                ))
            })?;

        Ok(ver)
    }

    pub async fn exists<C: sea_orm::ConnectionTrait>(
        db: &C,
        package_name: &str,
        version_str: &str,
    ) -> Result<bool> {
        // ✅ Use LIMIT 1 instead of COUNT for existence check
        let exists = VersionEntity::find()
            .inner_join(PackageEntity)
            .filter(PackageColumn::Name.eq(package_name))
            .filter(VersionColumn::QualifiedVersion.eq(version_str))
            .limit(1)
            .one(db)
            .await?
            .is_some();

        Ok(exists)
    }

    pub async fn increment_download_count(
        db: &sea_orm::DatabaseConnection,
        version_id: i64,
    ) -> Result<()> {
        use crate::entities::downloads::{ActiveModel, Column, Entity as DownloadsEntity};
        use sea_orm::sea_query::Alias;

        let today = Utc::now().date_naive();

        let active_model = ActiveModel {
            version: Set(version_id),
            day: Set(today),
            count: Set(1),
        };

        // Use value() to set count = count + 1 via raw SQL expression
        DownloadsEntity::insert(active_model)
            .on_conflict(
                OnConflict::columns([Column::Version, Column::Day])
                    .value(
                        Column::Count,
                        Expr::col((Alias::new("downloads"), Column::Count)).add(1),
                    )
                    .to_owned(),
            )
            .exec(db)
            .await?;

        Ok(())
    }

    pub async fn dependents(
        &self,
        db: &sea_orm::DatabaseConnection,
    ) -> Result<Vec<QualifiedPackageVersion>> {
        let results = QualifiedPackageVersion::from_iter_tuple(
            VersionEntity::find()
                .filter(SimpleExpr::cust_with_values(
                    "$1 = ANY(dependencies)",
                    [sea_orm::Value::BigInt(Some(self.id))],
                ))
                .find_also_related(PackageEntity)
                .order_by(crate::entities::package::Column::Name, Order::Asc)
                .find_also(VersionEntity, UserEntity)
                .find_also(VersionEntity, OrgEntity)
                .all(db)
                .await?,
        );

        Ok(results)
    }

    pub async fn dependencies(
        &self,
        db: &sea_orm::DatabaseConnection,
    ) -> Result<Vec<QualifiedPackageVersion>> {
        if self.dependencies.is_empty() {
            return Ok(vec![]);
        }

        let versions = QualifiedPackageVersion::from_iter_tuple(
            VersionEntity::find()
                .filter(VersionColumn::Id.is_in(self.dependencies.clone()))
                .find_also_related(PackageEntity)
                .order_by(crate::entities::package::Column::Name, Order::Asc)
                .order_by(VersionColumn::QualifiedVersion, Order::Desc)
                .find_also(VersionEntity, UserEntity)
                .find_also(VersionEntity, OrgEntity)
                .all(db)
                .await?,
        );

        Ok(versions)
    }

    pub async fn get_latest_versions(
        db: &sea_orm::DatabaseConnection,
        package_id: i64,
    ) -> Result<LatestVersions> {
        let latest_version_str: Option<String> = VersionEntity::find()
            .filter(VersionColumn::Package.eq(package_id))
            .filter(VersionColumn::YankedAt.is_null())
            .select_only()
            .column(VersionColumn::QualifiedVersion)
            .order_by_desc(VersionColumn::Id)
            .limit(1)
            .into_tuple()
            .one(db)
            .await?;

        let latest_version_str = latest_version_str
            .ok_or_else(|| Error::NotFound("No versions found for package".into()))?;

        let latest_version = VersionSerde(
            parse_version(&latest_version_str).expect("version already validated on insert"),
        );

        // If latest is stable, we're done
        if latest_version.is_stable() {
            return Ok(LatestVersions {
                latest_version: latest_version.clone(),
                latest_stable: Some(latest_version),
            });
        }

        let all_versions: Vec<String> = VersionEntity::find()
            .filter(VersionColumn::Package.eq(package_id))
            .filter(VersionColumn::YankedAt.is_null())
            .select_only()
            .column(VersionColumn::QualifiedVersion)
            .into_tuple()
            .all(db)
            .await?;

        let mut latest_stable = all_versions
            .into_iter()
            .map(|v_str| VersionSerde(parse_version(&v_str).expect("validated")))
            .collect::<Vec<_>>();

        latest_stable.sort_by(|a, b| b.cmp(a)); // Descending order

        let latest_stable = latest_stable
            .into_iter()
            .find(|v| v.is_stable());

        Ok(LatestVersions {
            latest_version,
            latest_stable,
        })
    }

    pub async fn get_package_version(
        db: &sea_orm::DatabaseConnection,
        package_name: &str,
        version_str: &str,
    ) -> Result<QualifiedPackageVersion> {
        let pkg = PackageEntity::find()
            .filter(PackageColumn::Name.eq(package_name))
            .one(db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Package '{}' not found", package_name)))?;

        let version = if version_str == "latest" {
            let latest_version = Self::get_latest_versions(db, pkg.id).await?;
            latest_version
                .latest_stable
                .unwrap_or(latest_version.latest_version)
        } else {
            VersionSerde(parse_version(version_str).map_err(|version_err| {
                Error::Validation(format!(
                    "Version '{}' is not a valid version: {}",
                    version_str, version_err
                ))
            })?)
        };

        Ok(QualifiedPackageVersion::from(
            VersionEntity::find()
                .filter(VersionColumn::Package.eq(pkg.id))
                .filter(VersionColumn::QualifiedVersion.eq(version))
                .find_also(VersionEntity, PackageEntity)
                .find_also(VersionEntity, UserEntity)
                .find_also(VersionEntity, OrgEntity)
                .one(db)
                .await?
                .ok_or_else(|| Error::NotFound(format!("Version '{}' not found", version_str)))?,
        ))
    }
}
