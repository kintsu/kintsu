use crate::{Error, Result, entities::*};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, Order, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum FavouriteEntity {
    Package(crate::entities::Package),
    Org(crate::entities::Org),
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct FavouriteWithEntity {
    pub id: i64,
    #[serde(flatten)]
    pub entity: FavouriteEntity,
}

pub async fn list_favourites(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    page: crate::engine::Page,
) -> Result<crate::engine::Paginated<FavouriteWithEntity>> {
    let query = UserFavouriteEntity::find()
        .filter(UserFavouriteColumn::UserId.eq(user_id))
        .find_also_related(PackageEntity)
        .find_also_related(OrgEntity)
        .order_by(UserFavouriteColumn::CreatedAt, Order::Desc);

    let paginator = query.paginate(db, page.size as u64);

    let (favourites, total_items) = tokio::try_join!(
        paginator.fetch_page(page.number.saturating_sub(1) as u64),
        paginator.num_items()
    )?;

    let mut items = Vec::new();

    for (fav, package_opt, org_opt) in favourites {
        if let Some(package) = package_opt {
            items.push(FavouriteWithEntity {
                id: fav.id,
                entity: FavouriteEntity::Package(package),
            });
        } else if let Some(org) = org_opt {
            items.push(FavouriteWithEntity {
                id: fav.id,
                entity: FavouriteEntity::Org(org),
            });
        }
    }

    let total_items = total_items as i64;
    let total_pages = (total_items + page.size - 1) / page.size;
    let next_page = if page.number < total_pages {
        Some(page.number + 1)
    } else {
        None
    };

    Ok(crate::engine::Paginated {
        items,
        page,
        next_page,
        total_items,
        total_pages,
    })
}

#[derive(Debug, Clone, Copy)]
pub enum FavouriteTarget {
    Package(i64),
    Org(i64),
}

pub async fn create_favourite(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    target: FavouriteTarget,
) -> Result<UserFavourite> {
    match target {
        FavouriteTarget::Package(package_id) => {
            let exists = PackageEntity::find()
                .filter(PackageColumn::Id.eq(package_id))
                .count(db)
                .await?
                > 0;

            if !exists {
                return Err(Error::NotFound(format!(
                    "Package with id {} not found",
                    package_id
                )));
            }
        },
        FavouriteTarget::Org(org_id) => {
            let exists = OrgEntity::find()
                .filter(OrgColumn::Id.eq(org_id))
                .count(db)
                .await?
                > 0;

            if !exists {
                return Err(Error::NotFound(format!(
                    "Organization with id {} not found",
                    org_id
                )));
            }
        },
    }

    // Create the favourite
    let (package_id, org_id) = match target {
        FavouriteTarget::Package(id) => (Some(id), None),
        FavouriteTarget::Org(id) => (None, Some(id)),
    };

    let active_model = UserFavouriteActiveModel {
        id: NotSet,
        created_at: NotSet,
        user_id: Set(user_id),
        package_id: Set(package_id),
        org_id: Set(org_id),
    };

    match UserFavouriteEntity::insert(active_model)
        .exec_with_returning(db)
        .await
    {
        Ok(favourite) => Ok(favourite),
        Err(e) => Err(e.into()),
    }
}

pub async fn delete_favourite(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    target: FavouriteTarget,
) -> Result<()> {
    let favourite = UserFavouriteEntity::find()
        .filter(UserFavouriteColumn::UserId.eq(user_id))
        .filter(match target {
            FavouriteTarget::Package(id) => UserFavouriteColumn::PackageId.eq(id),
            FavouriteTarget::Org(id) => UserFavouriteColumn::OrgId.eq(id),
        })
        .one(db)
        .await?;

    match favourite {
        Some(fav) => {
            let active_model: UserFavouriteActiveModel = fav.into();
            active_model.delete(db).await?;
            Ok(())
        },
        None => {
            let target_name = match target {
                FavouriteTarget::Package(id) => format!("package {}", id),
                FavouriteTarget::Org(id) => format!("organization {}", id),
            };
            Err(Error::NotFound(format!(
                "Favourite for {} not found",
                target_name
            )))
        },
    }
}

pub async fn package_favourite_count(
    db: &sea_orm::DatabaseConnection,
    package_id: i64,
) -> Result<u64> {
    let count = UserFavouriteEntity::find()
        .filter(UserFavouriteColumn::PackageId.eq(package_id))
        .count(db)
        .await?;

    Ok(count)
}

pub async fn org_favourite_count(
    db: &sea_orm::DatabaseConnection,
    org_id: i64,
) -> Result<u64> {
    let count = UserFavouriteEntity::find()
        .filter(UserFavouriteColumn::OrgId.eq(org_id))
        .count(db)
        .await?;

    Ok(count)
}
