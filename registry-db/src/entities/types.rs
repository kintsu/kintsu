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
    #[sea_orm(string_value = "grant-organization-role")]
    GrantOrganizationRole,
    #[sea_orm(string_value = "grant-schema-role")]
    GrantSchemaRole,
}

impl Into<&'static str> for &Permission {
    fn into(self) -> &'static str {
        match self {
            Permission::PublishPackage => "publish-package",
            Permission::YankPackage => "yank-package",
            Permission::GrantOrganizationRole => "grant-organization-role",
            Permission::GrantSchemaRole => "grant-schema-role",
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

impl Into<String> for &Scope {
    fn into(self) -> String {
        self.0.to_string()
    }
}
