use crate::{
    Error, PackageStorage, Result,
    engine::{
        Entity, OrderDirection, OwnerId, PackageOrdering, PackageOrderingField, Page, Paginated,
        version::QualifiedPackageVersion,
    },
    entities::*,
};

use chrono::{NaiveDate, Utc};
use kintsu_manifests::{
    InvalidManifest,
    package::{Dependency, PathOrText},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, FromQueryResult, JoinType, NotSet, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, Select, Set, TransactionTrait,
    sea_query::Expr,
};

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema, FromQueryResult)]
pub struct DownloadHistory {
    pub day: NaiveDate,
    pub version: String,
    pub count: i32,
}

pub struct StagePublishPackage {
    pub package_name: String,
    pub version: kintsu_manifests::version::Version,
    pub homepage: Option<String>,
    pub description: Option<String>,
    pub license: String,
    pub readme: String,
    pub repository: String,
    pub keywords: Vec<String>,
    pub manifest_dependencies: Vec<i64>,
}

impl StagePublishPackage {
    pub async fn process<C: sea_orm::ConnectionTrait + TransactionTrait>(
        db: &C,
        principal: &super::principal::PrincipalIdentity,
        storage: std::sync::Arc<PackageStorage>,

        fs: kintsu_fs::memory::MemoryFileSystem,
        manifest: kintsu_manifests::package::PackageManifest,

        declarations: kintsu_parser::declare::DeclarationVersion,
        manifest_dependencies: Vec<i64>,
    ) -> Result<Version> {
        let package = &manifest.package;

        let description = PathOrText::text_opt(package.description.as_ref(), &fs)?;
        let license = PathOrText::text_opt(package.license.as_ref(), &fs)?
            .ok_or(InvalidManifest::PackageMissingLicense)?;
        let readme = PathOrText::text_opt(package.readme.as_ref(), &fs)?
            .ok_or(InvalidManifest::PackageMissingReadme)?;

        let package_name = package.name.clone();
        let version = package.version.clone();
        let homepage = package.homepage.clone();
        let repository = package
            .repository
            .clone()
            .ok_or(InvalidManifest::PackageMissingRepository)?;

        let keywords = package.keywords.clone();

        let pkg = PackageEntity::find()
            .filter(PackageColumn::Name.eq(&package_name))
            .one(db)
            .await?;

        let package_id = pkg.as_ref().map(|p| p.id);

        let auth_result = super::fluent::AuthCheck::new(db, principal)
            .package(&package_name, package_id)
            .can_publish()
            .await?;

        let event = principal.audit_event(
            super::events::EventType::PermissionProtected {
                permission: Permission::PublishPackage,
                resource: super::authorization::ResourceIdentifier::Package(
                    super::authorization::PackageResource {
                        name: package_name.clone(),
                        id: package_id,
                    },
                ),
            },
            &auth_result,
        )?;
        kintsu_registry_events::emit_event(event)?;

        auth_result.require()?;

        let key_owner_id = principal.owner_id();

        Ok(db
            .transaction::<_, Version, Error>(move |db| {
                Box::pin(async move {
                    if Version::exists(db, &package_name, &version.to_string()).await? {
                        return Err(Error::PackageVersionExists {
                            package: package_name.to_string(),
                            version: version.to_string(),
                        });
                    }

                    let package_id = if let Some(pkg) = pkg {
                        pkg.id
                    } else {
                        let new_pkg_model = PackageActiveModel {
                            id: NotSet,
                            name: Set(package_name.clone()),
                        };
                        let new_pkg = new_pkg_model.insert(db).await?;

                        let schema_role_active_model = SchemaRoleActiveModel {
                            id: NotSet,
                            package: Set(new_pkg.id),
                            user_id: Set(key_owner_id.user_id()),
                            org_id: Set(key_owner_id.org_id()),
                            role: Set(SchemaRoleType::Admin),
                            revoked_at: NotSet,
                        };
                        schema_role_active_model.insert(db).await?;

                        new_pkg.id
                    };

                    let checksums = storage
                        .store_package(&package_name, &version.to_string(), &fs, &declarations)
                        .await?;

                    let new_version_model = VersionActiveModel {
                        id: NotSet,
                        package: Set(package_id),
                        qualified_version: Set(version.clone()),
                        source_checksum: Set(checksums.source_checksum.value().to_string()),
                        declarations_checksum: Set(checksums
                            .declarations_checksum
                            .value()
                            .to_string()),
                        description: Set(description.clone()),
                        homepage: Set(homepage.map(|s| s.to_string())),
                        license: Set(license.clone()),
                        license_text: Set(String::new()),
                        readme: Set(readme.clone()),
                        repository: Set(repository.to_string()),
                        keywords: Set(keywords),
                        publishing_user_id: Set(key_owner_id.user_id()),
                        publishing_org_id: Set(key_owner_id.org_id()),
                        dependencies: Set(manifest_dependencies.clone()),
                        created_at: NotSet,
                        yanked_at: NotSet,
                    };

                    Ok(new_version_model.insert(db).await?)
                })
            })
            .await?)
    }

    pub async fn manifest_dependencies<C: sea_orm::ConnectionTrait>(
        db: &C,
        dependencies: &kintsu_manifests::package::NamedDependencies,
    ) -> Result<Vec<i64>> {
        let mut deps_to_check = Vec::new();
        let mut unresolved = vec![];

        for (name, dep) in dependencies {
            match dep {
                Dependency::Path(..) | Dependency::Git(..) => {
                    unresolved.push(InvalidManifest::UnresolvedDependency {
                        name: name.clone(),
                        version: None,
                    });
                },
                Dependency::PathWithRemote(pwr) => {
                    deps_to_check.push((name.clone(), pwr.remote.version.clone()));
                },
                Dependency::Remote(remote) => {
                    deps_to_check.push((name.clone(), remote.version.clone()));
                },
            }
        }

        if !unresolved.is_empty() {
            return Err(InvalidManifest::UnresolvedDependencies {
                sources: unresolved,
            }
            .into());
        }

        if deps_to_check.is_empty() {
            return Ok(vec![]);
        }

        use sea_orm::Condition;

        let mut conditions = Condition::any();
        for (name, version) in &deps_to_check {
            conditions = conditions.add(
                Condition::all()
                    .add(PackageColumn::Name.eq(name.as_str()))
                    .add(VersionColumn::QualifiedVersion.eq(version.to_string())),
            );
        }

        let found_deps: Vec<(i64, String, String)> = VersionEntity::find()
            .inner_join(PackageEntity)
            .filter(conditions)
            .select_only()
            .column(VersionColumn::Id)
            .column(PackageColumn::Name)
            .column(VersionColumn::QualifiedVersion)
            .into_tuple()
            .all(db)
            .await?;

        let found_set: std::collections::HashSet<(String, String)> = found_deps
            .iter()
            .map(|(_, name, ver)| (name.clone(), ver.clone()))
            .collect();

        let mut missing = vec![];
        for (name, version) in &deps_to_check {
            if !found_set.contains(&(name.to_string(), version.to_string())) {
                missing.push(InvalidManifest::UnresolvedDependency {
                    name: name.clone(),
                    version: Some(version.clone()),
                });
            }
        }

        if !missing.is_empty() {
            return Err(InvalidManifest::UnresolvedDependencies { sources: missing }.into());
        }

        Ok(found_deps
            .into_iter()
            .map(|(version_id, _, _)| version_id)
            .collect())
    }
}

impl Package {
    pub async fn by_id<C: sea_orm::ConnectionTrait>(
        db: &C,
        package_id: i64,
    ) -> Result<Option<Self>> {
        PackageEntity::find()
            .filter(PackageColumn::Id.eq(package_id))
            .one(db)
            .await
            .map_err(Into::into)
    }

    pub async fn by_name<C: sea_orm::ConnectionTrait>(
        db: &C,
        package_name: &str,
    ) -> Result<Option<Self>> {
        PackageEntity::find()
            .filter(PackageColumn::Name.eq(package_name))
            .one(db)
            .await
            .map_err(Into::into)
    }

    pub async fn get_package_download_count<C: sea_orm::ConnectionTrait>(
        db: &C,
        package_name: &str,
    ) -> Result<i64> {
        let count: Option<Option<i64>> = DownloadsEntity::find()
            .join(JoinType::InnerJoin, DownloadsRelation::Version.def())
            .join(JoinType::InnerJoin, VersionRelation::Package.def())
            .filter(PackageColumn::Name.eq(package_name))
            .select_only()
            .column_as(Expr::cust("SUM(coalesce(downloads.count, 0))"), "sum")
            .into_tuple()
            .one(db)
            .await?;

        Ok(count.flatten().unwrap_or(0) as i64)
    }

    pub async fn package_download_history<C: sea_orm::ConnectionTrait>(
        db: &C,
        package_name: &str,
    ) -> Result<Vec<DownloadHistory>> {
        use sea_orm::Statement;

        let raw_sql = r#"
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
        "#;

        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            raw_sql,
            vec![package_name.into()],
        );

        let results = DownloadHistory::find_by_statement(stmt)
            .all(db)
            .await?;

        Ok(results)
    }

    pub async fn user_admins<C: sea_orm::ConnectionTrait>(
        db: &C,
        package_id: i64,
    ) -> Result<Vec<i64>> {
        Ok(SchemaRoleEntity::find()
            .filter(SchemaRoleColumn::UserId.is_not_null())
            .filter(SchemaRoleColumn::Package.eq(package_id))
            .filter(SchemaRoleColumn::RevokedAt.is_null())
            .filter(SchemaRoleColumn::Role.eq(SchemaRoleType::Admin))
            .select_only()
            .column(SchemaRoleColumn::UserId)
            .into_tuple::<Option<i64>>()
            .all(db)
            .await?
            .into_iter()
            .flatten()
            .collect())
    }

    pub async fn org_admins<C: sea_orm::ConnectionTrait>(
        db: &C,
        package_id: i64,
    ) -> Result<Vec<i64>> {
        let admins = SchemaRoleEntity::find()
            .filter(SchemaRoleColumn::OrgId.is_not_null())
            .filter(SchemaRoleColumn::Package.eq(package_id))
            .filter(SchemaRoleColumn::RevokedAt.is_null())
            .filter(SchemaRoleColumn::Role.eq(SchemaRoleType::Admin))
            .select_only()
            .column(SchemaRoleColumn::OrgId)
            .into_tuple::<Option<i64>>()
            .all(db)
            .await?
            .into_iter()
            .flatten()
            .collect();

        Ok(admins)
    }

    pub async fn yank_version<C: sea_orm::ConnectionTrait>(
        db: &C,
        principal: &super::principal::PrincipalIdentity,
        package_name: &str,
        version_str: &str,
    ) -> Result<()> {
        let pkg = PackageEntity::find()
            .filter(PackageColumn::Name.eq(package_name))
            .one(db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Package '{}' not found", package_name)))?;

        let auth_result = super::fluent::AuthCheck::new(db, principal)
            .package(package_name, Some(pkg.id))
            .can_yank()
            .await?;

        let event = principal.audit_event(
            super::events::EventType::PermissionProtected {
                permission: Permission::YankPackage,
                resource: super::authorization::ResourceIdentifier::Package(
                    super::authorization::PackageResource {
                        name: package_name.to_string(),
                        id: Some(pkg.id),
                    },
                ),
            },
            &auth_result,
        )?;
        kintsu_registry_events::emit_event(event)?;

        auth_result.require()?;

        let version = VersionEntity::find()
            .filter(VersionColumn::Package.eq(pkg.id))
            .filter(VersionColumn::QualifiedVersion.eq(version_str))
            .one(db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Version '{}' not found", version_str)))?;

        let mut active_model: VersionActiveModel = version.into();
        active_model.yanked_at = Set(Some(Utc::now()));
        active_model.update(db).await?;

        Ok(())
    }

    fn select_with_ordering(
        query: Select<PackageEntity>,
        ordering: PackageOrdering,
    ) -> Select<PackageEntity> {
        match (ordering.field, ordering.direction) {
            (PackageOrderingField::Name, OrderDirection::Asc) => {
                query.order_by_asc(PackageColumn::Name)
            },
            (PackageOrderingField::Name, OrderDirection::Desc) => {
                query.order_by_desc(PackageColumn::Name)
            },
            // placeholder
            (PackageOrderingField::DownloadCount, OrderDirection::Asc) => {
                query.order_by_asc(PackageColumn::Id)
            },
            (PackageOrderingField::DownloadCount, OrderDirection::Desc) => {
                query.order_by_desc(PackageColumn::Id)
            },
        }
    }

    pub async fn list_packages<C: sea_orm::ConnectionTrait>(
        db: &C,
        page: Page,
        ordering: PackageOrdering,
    ) -> Result<Paginated<Package>> {
        let mut query = PackageEntity::find();

        query = Self::select_with_ordering(query, ordering);

        let paginator = query.paginate(db, page.size as u64);

        let (items, total_items) = tokio::try_join!(
            paginator.fetch_page(page.number.saturating_sub(1) as u64),
            paginator.num_items()
        )?;

        let total_items = total_items as i64;
        let total_pages = (total_items + page.size - 1) / page.size;
        let next_page = if page.number < total_pages {
            Some(page.number + 1)
        } else {
            None
        };

        Ok(Paginated {
            items,
            page,
            next_page,
            total_items,
            total_pages,
        })
    }

    pub async fn search_packages<C: sea_orm::ConnectionTrait>(
        db: &C,
        query_str: &str,
        page: Page,
        ordering: PackageOrdering,
    ) -> Result<Paginated<Package>> {
        let search_pattern = format!("%{}%", query_str);

        let mut query = PackageEntity::find().filter(PackageColumn::Name.like(&search_pattern));

        query = Self::select_with_ordering(query, ordering);

        let paginator = query.paginate(db, page.size as u64);

        let (items, total_items) = tokio::try_join!(
            paginator.fetch_page(page.number.saturating_sub(1) as u64),
            paginator.num_items()
        )?;

        let total_items = total_items as i64;
        let total_pages = (total_items + page.size - 1) / page.size;
        let next_page = if page.number < total_pages {
            Some(page.number + 1)
        } else {
            None
        };

        Ok(Paginated {
            items,
            page,
            next_page,
            total_items,
            total_pages,
        })
    }

    pub async fn list_package_versions<C: sea_orm::ConnectionTrait>(
        db: &C,
        package_name: &str,
        page: Page,
        filter_user_id: Option<i64>,
        filter_org_id: Option<i64>,
    ) -> Result<Paginated<QualifiedPackageVersion>> {
        let mut query = VersionEntity::find()
            .find_also(VersionEntity, PackageEntity)
            .filter(PackageColumn::Name.eq(package_name))
            .find_also(VersionEntity, UserEntity)
            .find_also(VersionEntity, OrgEntity);

        if let Some(user_id) = filter_user_id {
            query = query.filter(VersionColumn::PublishingUserId.eq(user_id));
        }
        if let Some(org_id) = filter_org_id {
            query = query.filter(VersionColumn::PublishingOrgId.eq(org_id));
        }

        query = query.order_by_desc(VersionColumn::CreatedAt);

        let paginator = query.paginate(db, page.size as u64);

        let (items, total_items) = tokio::try_join!(
            paginator.fetch_page(page.number.saturating_sub(1) as u64),
            paginator.num_items()
        )?;

        let total_items = total_items as i64;
        let total_pages = (total_items + page.size - 1) / page.size;
        let next_page = if page.number < total_pages {
            Some(page.number + 1)
        } else {
            None
        };

        Ok(Paginated {
            items: QualifiedPackageVersion::from_iter_tuple(items),
            page,
            next_page,
            total_items,
            total_pages,
        })
    }

    pub async fn get_package_publishers<C: sea_orm::ConnectionTrait>(
        db: &C,
        package_name: &str,
    ) -> Result<Vec<Entity>> {
        let pkg = PackageEntity::find()
            .filter(PackageColumn::Name.eq(package_name))
            .one(db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Package '{}' not found", package_name)))?;

        // todo: should i just do a distinct select in sql joining orgs & users ?
        // could also do a groupby / agg to get a list of distinct versions per publisher
        let (user_ids, org_ids): (Vec<Option<i64>>, Vec<Option<i64>>) = VersionEntity::find()
            .filter(VersionColumn::Package.eq(pkg.id))
            .select_only()
            .column(VersionColumn::PublishingUserId)
            .column(VersionColumn::PublishingOrgId)
            .into_tuple::<(Option<i64>, Option<i64>)>()
            .all(db)
            .await?
            .into_iter()
            .unzip();

        let user_ids: Vec<i64> = user_ids
            .into_iter()
            .flatten()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let org_ids: Vec<i64> = org_ids
            .into_iter()
            .flatten()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let (mut users, mut orgs) = tokio::try_join!(
            async {
                if !user_ids.is_empty() {
                    let users = UserEntity::find()
                        .filter(UserColumn::Id.is_in(user_ids))
                        .all(db)
                        .await?;
                    Ok::<_, Error>(
                        users
                            .into_iter()
                            .map(Entity::User)
                            .collect::<Vec<_>>(),
                    )
                } else {
                    Ok(vec![])
                }
            },
            async {
                if !org_ids.is_empty() {
                    let orgs = OrgEntity::find()
                        .filter(OrgColumn::Id.is_in(org_ids))
                        .all(db)
                        .await?;
                    Ok::<_, Error>(
                        orgs.into_iter()
                            .map(Entity::Org)
                            .collect::<Vec<_>>(),
                    )
                } else {
                    Ok(vec![])
                }
            }
        )?;

        let mut publishers = Vec::new();
        publishers.append(&mut users);
        publishers.append(&mut orgs);

        Ok(publishers)
    }

    pub async fn get_transitive_dependencies<C: sea_orm::ConnectionTrait>(
        db: &C,
        version_ids: Vec<i64>,
    ) -> Result<Vec<TransitiveDependency>> {
        Ok(
            TransitiveDependency::find_by_statement(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                format!(
                    "
                SELECT
                    p.name as package_name,
                    v.qualified_version,
                    v.source_checksum,
                    v.declarations_checksum
                FROM
                    get_dependency_tree($1::bigint []) dt
                inner join package p on dt.package_id = p.id
                inner join version v on dt.version_id = v.id"
                ),
                vec![version_ids.into()],
            ))
            .all(db)
            .await?,
        )
    }
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema, FromQueryResult)]
pub struct TransitiveDependency {
    pub package_name: String,
    pub qualified_version: String,

    pub source_checksum: String,
    pub declarations_checksum: String,
}

impl From<TransitiveDependency> for kintsu_registry_storage::BulkGetPackage {
    fn from(value: TransitiveDependency) -> Self {
        kintsu_registry_storage::BulkGetPackage {
            package_name: value.package_name,
            version: value.qualified_version,
            checksums: kintsu_registry_storage::StoredPackageChecksum::new(
                value.source_checksum,
                value.declarations_checksum,
            ),
        }
    }
}

impl From<TransitiveDependency> for kintsu_registry_storage::BulkGetSource {
    fn from(value: TransitiveDependency) -> Self {
        kintsu_registry_storage::BulkGetSource {
            package_name: value.package_name,
            version: value.qualified_version,
            checksum: value.source_checksum.into(),
        }
    }
}
