use sea_orm::entity::prelude::*;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    utoipa :: ToSchema,
    serde :: Serialize,
    serde :: Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "org_role_type")]
pub enum OrgRoleType {
    #[sea_orm(string_value = "admin")]
    Admin,
    #[sea_orm(string_value = "member")]
    Member,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    utoipa :: ToSchema,
    serde :: Serialize,
    serde :: Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "schema_role_type")]
pub enum SchemaRoleType {
    #[sea_orm(string_value = "admin")]
    Admin,
    #[sea_orm(string_value = "author")]
    Author,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    utoipa :: ToSchema,
    serde :: Serialize,
    serde :: Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "permission")]
#[serde(rename_all = "kebab-case")]
pub enum Permission {
    #[sea_orm(string_value = "publish-package")]
    PublishPackage,
    #[sea_orm(string_value = "yank-package")]
    YankPackage,
    #[sea_orm(string_value = "grant-schema-role")]
    GrantSchemaRole,
    #[sea_orm(string_value = "revoke-schema-role")]
    RevokeSchemaRole,
    #[sea_orm(string_value = "grant-org-role")]
    GrantOrgRole,
    #[sea_orm(string_value = "revoke-org-role")]
    RevokeOrgRole,
    #[sea_orm(string_value = "create-org-token")]
    CreateOrgToken,
    #[sea_orm(string_value = "revoke-org-token")]
    RevokeOrgToken,
    #[sea_orm(string_value = "list-org-token")]
    ListOrgToken,
    #[sea_orm(string_value = "create-personal-token")]
    CreatePersonalToken,
    #[sea_orm(string_value = "revoke-personal-token")]
    RevokePersonalToken,
}

impl Permission {
    pub fn is_api_key_only(&self) -> bool {
        matches!(self, Self::PublishPackage | Self::YankPackage)
    }
}

impl From<&Permission> for &'static str {
    fn from(val: &Permission) -> Self {
        match val {
            Permission::PublishPackage => "publish-package",
            Permission::YankPackage => "yank-package",
            Permission::GrantSchemaRole => "grant-schema-role",
            Permission::RevokeSchemaRole => "revoke-schema-role",
            Permission::GrantOrgRole => "grant-org-role",
            Permission::RevokeOrgRole => "revoke-org-role",
            Permission::CreateOrgToken => "create-org-token",
            Permission::RevokeOrgToken => "revoke-org-token",
            Permission::ListOrgToken => "list-org-token",
            Permission::CreatePersonalToken => "create-personal-token",
            Permission::RevokePersonalToken => "revoke-personal-token",
        }
    }
}

impl std::fmt::Display for Permission {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let s: &'static str = self.into();
        write!(f, "{}", s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[schema(
    example = "my-package-*",
    description = "A scope pattern that applies to all packages uncreated or owned by user starting with 'my-package-'"
)]
pub struct Scope(String);

impl Scope {
    /// Create a new Scope from a string pattern
    pub fn new(pattern: impl Into<String>) -> Self {
        Self(pattern.into())
    }

    pub fn pattern(&self) -> &str {
        &self.0
    }

    pub fn is_valid(scope: &str) -> bool {
        let wildcard_count = scope.matches('*').count();
        if wildcard_count > 1 {
            return false;
        }
        if wildcard_count == 1 && !scope.ends_with('*') {
            return false;
        }
        true
    }

    pub fn is_match(
        scope: &str,
        package_name: &str,
    ) -> bool {
        if scope.ends_with('*') {
            let prefix = scope.trim_end_matches('*');
            package_name.starts_with(prefix)
        } else {
            scope == package_name
        }
    }
}

impl From<&Scope> for String {
    fn from(val: &Scope) -> Self {
        ToString::to_string(&val.0)
    }
}
