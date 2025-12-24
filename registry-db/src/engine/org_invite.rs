use crate::{entities::*, *};
use chrono::Utc;
use sea_orm::{ActiveValue::*, entity::*};

pub async fn respond_to_invitation<C: sea_orm::ConnectionTrait>(
    db: &C,
    principal: &super::principal::PrincipalIdentity,
    invitation_id: i64,
    accepted: bool,
) -> Result<()> {
    use kintsu_registry_auth::AuditEvent;

    if !principal.is_session() {
        return Err(Error::Validation(
            "Invitation response requires user session (not API key)".into(),
        ));
    }

    let user = principal
        .user()
        .ok_or_else(|| Error::Internal("Session principal missing user data".into()))?;

    let invitation = OrgInvitationEntity::find_by_id(invitation_id)
        .one(db)
        .await?
        .ok_or_else(|| Error::NotFound("Invitation not found".into()))?;

    if invitation.accepted_at.is_some() || invitation.revoked_at.is_some() {
        return Err(Error::Validation("Invitation already responded to".into()));
    }

    let mut active_model: OrgInvitationActiveModel = invitation.clone().into();
    if accepted {
        active_model.accepted_at = Set(Some(Utc::now()));

        let role_model = OrgRoleActiveModel {
            org_id: Set(invitation.org_id),
            user_id: Set(user.id),
            role: Set(invitation.role),
            revoked_at: NotSet,
        };
        role_model.insert(db).await?;
    } else {
        active_model.revoked_at = Set(Some(Utc::now()));
    }
    active_model.update(db).await?;

    let event = AuditEvent::builder()
        .timestamp(chrono::Utc::now())
        .principal_type(principal.principal_type())
        .principal_id(principal.principal_id())
        .event_type(
            kintsu_registry_auth::AuditEventType::OrganizationInviteResponse {
                invitation_id,
                org_id: invitation.org_id,
                accepted,
            },
        )
        .allowed(true)
        .reason("Session-only operation - latent user permission".to_string())
        .policy_checks(vec![])
        .build();

    kintsu_registry_events::emit_event(event)?;

    Ok(())
}
