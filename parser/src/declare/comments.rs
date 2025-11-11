//! Comment preservation for type declarations

use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclComment {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
}

impl DeclComment {
    pub fn new() -> Self {
        Self {
            comments: Vec::new(),
        }
    }

    pub fn from_vec(comments: Vec<String>) -> Self {
        Self { comments }
    }

    pub fn is_empty(&self) -> bool {
        self.comments.is_empty()
    }

    pub fn push(
        &mut self,
        comment: String,
    ) {
        self.comments.push(comment);
    }

    pub fn merge(
        &mut self,
        other: DeclComment,
    ) {
        self.comments.extend(other.comments);
    }
}
