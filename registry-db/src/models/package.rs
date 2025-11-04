use diesel::prelude::*;
use serde::Serialize;

use crate::schema::package;

#[derive(Debug, Identifiable, AsChangeset, HasQuery, Serialize)]
#[diesel(table_name = package)]
pub struct Package {
    pub id: i64,
    pub name: String,
}
