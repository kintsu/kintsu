//! Tagging errors (KTG) - ERR-0009
//! Errors related to variant tagging in oneof and error types.

define_domain_errors! {
    /// Tagging errors (KTG domain)
    pub enum TaggingError {
        /// KTG2001: Tag parameter must be string
        TagParameterInvalidType {
            code: (TG, Validation, 1),
            message: "attribute 'tag' parameter '{param}' must be a string literal",
            help: "use a string: #[tag(name = \"type\")]",
            fields: { param: String },
        },

        /// KTG2002: Tag on non-variant type
        TagOnNonVariantType {
            code: (TG, Validation, 2),
            message: "attribute 'tag' can only be applied to oneof or error types",
            help: "tagging attributes are only valid on oneof and error types",
        },

        /// KTG2003: Internal tag requires struct variants
        InternalTagRequiresStruct {
            code: (TG, Validation, 3),
            message: "internal tagging requires all variants to be struct types",
            help: "use external or adjacent tagging for non-struct variants",
        },

        /// KTG3001: Multiple tag styles
        MultipleTagStyles {
            code: (TG, Conflict, 1),
            message: "attribute 'tag' specifies multiple tagging styles",
            help: "choose one style: external, internal, adjacent, or untagged",
        },

        /// KTG3002: Internal tag field conflict
        InternalTagFieldConflict {
            code: (TG, Conflict, 2),
            message: "internal tag field '{name}' conflicts with variant field at index {index}",
            help: "rename the tag field or the variant field",
            fields: { name: String, index: usize },
        },

        /// KTG3003: Adjacent field name conflict
        AdjacentFieldNameConflict {
            code: (TG, Conflict, 3),
            message: "adjacent tag fields '{tag_field}' and '{content_field}' must be distinct",
            help: "use different names for tag and content fields",
            fields: { tag_field: String, content_field: String },
        },

        /// KTG3004: Untagged duplicate type
        UntaggedDuplicateType {
            code: (TG, Conflict, 4),
            message: "untagged union has duplicate type '{type_name}' at indices {indices}",
            help: "untagged unions require all variants to have distinct types",
            fields: { type_name: String, indices: String },
        },

        /// KTG3005: Untagged indistinguishable variants
        UntaggedIndistinguishable {
            code: (TG, Conflict, 5),
            message: "untagged variants at indices {indices} cannot be distinguished",
            help: "use tagged serialization or restructure variants",
            fields: { indices: String },
        },
    }
}

impl TaggingError {
    pub fn invalid_param_type(param: impl Into<String>) -> Self {
        Self::TagParameterInvalidType {
            param: param.into(),
            span: None,
        }
    }

    pub fn non_variant_type() -> Self {
        Self::TagOnNonVariantType { span: None }
    }

    pub fn internal_requires_struct() -> Self {
        Self::InternalTagRequiresStruct { span: None }
    }

    pub fn multiple_styles() -> Self {
        Self::MultipleTagStyles { span: None }
    }

    pub fn internal_field_conflict(
        name: impl Into<String>,
        index: usize,
    ) -> Self {
        Self::InternalTagFieldConflict {
            name: name.into(),
            index,
            span: None,
        }
    }

    pub fn adjacent_field_conflict(
        tag_field: impl Into<String>,
        content_field: impl Into<String>,
    ) -> Self {
        Self::AdjacentFieldNameConflict {
            tag_field: tag_field.into(),
            content_field: content_field.into(),
            span: None,
        }
    }

    pub fn untagged_duplicate(
        type_name: impl Into<String>,
        indices: impl IntoIterator<Item = usize>,
    ) -> Self {
        let indices = indices
            .into_iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Self::UntaggedDuplicateType {
            type_name: type_name.into(),
            indices,
            span: None,
        }
    }

    pub fn untagged_indistinguishable(indices: impl IntoIterator<Item = usize>) -> Self {
        let indices = indices
            .into_iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Self::UntaggedIndistinguishable {
            indices,
            span: None,
        }
    }
}
