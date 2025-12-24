use actix_web::FromRequest;
use kintsu_registry_db::engine::{Entity, PrincipalIdentity};

pub struct Principal {
    pub id: PrincipalIdentity,
}

impl AsRef<PrincipalIdentity> for Principal {
    fn as_ref(&self) -> &PrincipalIdentity {
        &self.id
    }
}

impl std::ops::Deref for Principal {
    type Target = PrincipalIdentity;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl FromRequest for Principal {
    type Error = crate::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<Self, Self::Error>>>,
    >;

    fn from_request(
        req: &actix_web::HttpRequest,
        _: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let api_key = super::apikey::ApiKey::extract(req);
        let session = super::session::SessionData::extract(req);
        let db = req.app_data::<crate::DbConn>().cloned();

        Box::pin(async move {
            let db = db.ok_or_else(|| crate::Error::missing_data("DbPool"))?;

            Ok(Self {
                id: match tokio::join!(api_key, session) {
                    (Ok(key), _) => {
                        let key = key.into_inner();
                        let owner = key.get_token_owner(db.as_ref()).await?;
                        match owner {
                            Entity::User(user) => PrincipalIdentity::UserApiKey { user, key },
                            Entity::Org(org) => PrincipalIdentity::OrgApiKey { org, key },
                        }
                    },
                    (_, Ok(session)) => {
                        PrincipalIdentity::UserSession {
                            user: session.user.user,
                        }
                    },
                    (
                        Err(crate::Error::AuthorizationRequired),
                        Err(crate::Error::AuthorizationRequired),
                    ) => {
                        return Err(crate::Error::AuthorizationRequired);
                    },
                    (Err(e), Err(crate::Error::AuthorizationRequired)) => return Err(e),
                    (Err(crate::Error::AuthorizationRequired), Err(e)) => return Err(e),
                    (Err(key_err), Err(sess_err)) => {
                        return Err(crate::Error::Multiple(vec![key_err, sess_err]));
                    },
                },
            })
        })
    }
}
