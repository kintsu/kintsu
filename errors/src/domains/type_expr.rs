//! Type expression errors (KTE) - ERR-0010
//! Errors related to type expression operators (Pick, Omit, Partial, etc.)

define_domain_errors! {
    /// Type expression errors (KTE domain)
    pub enum TypeExprError {
        /// KTE0001: Missing open bracket
        MissingOpenBracket {
            code: (TE, Syntax, 1),
            message: "expected '[' after operator name",
            help: "use bracket syntax: Pick[Type, fields]",
        },

        /// KTE0002: Unclosed bracket
        UnclosedBracket {
            code: (TE, Syntax, 2),
            message: "expected ']' to close operator",
            help: "close the expression with ']'",
        },

        /// KTE0003: Invalid selector
        InvalidSelector {
            code: (TE, Syntax, 3),
            message: "expected identifier in selector list",
            help: "selectors must be valid identifiers",
        },

        /// KTE0004: Missing separator
        MissingSeparator {
            code: (TE, Syntax, 4),
            message: "expected ',' or '|' between selectors",
            help: "separate selectors with ',' or '|'",
        },

        /// KTE1001: Unknown field
        UnknownField {
            code: (TE, Resolution, 1),
            message: "unknown field '{field}' in type '{type_name}'",
            help: "check field name spelling",
            fields: { field: String, type_name: String },
        },

        /// KTE1002: Unknown variant
        UnknownVariant {
            code: (TE, Resolution, 2),
            message: "unknown variant '{variant}' in type '{type_name}'",
            help: "check variant name spelling",
            fields: { variant: String, type_name: String },
        },

        /// KTE2001: Expected struct type
        ExpectedStructType {
            code: (TE, Validation, 1),
            message: "expected struct type for {operator}, found {found}",
            help: "this operator only works on struct types",
            fields: { operator: String, found: String },
        },

        /// KTE2002: Expected oneof type
        ExpectedOneofType {
            code: (TE, Validation, 2),
            message: "expected oneof type for {operator}, found {found}",
            help: "this operator only works on oneof types",
            fields: { operator: String, found: String },
        },

        /// KTE2003: Expected array type
        ExpectedArrayType {
            code: (TE, Validation, 3),
            message: "expected array type for ArrayItem, found {found}",
            help: "ArrayItem extracts the element type from arrays",
            fields: { found: String },
        },

        /// KTE2004: Cannot access fields on type
        CannotAccessFieldsOnType {
            code: (TE, Validation, 4),
            message: "cannot access fields on {type_kind} type '{type_name}'",
            help: "field projection only works on struct types",
            fields: { type_kind: String, type_name: String },
        },

        /// KTE4001: Empty selector list
        EmptySelectorList {
            code: (TE, Missing, 1),
            message: "{operator} requires at least one selector",
            help: "specify fields or variants to include/exclude",
            fields: { operator: String },
        },

        /// KTE4002: No fields remain
        NoFieldsRemain {
            code: (TE, Missing, 2),
            message: "{operator} would remove all fields from '{type_name}'",
            help: "ensure at least one field remains",
            fields: { operator: String, type_name: String },
        },

        /// KTE4003: No variants remain
        NoVariantsRemain {
            code: (TE, Missing, 3),
            message: "{operator} would remove all variants from '{type_name}'",
            help: "ensure at least one variant remains",
            fields: { operator: String, type_name: String },
        },

        /// KTE5001: Cyclic type expression
        CyclicTypeExpression {
            code: (TE, Cycle, 1),
            message: "cyclic type expression: {chain}",
            help: "type expressions cannot reference themselves",
            fields: { chain: String },
        },

        /// KTE8001: Duplicate selector ignored
        DuplicateSelectorIgnored {
            code: (TE, Warning, 1),
            message: "duplicate selector '{name}' in {operator}",
            help: "remove the duplicate",
            severity: Warning,
            fields: { name: String, operator: String },
        },

        /// KTE8002: Redundant Partial
        RedundantPartial {
            code: (TE, Warning, 2),
            message: "Partial on '{type_name}' has no effect (all fields already optional)",
            help: "remove the redundant Partial operator",
            severity: Warning,
            fields: { type_name: String },
        },

        /// KTE8003: Redundant Required
        RedundantRequired {
            code: (TE, Warning, 3),
            message: "Required on '{type_name}' has no effect (no optional fields)",
            help: "remove the redundant Required operator",
            severity: Warning,
            fields: { type_name: String },
        },
    }
}

impl TypeExprError {
    pub fn missing_bracket() -> Self {
        Self::MissingOpenBracket { span: None }
    }

    pub fn unclosed_bracket() -> Self {
        Self::UnclosedBracket { span: None }
    }

    pub fn invalid_selector() -> Self {
        Self::InvalidSelector { span: None }
    }

    pub fn missing_separator() -> Self {
        Self::MissingSeparator { span: None }
    }

    pub fn unknown_field(
        field: impl Into<String>,
        type_name: impl Into<String>,
    ) -> Self {
        Self::UnknownField {
            field: field.into(),
            type_name: type_name.into(),
            span: None,
        }
    }

    pub fn unknown_variant(
        variant: impl Into<String>,
        type_name: impl Into<String>,
    ) -> Self {
        Self::UnknownVariant {
            variant: variant.into(),
            type_name: type_name.into(),
            span: None,
        }
    }

    pub fn expected_struct(
        operator: impl Into<String>,
        found: impl Into<String>,
    ) -> Self {
        Self::ExpectedStructType {
            operator: operator.into(),
            found: found.into(),
            span: None,
        }
    }

    pub fn expected_oneof(
        operator: impl Into<String>,
        found: impl Into<String>,
    ) -> Self {
        Self::ExpectedOneofType {
            operator: operator.into(),
            found: found.into(),
            span: None,
        }
    }

    pub fn expected_array(found: impl Into<String>) -> Self {
        Self::ExpectedArrayType {
            found: found.into(),
            span: None,
        }
    }

    pub fn empty_selector(operator: impl Into<String>) -> Self {
        Self::EmptySelectorList {
            operator: operator.into(),
            span: None,
        }
    }

    pub fn no_fields_remain(
        operator: impl Into<String>,
        type_name: impl Into<String>,
    ) -> Self {
        Self::NoFieldsRemain {
            operator: operator.into(),
            type_name: type_name.into(),
            span: None,
        }
    }

    pub fn cyclic(chain: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let chain = chain
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
            .join(" -> ");
        Self::CyclicTypeExpression { chain, span: None }
    }
}
