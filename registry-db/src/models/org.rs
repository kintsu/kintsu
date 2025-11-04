use crate::schema::org;
use diesel::prelude::*;
use serde::Serialize;

#[derive(Debug, Identifiable, AsChangeset, HasQuery, Serialize, Clone)]
#[diesel(table_name = org)]
pub struct Org {
    pub id: i64,
    pub name: String,
    pub gh_id: i32,
}
