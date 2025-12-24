//! Organization Invitation Tests
//!
//! Tests for registry-db/src/engine/org_invite.rs
//! Covers responding to org invitations.

mod common;

use common::fixtures;
use kintsu_registry_db::{
    Error,
    engine::{PrincipalIdentity, org_invite::respond_to_invitation},
    entities::*,
    tst::TestDbCtx,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

// Re-import column types for filtering
use kintsu_registry_db::entities::org_role::Column as OrgRoleColumn;

async fn create_api_key_principal(
    ctx: &TestDbCtx,
    user: &User,
    perms: Vec<Permission>,
) -> PrincipalIdentity {
    let session = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(vec!["*"])
        .permissions(perms)
        .insert(&ctx.conn, &session)
        .await
        .expect("Failed to create API key");

    let api_key = ApiKey {
        id: one_time.api_key.id,
        description: one_time.api_key.description,
        expires: one_time.api_key.expires,
        scopes: one_time.api_key.scopes,
        permissions: one_time.api_key.permissions,
        user_id: one_time.api_key.user_id,
        org_id: one_time.api_key.org_id,
        last_used_at: None,
        revoked_at: None,
    };

    PrincipalIdentity::UserApiKey {
        user: user.clone(),
        key: api_key,
    }
}

#[tokio::test]
async fn accept_invitation_success() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin user");

    let invitee = fixtures::user()
        .gh_login("invitee")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create invitee");

    let org = fixtures::org()
        .name("invite-test-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Admin role setup
    fixtures::org_role(org.id, admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    // Create invitation for invitee (by gh_login)
    let invitation = fixtures::org_invitation(org.id, admin_user.id, "invitee")
        .member()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create invitation");

    // Invitee accepts using session principal
    let session_principal = PrincipalIdentity::UserSession {
        user: invitee.clone(),
    };

    respond_to_invitation(&ctx.conn, &session_principal, invitation.id, true)
        .await
        .expect("Failed to accept invitation");

    // Verify invitation was accepted
    let updated = OrgInvitationEntity::find_by_id(invitation.id)
        .one(&ctx.conn)
        .await
        .expect("DB error")
        .expect("Invitation not found");

    assert!(updated.accepted_at.is_some());
    assert!(updated.revoked_at.is_none());

    // Verify org_role was created
    let role = OrgRoleEntity::find()
        .filter(OrgRoleColumn::OrgId.eq(org.id))
        .filter(OrgRoleColumn::UserId.eq(invitee.id))
        .one(&ctx.conn)
        .await
        .expect("DB error")
        .expect("Role not found");

    assert_eq!(role.role, OrgRoleType::Member);
    assert!(role.revoked_at.is_none());
}

#[tokio::test]
async fn decline_invitation_success() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("invite-admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin user");

    let invitee = fixtures::user()
        .gh_login("decliner")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create invitee");

    let org = fixtures::org()
        .name("decline-test-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    let invitation = fixtures::org_invitation(org.id, admin_user.id, "decliner")
        .member()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create invitation");

    let session_principal = PrincipalIdentity::UserSession {
        user: invitee.clone(),
    };

    respond_to_invitation(&ctx.conn, &session_principal, invitation.id, false)
        .await
        .expect("Failed to decline invitation");

    // Verify invitation was revoked
    let updated = OrgInvitationEntity::find_by_id(invitation.id)
        .one(&ctx.conn)
        .await
        .expect("DB error")
        .expect("Invitation not found");

    assert!(updated.revoked_at.is_some());
    assert!(updated.accepted_at.is_none());

    // Verify NO org_role was created
    let role = OrgRoleEntity::find()
        .filter(OrgRoleColumn::OrgId.eq(org.id))
        .filter(OrgRoleColumn::UserId.eq(invitee.id))
        .one(&ctx.conn)
        .await
        .expect("DB error");

    assert!(role.is_none());
}

#[tokio::test]
async fn respond_invitation_requires_session() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("api-admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin");

    let invitee = fixtures::user()
        .gh_login("api-key-user")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .name("api-key-test-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    let invitation = fixtures::org_invitation(org.id, admin_user.id, "api-key-user")
        .member()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create invitation");

    // Try with API key principal (should fail)
    let api_key_principal =
        create_api_key_principal(&ctx, &invitee, vec![Permission::GrantOrgRole]).await;

    let result = respond_to_invitation(&ctx.conn, &api_key_principal, invitation.id, true).await;

    assert!(matches!(result, Err(Error::Validation(_))));
}

#[tokio::test]
async fn respond_invitation_not_found() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let session_principal = PrincipalIdentity::UserSession { user: user.clone() };

    let result = respond_to_invitation(&ctx.conn, &session_principal, 99999, true).await;

    assert!(matches!(result, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn respond_invitation_already_accepted() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("double-admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin");

    let invitee = fixtures::user()
        .gh_login("double-accepter")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .name("double-accept-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    let invitation = fixtures::org_invitation(org.id, admin_user.id, "double-accepter")
        .member()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create invitation");

    let session_principal = PrincipalIdentity::UserSession {
        user: invitee.clone(),
    };

    // Accept first time
    respond_to_invitation(&ctx.conn, &session_principal, invitation.id, true)
        .await
        .expect("Failed to accept invitation");

    // Try to accept again
    let result = respond_to_invitation(&ctx.conn, &session_principal, invitation.id, true).await;

    assert!(matches!(result, Err(Error::Validation(_))));
}

#[tokio::test]
async fn respond_invitation_already_revoked() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("revoke-admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin");

    let invitee = fixtures::user()
        .gh_login("double-decliner")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .name("double-decline-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    let invitation = fixtures::org_invitation(org.id, admin_user.id, "double-decliner")
        .member()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create invitation");

    let session_principal = PrincipalIdentity::UserSession {
        user: invitee.clone(),
    };

    // Decline first time
    respond_to_invitation(&ctx.conn, &session_principal, invitation.id, false)
        .await
        .expect("Failed to decline invitation");

    // Try to respond again (either accept or decline)
    let result = respond_to_invitation(&ctx.conn, &session_principal, invitation.id, true).await;

    assert!(matches!(result, Err(Error::Validation(_))));
}
