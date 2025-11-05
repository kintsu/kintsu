use std::ops::Deref;

pub struct ApiKey {
    db: kintsu_registry_db::models::api_key::ApiKey,
}

impl AsRef<kintsu_registry_db::models::api_key::ApiKey> for ApiKey {
    fn as_ref(&self) -> &kintsu_registry_db::models::api_key::ApiKey {
        &self.db
    }
}

impl Deref for ApiKey {
    type Target = kintsu_registry_db::models::api_key::ApiKey;

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
        let pool = req.app_data::<crate::DbPool>().cloned();

        let auth_header = req
            .headers()
            .get(actix_web::http::header::AUTHORIZATION)
            .and_then(|header_value| header_value.to_str().ok())
            .map(|s| s.to_string());

        Box::pin(async move {
            let pool = pool.ok_or_else(|| crate::Error::missing_data("DbPool"))?;

            let auth_header = auth_header.ok_or_else(|| crate::Error::AuthorizationRequired)?;

            if !auth_header.starts_with("Bearer ") {
                return Err(crate::Error::session("invalid authorization header format"));
            }

            let raw_token = auth_header
                .trim_start_matches("Bearer ")
                .into();

            let mut conn = pool.get().await?;

            Ok(Self {
                db: kintsu_registry_db::models::api_key::ApiKey::by_raw_token(
                    &mut conn, &raw_token,
                )
                .await?,
            })
        })
    }
}
