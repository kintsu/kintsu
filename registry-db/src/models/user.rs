use crate::schema::users;
use diesel::{prelude::*, upsert::excluded};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Identifiable, AsChangeset, HasQuery, Serialize, Deserialize, Clone, ToSchema)]
#[diesel(table_name = users)]
pub struct User {
    #[schema(example = 1)]
    pub id: i64,
    #[schema(example = "foo@bar.com")]
    pub email: String,
    #[schema(example = 123)]
    pub gh_id: i32,
    #[schema(example = "foobar")]
    pub gh_login: String,
    #[schema(example = "https://avatars.githubusercontent.com/u/123?v=4")]
    pub gh_avatar: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub email: &'a str,
    pub gh_id: i32,
    pub gh_login: &'a str,
    pub gh_avatar: Option<&'a str>,
}

impl<'a> NewUser<'a> {
    pub async fn qualify(
        self,
        conn: &mut diesel_async::AsyncPgConnection,
    ) -> crate::Result<User> {
        Ok(diesel::insert_into(users::table)
            .values(&self)
            .on_conflict(users::gh_id)
            .do_update()
            .set((
                users::email.eq(excluded(users::email)),
                users::gh_login.eq(excluded(users::gh_login)),
                users::gh_avatar.eq(excluded(users::gh_avatar)),
            ))
            .get_result(conn)
            .await?)
    }
}
