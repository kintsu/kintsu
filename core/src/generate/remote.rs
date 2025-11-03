use std::collections::HashMap;

use validator::Validate;

#[derive(serde::Deserialize, Debug, PartialEq, Validate)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct RemoteConfig {
    #[validate(url)]
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}
