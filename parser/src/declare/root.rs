use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{context::DeclNamedItemContext, namespace::DeclNamespace};

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeRegistryDeclaration {
    pub package: String,
    /// Map of namespace name to namespace declarations
    pub namespaces: BTreeMap<String, DeclNamespace>,
    pub external_refs: BTreeSet<DeclNamedItemContext>,
}

impl TypeRegistryDeclaration {
    pub fn new(package: String) -> Self {
        Self {
            package,
            namespaces: BTreeMap::new(),
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

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclarationBundle {
    /// Root package declarations with its external_refs
    pub root: TypeRegistryDeclaration,

    /// Dependency package declarations
    /// Key: snake_case package name (e.g., "pkg_10")
    /// Value: declarations for that dependency
    pub dependencies: BTreeMap<String, TypeRegistryDeclaration>,
}
