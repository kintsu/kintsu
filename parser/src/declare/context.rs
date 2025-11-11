use serde::{Deserialize, Serialize};

use crate::ctx::{NamedItemContext, RefContext};

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeclRefContext {
    pub package: String,
    pub namespace: Vec<String>,
}

impl DeclRefContext {
    pub fn from_ref_context(ctx: &RefContext) -> Self {
        Self {
            package: ctx.package.clone(),
            namespace: ctx.namespace.clone(),
        }
    }
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeclNamedItemContext {
    pub context: DeclRefContext,
    pub name: String,
}

impl DeclNamedItemContext {
    pub(crate) fn from_named_item_context(ctx: &NamedItemContext) -> Self {
        Self {
            context: DeclRefContext::from_ref_context(&ctx.context),
            name: ctx.name.borrow_string().clone(),
        }
    }

    pub fn qualified_path(&self) -> String {
        let mut parts = vec![self.context.package.clone()];
        parts.extend(self.context.namespace.iter().cloned());
        parts.push(self.name.clone());
        parts.join("::")
    }

    pub(crate) fn is_external(
        &self,
        root_package: &str,
    ) -> bool {
        self.context.package != root_package
    }
}
