use actix_web::{FromRequest, cookie::*};
use secrecy::{ExposeSecret, SecretString};
use utoipa::ToSchema;

const COOKIE_NAME: &str = "kintsu_session";
const COOKIE_SESSION_DAYS: i64 = 7;

#[derive(serde::Deserialize, serde::Serialize, Debug, ToSchema)]
pub struct PublicData {
    #[serde(flatten)]
    pub user: kintsu_registry_db::entities::User,
    pub authenticated_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct SessionData {
    pub token: String,

    pub user: PublicData,

    #[serde(skip, default)]
    pub dirty: bool,
}

impl SessionData {
    pub fn new(
        user: kintsu_registry_db::entities::User,
        token: SecretString,
    ) -> Self {
        Self {
            dirty: false,
            token: token.expose_secret().to_string(),
            user: PublicData {
                user,
                authenticated_at: chrono::Utc::now(),
                expires_at: chrono::Utc::now() + chrono::Duration::days(COOKIE_SESSION_DAYS),
            },
        }
    }

    pub fn jar(
        &mut self,
        jar: &mut CookieJar,
        key: &Key,
        domain: String,
    ) -> crate::Result<()> {
        self.dirty = false;

        let mut user = jar.private_mut(&key);

        let mut cookie = Cookie::new(COOKIE_NAME, serde_json::to_string(self)?);

        cookie.set_path("/");
        cookie.set_secure(true);
        cookie.set_http_only(false);
        cookie.set_same_site(SameSite::Strict);
        cookie.set_max_age(actix_web::cookie::time::Duration::days(COOKIE_SESSION_DAYS));
        cookie.set_domain(domain);

        user.add(cookie);

        Ok(())
    }

    pub fn from_cookie(
        cookie: Cookie<'static>,
        key: &Key,
    ) -> crate::Result<Self> {
        let mut jar = CookieJar::new();
        jar.add_original(cookie.clone());

        let cookie = jar
            .private(&key)
            .get(COOKIE_NAME)
            .ok_or_else(|| crate::Error::session("missing session cookie"))?;

        let session: SessionData = serde_json::from_str(cookie.value())?;

        Ok(session)
    }
}

impl FromRequest for SessionData {
    type Error = crate::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let key = req
            .app_data::<actix_web::web::Data<Key>>()
            .cloned();

        let cookies = req
            .cookies()
            .map_err(crate::Error::from)
            .map(|cookies| {
                cookies
                    .iter()
                    .find(|&cookie| cookie.name() == COOKIE_NAME)
                    .ok_or_else(|| crate::Error::session("missing session cookie"))
                    .cloned()
            });

        Box::pin(async move {
            let key = key.ok_or_else(|| crate::Error::missing_data("cookie::Key"))?;

            let cookie = cookies??;

            let session: SessionData = SessionData::from_cookie(cookie, key.get_ref())?;

            Ok(session)
        })
    }
}
