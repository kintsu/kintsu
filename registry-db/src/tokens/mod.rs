// this is unashamedly adapted from crates.io's token implementation with the following changes:
// - localized token prefix
// - uses sha256 crate for hashing instead of sha2 crate
// https://github.com/rust-lang/crates.io/blob/main/crates/crates_io_database/src/utils/token.rs

use diesel::{
    AsExpression, FromSqlRow, deserialize::FromSql, pg::Pg, serialize::ToSql, sql_types::Bytea,
};
use rand::{Rng, distr::Alphanumeric};
use secrecy::{ExposeSecret, SecretSlice, SecretString};
use sha256::digest;

const TOKEN_LENGTH: usize = 64;
const TOKEN_PREFIX: &str = "kintsu_";

#[derive(FromSqlRow, AsExpression)]
#[diesel(sql_type = Bytea)]
pub struct TokenHash(SecretSlice<u8>);

impl std::fmt::Debug for TokenHash {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_tuple("TokenHash")
            .field(&"[REDACTED]")
            .finish()
    }
}

impl TokenHash {
    pub fn from_token(token: &str) -> Option<Self> {
        if !token.starts_with(TOKEN_PREFIX) || token.len() != TOKEN_LENGTH {
            return None;
        }
        Some(Self(Self::hash(token)))
    }

    fn hash(plaintext: &str) -> SecretSlice<u8> {
        let hash = digest(plaintext);
        SecretSlice::new(hash.as_bytes().into())
    }
}

impl ToSql<Bytea, Pg> for TokenHash {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, Pg>,
    ) -> diesel::serialize::Result {
        diesel::serialize::ToSql::<Bytea, Pg>::to_sql(self.0.expose_secret(), out)
    }
}

impl FromSql<Bytea, Pg> for TokenHash {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let vec: Vec<u8> = diesel::deserialize::FromSql::<Bytea, Pg>::from_sql(bytes)?;
        Ok(TokenHash(SecretSlice::new(vec.into())))
    }
}

pub struct RawToken(SecretString);

impl RawToken {
    pub fn generate() -> Self {
        Self(generate_token().into())
    }

    pub fn hashed(&self) -> TokenHash {
        TokenHash(TokenHash::hash(self.0.expose_secret()))
    }
}

impl ExposeSecret<str> for RawToken {
    fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

fn generate_token() -> String {
    let rand_string: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(TOKEN_LENGTH - TOKEN_PREFIX.len())
        .map(char::from)
        .collect();
    format!("{}{}", TOKEN_PREFIX, rand_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generated_and_parse() {
        let token = RawToken::generate();
        assert!(
            token
                .expose_secret()
                .starts_with(TOKEN_PREFIX)
        );
        assert_eq!(
            token.hashed().0.expose_secret(),
            digest(token.expose_secret().as_bytes()).as_bytes()
        );

        let parsed =
            TokenHash::from_token(token.expose_secret()).expect("failed to parse back the token");
        assert_eq!(parsed.0.expose_secret(), token.hashed().0.expose_secret());
    }

    #[test]
    fn test_parse_not_our_token() {
        assert!(TokenHash::from_token("nokind").is_none());
    }
}
