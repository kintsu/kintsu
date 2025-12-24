use crate::{engine::OwnerId, entities::*};

#[derive(Debug, Clone)]
pub enum PrincipalIdentity {
    UserSession { user: User },
    UserApiKey { user: User, key: ApiKey },
    OrgApiKey { org: Org, key: ApiKey },
}

impl PrincipalIdentity {
    pub fn owner_id(&self) -> OwnerId {
        match self {
            Self::UserSession { user, .. } => OwnerId::User(user.id),
            Self::UserApiKey { user, .. } => OwnerId::User(user.id),
            Self::OrgApiKey { org, .. } => OwnerId::Org(org.id),
        }
    }

    pub fn api_key(&self) -> Option<&ApiKey> {
        match self {
            Self::UserSession { .. } => None,
            Self::UserApiKey { key, .. } => Some(key),
            Self::OrgApiKey { key, .. } => Some(key),
        }
    }

    pub fn user(&self) -> Option<&User> {
        match self {
            Self::UserSession { user, .. } => Some(user),
            Self::UserApiKey { user, .. } => Some(user),
            Self::OrgApiKey { .. } => None,
        }
    }

    pub fn org(&self) -> Option<&Org> {
        match self {
            Self::OrgApiKey { org, .. } => Some(org),
            _ => None,
        }
    }

    pub fn is_session(&self) -> bool {
        matches!(self, Self::UserSession { .. })
    }

    pub fn is_api_key(&self) -> bool {
        !self.is_session()
    }

    pub fn principal_type(&self) -> kintsu_registry_auth::PrincipalType {
        use kintsu_registry_auth::PrincipalType;
        match self {
            Self::UserSession { .. } => PrincipalType::UserSession,
            Self::UserApiKey { .. } => PrincipalType::UserApiKey,
            Self::OrgApiKey { .. } => PrincipalType::OrgApiKey,
        }
    }

    pub fn principal_id(&self) -> i64 {
        match self.owner_id() {
            OwnerId::User(id) => id,
            OwnerId::Org(id) => id,
        }
    }

    pub fn audit_event(
        &self,
        event_type: kintsu_registry_auth::AuditEventType,
        result: &kintsu_registry_auth::AuthorizationResult,
    ) -> kintsu_registry_auth::AuditEvent {
        kintsu_registry_auth::AuditEvent::builder()
            .timestamp(chrono::Utc::now())
            .principal_type(self.principal_type())
            .principal_id(self.principal_id())
            .event_type(event_type)
            .allowed(result.allowed)
            .reason(result.reason.clone())
            .policy_checks(result.checks.clone())
            .build()
    }
}
