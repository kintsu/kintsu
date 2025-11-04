use crate::{
    Error, Result,
    models::{api_key::ApiKey, user::User},
    schema::{api_key, users},
    tokens::TokenHash,
};
use chrono::Utc;
use diesel::{pg::Pg, prelude::*};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use secrecy::{ExposeSecret, SecretString};

pub async fn get_auth_token(
    conn: &mut AsyncPgConnection,
    raw_token: &SecretString,
) -> Result<ApiKey> {
    let Some(token_hash) = TokenHash::from_token(raw_token.expose_secret()) else {
        return Err(Error::InvalidToken);
    };

    <ApiKey as HasQuery<Pg>>::query()
        .filter(api_key::key.eq(token_hash))
        .filter(api_key::expires.gt(Utc::now()))
        .filter(api_key::revoked_at.is_null())
        .select(ApiKey::as_select())
        .first(conn)
        .await
        .map_err(|_| Error::InvalidToken)
}

pub async fn list_user_tokens(
    conn: &mut AsyncPgConnection,
    user_id: i64,
) -> Result<Vec<ApiKey>> {
    api_key::table
        .filter(api_key::user_id.eq(user_id))
        .order(api_key::id.desc())
        .select(ApiKey::as_select())
        .load(conn)
        .await
        .map_err(Into::into)
}

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

pub async fn get_user_by_token(
    conn: &mut AsyncPgConnection,
    raw_token: &SecretString,
) -> Result<User> {
    let api_key_record = get_auth_token(conn, raw_token).await?;

    let user_id = api_key_record
        .user_id
        .ok_or_else(|| Error::Unauthorized("Token has no associated user".into()))?;

    let user = users::table
        .filter(users::id.eq(user_id))
        .first::<User>(conn)
        .await?;

    Ok(user)
}

pub async fn revoke_token(
    conn: &mut AsyncPgConnection,
    token_id: i64,
    user_id: i64,
) -> Result<()> {
    let updated = diesel::update(api_key::table)
        .filter(api_key::id.eq(token_id))
        .filter(api_key::user_id.eq(user_id))
        .set(api_key::revoked_at.eq(Some(Utc::now())))
        .execute(conn)
        .await?;

    if updated == 0 {
        return Err(Error::NotFound(
            "Token not found or not owned by user".into(),
        ));
    }

    Ok(())
}
