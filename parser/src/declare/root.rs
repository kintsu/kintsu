use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{context::DeclNamedItemContext, namespace::DeclNamespace};

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeRegistryDeclaration {
    pub package: String,
    pub namespaces: Vec<DeclNamespace>,
    pub external_refs: BTreeSet<DeclNamedItemContext>,
}

impl TypeRegistryDeclaration {
    pub fn new(package: String) -> Self {
        Self {
            package,
            namespaces: Vec::new(),
            external_refs: BTreeSet::new(),
        }
    }

    pub fn extend_refs(
        &mut self,
        refs: std::collections::BTreeSet<DeclNamedItemContext>,
    ) {
        self.external_refs.extend(refs);
    }
}
