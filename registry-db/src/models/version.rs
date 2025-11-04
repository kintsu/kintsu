use crate::schema::version;
use diesel::prelude::*;
use serde::Serialize;

#[derive(Debug, Identifiable, HasQuery, Serialize, Clone)]
#[diesel(
    table_name = version,
    check_for_backend(diesel::pg::Pg),
    primary_key(id),
    belongs_to(crate::models::package::Package, foreign_key = package),
    belongs_to(crate::models::org::Org, foreign_key = publishing_org),
    belongs_to(crate::models::user::User, foreign_key = publishing_user)
)]
pub struct Version {
    pub id: i64,
    pub package: i64,
    #[diesel(deserialize_as = String)]
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
    pub async fn select_by_package_and_version(
        package_id: i64,
        version: &str,
    ) -> QueryResult<Self> {
        todo!()
    }
}

#[derive(Debug, Serialize)]
pub struct VersionWithPublisher {
    #[serde(flatten)]
    pub version: Version,
    pub publisher: super::Entity,
}

pub struct LatestVersions {
    pub latest_version: Version,
    pub latest_stable: Option<Version>,
}
