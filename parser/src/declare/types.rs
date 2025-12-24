use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::context::DeclNamedItemContext;

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Builtin {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    Usize,
    F16,
    F32,
    F64,
    Bool,
    Str,
    DateTime,
    Complex,
    Binary,
    Base64,
    Never,
}

impl Builtin {
    pub fn from_ast_builtin(builtin: &crate::ast::ty::Builtin) -> Self {
        use crate::ast::ty::Builtin as AstBuiltin;
        match builtin {
            AstBuiltin::I8(_) => Self::I8,
            AstBuiltin::I16(_) => Self::I16,
            AstBuiltin::I32(_) => Self::I32,
            AstBuiltin::I64(_) => Self::I64,
            AstBuiltin::U8(_) => Self::U8,
            AstBuiltin::U16(_) => Self::U16,
            AstBuiltin::U32(_) => Self::U32,
            AstBuiltin::U64(_) => Self::U64,
            AstBuiltin::Usize(_) => Self::Usize,
            AstBuiltin::F16(_) => Self::F16,
            AstBuiltin::F32(_) => Self::F32,
            AstBuiltin::F64(_) => Self::F64,
            AstBuiltin::Bool(_) => Self::Bool,
            AstBuiltin::Str(_) => Self::Str,
            AstBuiltin::DateTime(_) => Self::DateTime,
            AstBuiltin::Complex(_) => Self::Complex,
            AstBuiltin::Binary(_) => Self::Binary,
            AstBuiltin::Base64(_) => Self::Base64,
            AstBuiltin::Never(_) => Self::Never,
        }
    }
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeclType {
    Builtin {
        ty: Builtin,
    },

    Named {
        reference: DeclNamedItemContext,
    },

    Array {
        #[cfg_attr(feature = "api", schema(no_recursion))]
        element_type: Box<DeclType>,
    },

    SizedArray {
        #[cfg_attr(feature = "api", schema(no_recursion))]
        element_type: Box<DeclType>,
        size: u64,
    },

    Result {
        #[cfg_attr(feature = "api", schema(no_recursion))]
        ok_type: Box<DeclType>,
        error: DeclNamedItemContext,
    },

    Optional {
        #[cfg_attr(feature = "api", schema(no_recursion))]
        inner_type: Box<DeclType>,
    },

    Map {
        #[cfg_attr(feature = "api", schema(no_recursion))]
        key_type: Box<DeclType>,
        #[cfg_attr(feature = "api", schema(no_recursion))]
        value_type: Box<DeclType>,
    },

    Paren {
        #[cfg_attr(feature = "api", schema(no_recursion))]
        inner_type: Box<DeclType>,
    },
}

impl DeclType {
    pub fn collect_external_refs(
        &self,
        root_package: &str,
        refs: &mut HashSet<DeclNamedItemContext>,
    ) {
        match self {
            Self::Named { reference } => {
                if reference.is_external(root_package) {
                    refs.insert(reference.clone());
                }
            },
            Self::Array { element_type } => {
                element_type.collect_external_refs(root_package, refs);
            },
            Self::SizedArray { element_type, .. } => {
                element_type.collect_external_refs(root_package, refs);
            },
            Self::Result { ok_type, error } => {
                ok_type.collect_external_refs(root_package, refs);
                if error.is_external(root_package) {
                    refs.insert(error.clone());
                }
            },
            Self::Optional { inner_type } => {
                inner_type.collect_external_refs(root_package, refs);
            },
            Self::Map {
                key_type,
                value_type,
            } => {
                key_type.collect_external_refs(root_package, refs);
                value_type.collect_external_refs(root_package, refs);
            },
            Self::Paren { inner_type } => {
                inner_type.collect_external_refs(root_package, refs);
            },
            Self::Builtin { .. } => {},
        }
    }
}
