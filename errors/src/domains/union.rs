//! Union errors (KUN) - ERR-0007
//! Errors related to union type operations, field merging, and tagging validation.

define_domain_errors! {
    /// Union errors (KUN domain)
    pub enum UnionError {
        /// KUN2001: Union operand must be struct
        UnionOperandNotStruct {
            code: (UN, Validation, 1),
            message: "union operand must be struct type: found {found_type} '{operand_name}'",
            help: "union operations require struct types",
            fields: { found_type: String, operand_name: String },
        },

        /// KUN3001: Union field conflict
        UnionFieldConflict {
            code: (UN, Conflict, 1),
            message: "union field conflict: field '{field_name}' appears in multiple operands with different types",
            help: "leftmost field definition takes precedence; rename to preserve both",
            severity: Warning,
            fields: { field_name: String },
        },

        /// KUN8001: Union field shadowed
        UnionFieldShadowed {
            code: (UN, Warning, 1),
            message: "field '{field_name}' from '{operand_name}' is shadowed by earlier operand",
            help: "this field will not appear in merged result; consider renaming",
            severity: Warning,
            fields: { field_name: String, operand_name: String },
        },

        /// KUN2002: Adjacent tagging conflict - name and content fields must differ
        AdjacentTagConflict {
            code: (UN, Validation, 2),
            message: "adjacent tagging: name field '{name}' and content field '{content}' must be different",
            help: "use different field names for tag and content in adjacent tagging",
            fields: { name: String, content: String },
        },

        /// KUN2003: Internal tagging field conflict
        InternalTagFieldConflict {
            code: (UN, Validation, 3),
            message: "internal tagging: tag field '{tag_field}' conflicts with existing field in variant '{variant}'",
            help: "rename the tag field or the conflicting variant field",
            fields: { tag_field: String, variant: String },
        },
    }
}

impl UnionError {
    pub fn operand_not_struct(
        found_type: impl Into<String>,
        operand_name: impl Into<String>,
    ) -> Self {
        Self::UnionOperandNotStruct {
            found_type: found_type.into(),
            operand_name: operand_name.into(),
            span: None,
        }
    }

    /// Alias for `operand_not_struct` with parameters in reverse order.
    pub fn non_struct_operand(
        operand_name: impl Into<String>,
        found_type: impl Into<String>,
    ) -> Self {
        Self::operand_not_struct(found_type, operand_name)
    }

    pub fn field_conflict(field_name: impl Into<String>) -> Self {
        Self::UnionFieldConflict {
            field_name: field_name.into(),
            span: None,
        }
    }

    pub fn field_shadowed(
        field_name: impl Into<String>,
        operand_name: impl Into<String>,
    ) -> Self {
        Self::UnionFieldShadowed {
            field_name: field_name.into(),
            operand_name: operand_name.into(),
            span: None,
        }
    }

    pub fn adjacent_tag_conflict(
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self::AdjacentTagConflict {
            name: name.into(),
            content: content.into(),
            span: None,
        }
    }

    pub fn internal_tag_field_conflict(
        tag_field: impl Into<String>,
        variant: impl Into<String>,
    ) -> Self {
        Self::InternalTagFieldConflict {
            tag_field: tag_field.into(),
            variant: variant.into(),
            span: None,
        }
    }
}
