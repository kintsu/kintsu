use crate::schema::org_admin;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

#[derive(Insertable, Associations, Identifiable, Debug, Clone, utoipa::ToSchema)]
#[diesel(
    table_name = org_admin,
    check_for_backend(diesel::pg::Pg),
    primary_key(org_id, user_id),
    belongs_to(crate::models::org::Org, foreign_key = org_id),
    belongs_to(crate::models::user::User, foreign_key = user_id)
)]
pub struct OrgAdmin {
    pub org_id: i64,
    pub user_id: i64,
    pub revoked_at: Option<DateTime<Utc>>,
}
