use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::defs::Spanned;

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    pub version: u32,
}

impl Meta {
    pub fn new(version: u32) -> Self {
        Self { version }
    }

    pub fn from_resolved_version(
        item_name: &str,
        resolved_versions: &BTreeMap<String, Spanned<u32>>,
    ) -> Option<Self> {
        resolved_versions
            .get(item_name)
            .map(|v| Self::new(v.value))
    }
}
