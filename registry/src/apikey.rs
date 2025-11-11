use std::ops::Deref;

pub struct ApiKey {
    pub(crate) db: kintsu_registry_db::entities::ApiKey,
}

impl AsRef<kintsu_registry_db::entities::ApiKey> for ApiKey {
    fn as_ref(&self) -> &kintsu_registry_db::entities::ApiKey {
        &self.db
    }
}

impl Deref for ApiKey {
    type Target = kintsu_registry_db::entities::ApiKey;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl actix_web::FromRequest for ApiKey {
    type Error = crate::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<Self, Self::Error>>>,
    >;

    fn from_request(
        req: &actix_web::HttpRequest,
        _: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let pool = req.app_data::<crate::DbConn>().cloned();

        let auth_header = req
            .headers()
            .get(actix_web::http::header::AUTHORIZATION)
            .and_then(|header_value| header_value.to_str().ok())
            .map(|s| s.to_string());

        Box::pin(async move {
            let conn = pool.ok_or_else(|| crate::Error::missing_data("DbConn"))?;

            let auth_header = auth_header.ok_or_else(|| crate::Error::AuthorizationRequired)?;

            if auth_header.len() > kintsu_registry_db::MAX_TOKEN_HEADER_LENGTH {
                return Err(crate::Error::session("authorization header too long"));
            }

            if !auth_header.starts_with("Bearer ") {
                return Err(crate::Error::session("invalid authorization header format"));
            }

            let raw_token = auth_header
                .trim_start_matches("Bearer ")
                .into();

            Ok(Self {
                db: kintsu_registry_db::entities::ApiKey::by_raw_token(conn.as_ref(), &raw_token)
                    .await?,
            })
        })
    }
}
