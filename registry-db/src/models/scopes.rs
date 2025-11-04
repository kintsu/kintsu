use diesel::{deserialize::FromSql, pg::Pg, serialize::ToSql, sql_types::Text};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
pub struct Scope {
    #[schema(example = "my-package-*")]
    pub pattern: String,
}

impl Scope {
    pub fn is_match(
        &self,
        package: &str,
    ) -> bool {
        self.pattern == package
    }
}

impl Into<String> for &Scope {
    fn into(self) -> String {
        self.pattern.clone()
    }
}

impl FromSql<Text, Pg> for Scope {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let s: String = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        Ok(Scope { pattern: s })
    }
}

impl ToSql<Text, Pg> for Scope {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, Pg>,
    ) -> diesel::serialize::Result {
        let s: &str = &self.pattern;
        <str as ToSql<Text, Pg>>::to_sql(s, out)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Permission {
    PublishPackage,
    DeletePackage,
    YankPackage,
    ChangeOwnership,
}

impl ToSql<Text, Pg> for Permission {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, Pg>,
    ) -> diesel::serialize::Result {
        let s: &'static str = self.into();
        <str as ToSql<Text, Pg>>::to_sql(s, out)
    }
}

impl FromSql<Text, Pg> for Permission {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let s: String = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        Permission::try_from(s.as_str()).map_err(|e| e.into())
    }
}

impl Into<&'static str> for &Permission {
    fn into(self) -> &'static str {
        match self {
            Permission::PublishPackage => "publish-package",
            Permission::DeletePackage => "delete-package",
            Permission::YankPackage => "yank-package",
            Permission::ChangeOwnership => "change-ownership",
        }
    }
}

impl TryFrom<&str> for Permission {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "publish-package" => Ok(Permission::PublishPackage),
            "delete-package" => Ok(Permission::DeletePackage),
            "yank-package" => Ok(Permission::YankPackage),
            "change-ownership" => Ok(Permission::ChangeOwnership),
            _ => Err(format!("invalid permission: {}", value)),
        }
    }
}
