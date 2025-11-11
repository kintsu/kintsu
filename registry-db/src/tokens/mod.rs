// this is unashamedly adapted from crates.io's token implementation with the following changes:
// - localized token prefix
// - uses sha256 crate for hashing instead of sha2 crate
// https://github.com/rust-lang/crates.io/blob/main/crates/crates_io_database/src/utils/token.rs

use rand::{Rng, distr::Alphanumeric};
use sea_orm::{prelude::StringLen, sea_query::ValueTypeErr};
use secrecy::{ExposeSecret, SecretSlice, SecretString};
use sha256::digest;

const TOKEN_LENGTH: usize = 64;
const TOKEN_PREFIX: &str = "kintsu_";

pub const MAX_TOKEN_HEADER_LENGTH: usize = TOKEN_LENGTH + "Bearer ".len();

#[derive(Clone)]
pub struct TokenHash(SecretSlice<u8>);

impl PartialEq for TokenHash {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.0.expose_secret() == other.0.expose_secret()
    }
}

impl Eq for TokenHash {}

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

impl From<TokenHash> for sea_orm::Value {
    fn from(token_hash: TokenHash) -> Self {
        sea_orm::Value::Bytes(Some(token_hash.0.expose_secret().to_vec()))
    }
}

impl sea_orm::TryGetable for TokenHash {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        index: I,
    ) -> Result<Self, sea_orm::TryGetError> {
        let v: Vec<u8> = sea_orm::TryGetable::try_get_by(res, index)?;
        Ok(TokenHash(SecretSlice::new(v.into())))
    }
}

impl sea_orm::sea_query::ValueType for TokenHash {
    fn type_name() -> String {
        std::any::type_name::<Self>().to_string()
    }

    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::Bytes
    }

    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::sea_query::ColumnType::VarBinary(StringLen::None)
    }

    fn try_from(v: sea_orm::Value) -> Result<Self, ValueTypeErr> {
        match v {
            sea_orm::Value::Bytes(Some(b)) => Ok(TokenHash(SecretSlice::new(b.into()))),
            _ => Err(ValueTypeErr),
        }
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
