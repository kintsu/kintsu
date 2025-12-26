//! Type definition errors (KTY) - ERR-0005
//! Errors related to struct, enum, error, and oneof declarations.

define_domain_errors! {
    /// Type definition errors (KTY domain)
    pub enum TypeDefError {
        /// KTY2001: Missing error type for fallible operation
        MissingErrorType {
            code: (TY, Validation, 1),
            message: "operation '{operation}' returns a fallible type but has no error type defined",
            help: "add an error type to the operation or remove the '!' from the return type",
            fields: { operation: String },
        },

        /// KTY2002: Union operand must be struct
        UnionOperandMustBeStruct {
            code: (TY, Validation, 2),
            message: "union operand must be struct type: found {found_type} '{operand_name}'",
            help: "only struct types can be used in union operations",
            fields: { found_type: String, operand_name: String },
        },

        /// KTY3001: Identifier conflict in namespace
        IdentConflict {
            code: (TY, Conflict, 1),
            message: "{namespace} has conflicts. {tag} {ident} is declared multiple times",
            help: "rename one of the conflicting declarations",
            fields: { namespace: String, tag: String, ident: String },
        },

        /// KTY3002: Duplicate type registration
        DuplicateType {
            code: (TY, Conflict, 2),
            message: "duplicate type '{name}' already registered",
            help: "rename the type to avoid conflict",
            fields: { name: String },
        },

        /// KTY3003: Duplicate field in type
        DuplicateField {
            code: (TY, Conflict, 3),
            message: "duplicate field '{name}' in {type_kind} '{type_name}'",
            help: "rename one of the duplicate fields",
            fields: { name: String, type_kind: String, type_name: String },
        },

        /// KTY5001: Type circular dependency
        TypeCircularDependency {
            code: (TY, Cycle, 1),
            message: "circular type dependency detected: {path}",
            help: "break the cycle by restructuring type definitions",
            fields: { path: String },
        },

        // Type Expression Errors (KTE) - RFC-0018, SPEC-0017, TSY-0014

        /// KTE2001: Invalid target type for operator
        TypeExprTargetMismatch {
            code: (TY, Validation, 10),
            message: "{operator}: expected {expected} type, found {actual}",
            help: "ensure the target type matches operator requirements",
            fields: { operator: String, expected: String, actual: String },
        },

        /// KTE1001: Unknown field in selector
        TypeExprFieldNotFound {
            code: (TY, Resolution, 10),
            message: "{operator}: field '{field}' not found in type {type_name}",
            help: "check that the field name exists in the target type",
            fields: { operator: String, field: String, type_name: String },
        },

        /// KTE1002: Unknown variant in selector
        TypeExprVariantNotFound {
            code: (TY, Resolution, 11),
            message: "{operator}: variant '{variant}' not found in type {type_name}",
            help: "check that the variant name exists in the target oneof",
            fields: { operator: String, variant: String, type_name: String },
        },

        /// KTE4001: Empty selector list
        TypeExprEmptySelectors {
            code: (TY, Validation, 11),
            message: "{operator}: selector list cannot be empty",
            help: "provide at least one field or variant selector",
            fields: { operator: String },
        },

        /// KTE4002: No fields remain after operation
        TypeExprNoFieldsRemain {
            code: (TY, Validation, 12),
            message: "{operator}: no fields remain after operation",
            help: "ensure at least one field remains after omission",
            fields: { operator: String },
        },

        /// KTE4003: No variants remain after operation
        TypeExprNoVariantsRemain {
            code: (TY, Validation, 13),
            message: "{operator}: no variants remain after operation",
            help: "ensure at least one variant remains after exclusion",
            fields: { operator: String },
        },

        /// KTE8001: Cyclic type expression reference
        TypeExprCycle {
            code: (TY, Cycle, 2),
            message: "type expression cycle detected: {path}",
            help: "break the cycle by restructuring type expressions",
            fields: { path: String },
        },

        /// KTE2002: Unresolved type in type expression
        TypeExprUnresolved {
            code: (TY, Resolution, 12),
            message: "unresolved type in type expression: '{name}'",
            help: "ensure the type is defined before use",
            fields: { name: String },
        },
    }
}

impl TypeDefError {
    pub fn missing_error_type(operation: impl Into<String>) -> Self {
        Self::MissingErrorType {
            operation: operation.into(),
            span: None,
        }
    }

    pub fn union_operand_not_struct(
        found_type: impl Into<String>,
        operand_name: impl Into<String>,
    ) -> Self {
        Self::UnionOperandMustBeStruct {
            found_type: found_type.into(),
            operand_name: operand_name.into(),
            span: None,
        }
    }

    pub fn ident_conflict(
        namespace: impl Into<String>,
        tag: impl Into<String>,
        ident: impl Into<String>,
    ) -> Self {
        Self::IdentConflict {
            namespace: namespace.into(),
            tag: tag.into(),
            ident: ident.into(),
            span: None,
        }
    }

    pub fn duplicate_type(name: impl Into<String>) -> Self {
        Self::DuplicateType {
            name: name.into(),
            span: None,
        }
    }

    pub fn duplicate_field(
        name: impl Into<String>,
        type_kind: impl Into<String>,
        type_name: impl Into<String>,
    ) -> Self {
        Self::DuplicateField {
            name: name.into(),
            type_kind: type_kind.into(),
            type_name: type_name.into(),
            span: None,
        }
    }

    pub fn circular_dependency(types: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let path = types
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
            .join(" -> ");
        Self::TypeCircularDependency { path, span: None }
    }

    // Type Expression Error constructors (KTE) - RFC-0018, SPEC-0017, TSY-0014

    pub fn type_expr_target_mismatch(
        operator: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::TypeExprTargetMismatch {
            operator: operator.into(),
            expected: expected.into(),
            actual: actual.into(),
            span: None,
        }
    }

    pub fn type_expr_field_not_found(
        operator: impl Into<String>,
        field: impl Into<String>,
        type_name: impl Into<String>,
    ) -> Self {
        Self::TypeExprFieldNotFound {
            operator: operator.into(),
            field: field.into(),
            type_name: type_name.into(),
            span: None,
        }
    }

    pub fn type_expr_variant_not_found(
        operator: impl Into<String>,
        variant: impl Into<String>,
        type_name: impl Into<String>,
    ) -> Self {
        Self::TypeExprVariantNotFound {
            operator: operator.into(),
            variant: variant.into(),
            type_name: type_name.into(),
            span: None,
        }
    }

    pub fn type_expr_empty_selectors(operator: impl Into<String>) -> Self {
        Self::TypeExprEmptySelectors {
            operator: operator.into(),
            span: None,
        }
    }

    pub fn type_expr_no_fields_remain(operator: impl Into<String>) -> Self {
        Self::TypeExprNoFieldsRemain {
            operator: operator.into(),
            span: None,
        }
    }

    pub fn type_expr_no_variants_remain(operator: impl Into<String>) -> Self {
        Self::TypeExprNoVariantsRemain {
            operator: operator.into(),
            span: None,
        }
    }

    pub fn type_expr_cycle(types: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let path = types
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
            .join(" -> ");
        Self::TypeExprCycle { path, span: None }
    }

    pub fn type_expr_unresolved(name: impl Into<String>) -> Self {
        Self::TypeExprUnresolved {
            name: name.into(),
            span: None,
        }
    }
}
