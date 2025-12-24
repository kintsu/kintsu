use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{comments::DeclComment, context::DeclNamedItemContext, definitions::TypeDefinition};

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclNamespace {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DeclNamedItemContext>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<TypeDefinition>,

    #[cfg_attr(feature = "api", schema(no_recursion))]
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub namespaces: BTreeMap<String, Box<DeclNamespace>>,

    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}

impl DeclNamespace {
    pub fn collect_external_refs(
        &self,
        root_package: &str,
        refs: &mut std::collections::HashSet<DeclNamedItemContext>,
    ) {
        if let Some(error) = &self.error
            && error.is_external(root_package)
        {
            refs.insert(error.clone());
        }

        for type_def in &self.types {
            type_def.collect_external_refs(root_package, refs);
        }

        for ns in self.namespaces.values() {
            ns.collect_external_refs(root_package, refs);
        }
    }
}
