//! Type resolution errors (KTR) - ERR-0006
//! Errors during type reference resolution.

use std::collections::HashSet;

define_domain_errors! {
    /// Type resolution errors (KTR domain)
    pub enum ResolutionError {
        /// KTR1001: Resolution error (generic)
        ResolutionError {
            code: (TR, Resolution, 1),
            message: "resolution error. could not resolve '{path}'",
            help: "ensure the type is defined and accessible from current namespace",
            fields: { path: String },
        },

        /// KTR1002: Undefined type
        UndefinedType {
            code: (TR, Resolution, 2),
            message: "undefined type: '{name}'",
            help: "check spelling or define the type",
            fields: { name: String },
        },

        /// KTR1003: Unresolved type
        UnresolvedType {
            code: (TR, Resolution, 3),
            message: "unresolved type: '{name}'",
            help: "the type could not be resolved after all resolution passes",
            fields: { name: String },
        },

        /// KTR5001: Circular dependency
        CircularDependency {
            code: (TR, Cycle, 1),
            message: "circular dependency detected: {chain}",
            help: "restructure to break the circular import",
            fields: { chain: String },
        },

        /// KTR5002: Schema circular dependency
        SchemaCircularDependency {
            code: (TR, Cycle, 2),
            message: "circular schema dependency detected: {schemas}",
            help: "reorganize schema files to eliminate circular imports",
            fields: { schemas: String },
        },

        /// KTR5003: Circular alias
        CircularAlias {
            code: (TR, Cycle, 3),
            message: "circular alias detected: {chain}",
            help: "break the cycle by removing one alias",
            fields: { chain: String },
        },
    }
}

impl ResolutionError {
    pub fn resolution(path: impl Into<String>) -> Self {
        Self::ResolutionError {
            path: path.into(),
            span: None,
        }
    }

    pub fn undefined_type(name: impl Into<String>) -> Self {
        Self::UndefinedType {
            name: name.into(),
            span: None,
        }
    }

    pub fn unresolved_type(name: impl Into<String>) -> Self {
        Self::UnresolvedType {
            name: name.into(),
            span: None,
        }
    }

    pub fn circular_dependency(deps: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let chain = deps
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
            .join(" -> ");
        Self::CircularDependency { chain, span: None }
    }

    pub fn schema_circular(schemas: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let schemas = schemas
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
            .join(" -> ");
        Self::SchemaCircularDependency {
            schemas,
            span: None,
        }
    }

    pub fn circular_alias(chain: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let chain = chain
            .into_iter()
            .map(|s| s.as_ref().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        Self::CircularAlias { chain, span: None }
    }

    pub fn circular_alias_from_set(chain: HashSet<String>) -> Self {
        let chain = chain
            .into_iter()
            .collect::<Vec<_>>()
            .join(" -> ");
        Self::CircularAlias { chain, span: None }
    }

    /// Creates a type circular dependency error - same as circular_dependency.
    /// This exists for compatibility with the parser which has separate TypeCircularDependency.
    pub fn type_cycle(types: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let chain = types
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
            .join(" -> ");
        Self::CircularDependency { chain, span: None }
    }
}
