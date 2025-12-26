//! Namespace errors (KNS) - ERR-0004
//! Errors related to namespace declaration and organization.

define_domain_errors! {
    /// Namespace errors (KNS domain)
    pub enum NamespaceError {
        /// KNS1001: Namespace not declared
        NsNotDeclared {
            code: (NS, Resolution, 1),
            message: "namespace is not declared",
            help: "add 'namespace <name>;' at the top of the file",
        },

        /// KNS1002: Unresolved dependency
        UnresolvedDependency {
            code: (NS, Resolution, 2),
            message: "use statement '{name}' is not a local namespace and not in package dependencies",
            help: "add the dependency to your manifest or define the namespace locally",
            fields: { name: String },
        },

        /// KNS3001: Multiple namespace declarations
        NsConflict {
            code: (NS, Conflict, 1),
            message: "only one namespace may be declared in a payload declaration file",
            help: "remove the duplicate namespace declaration",
        },

        /// KNS3002: Namespace directory conflict
        NsDirConflict {
            code: (NS, Conflict, 2),
            message: "namespace {namespace} is already declared for {parent}, {attempted} cannot be declared",
            help: "each namespace must correspond to exactly one directory",
            fields: { namespace: String, parent: String, attempted: String },
        },

        /// KNS3003: Namespace mismatch
        NamespaceMismatch {
            code: (NS, Conflict, 3),
            message: "namespace mismatch: expected {expected}, found {found}",
            help: "ensure declared namespace matches the file's directory location",
            fields: { expected: String, found: String },
        },

        /// KNS3004: Duplicate namespace
        DuplicateNamespace {
            code: (NS, Conflict, 4),
            message: "namespace {name} is declared multiple times",
            help: "rename one of the conflicting namespaces",
            fields: { name: String },
        },

        /// KNS4001: Use path not found
        UsePathNotFound {
            code: (NS, Missing, 1),
            message: "use statement '{name}' does not correspond to a .ks file or directory",
            help: "check the path exists or define the namespace",
            fields: { name: String },
        },
    }
}

impl NamespaceError {
    pub fn not_declared() -> Self {
        Self::NsNotDeclared { span: None }
    }

    pub fn conflict() -> Self {
        Self::NsConflict { span: None }
    }

    pub fn dir_conflict(
        namespace: impl Into<String>,
        parent: impl Into<String>,
        attempted: impl Into<String>,
    ) -> Self {
        Self::NsDirConflict {
            namespace: namespace.into(),
            parent: parent.into(),
            attempted: attempted.into(),
            span: None,
        }
    }

    pub fn mismatch(
        expected: impl Into<String>,
        found: impl Into<String>,
    ) -> Self {
        Self::NamespaceMismatch {
            expected: expected.into(),
            found: found.into(),
            span: None,
        }
    }

    pub fn duplicate(name: impl Into<String>) -> Self {
        Self::DuplicateNamespace {
            name: name.into(),
            span: None,
        }
    }

    pub fn unresolved_dep(name: impl Into<String>) -> Self {
        Self::UnresolvedDependency {
            name: name.into(),
            span: None,
        }
    }

    pub fn use_not_found(name: impl Into<String>) -> Self {
        Self::UsePathNotFound {
            name: name.into(),
            span: None,
        }
    }
}
