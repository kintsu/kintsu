use crate::{
    Result,
    schema::{downloads, package, version},
};
use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

#[derive(Debug, Clone, serde::Serialize, diesel::QueryableByName)]
pub struct DownloadHistory {
    #[diesel(sql_type = diesel::sql_types::Date)]
    pub day: NaiveDate,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub version: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub count: i32,
}

pub async fn increment_download_count(
    conn: &mut AsyncPgConnection,
    version_id: i64,
) -> Result<()> {
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

pub async fn get_package_download_history(
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
        ORDER BY d.day DESC, v.qualified_version
        "#,
    )
    .bind::<diesel::sql_types::Text, _>(package_name)
    .load::<DownloadHistory>(conn)
    .await?;

    Ok(results)
}
