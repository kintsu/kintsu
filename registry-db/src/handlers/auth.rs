use crate::{Result, models::user::User};
use diesel_async::AsyncPgConnection;

pub async fn create_or_update_user_from_oauth(
    conn: &mut AsyncPgConnection,
    gh_id: i32,
    gh_login: &str,
    gh_avatar: Option<&str>,
    email: &str,
) -> Result<User> {
    let new_user = crate::models::user::NewUser {
        gh_id,
        gh_login,
        gh_avatar,
        email,
    };

    Ok(new_user.qualify(conn).await?)
}
