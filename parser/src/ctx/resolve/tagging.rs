//! Tagging Validation Phase (Phase 4.5)
//!
//! Validates variant tagging attributes and resolves inheritance.
//! This phase runs after `validate_unions` and before `merge_unions`.
//!
//! **Spec references:** RFC-0017, SPEC-0016, TSY-0013

use std::collections::HashMap;

use crate::{
    ast::{
        err::ErrorType,
        items::CommentOrMeta,
        meta::{ItemMeta, ItemMetaItem, TagAttribute, TagStyle},
        one_of::OneOf,
        strct::Sep,
        ty::Type,
        variadic::Variant,
    },
    ctx::{NamespaceChild, resolve::TypeResolver},
    defs::Spanned,
    tokens::ast::RepeatedItem,
};

impl TypeResolver {
    /// Validate tagging attributes (Phase 4.5)
    ///
    /// Validates `#[tag(...)]` and `#[rename(...)]` attributes and resolves
    /// namespace-to-type inheritance per SPEC-0016 Phase 4.
    ///
    /// **Spec references:** RFC-0017, SPEC-0016 Phases 2-5
    pub(super) async fn validate_tagging(&mut self) -> crate::Result<()> {
        tracing::debug!("validate_tagging: starting phase 4.5");

        let ns = self.namespace.lock().await;

        // Get namespace-level tag for inheritance
        let ns_tag = ns.tag.as_ref().map(|t| &t.value);

        // Collect tagging errors
        for (_name, child) in &ns.children {
            match &child.value {
                NamespaceChild::OneOf(oneof_def) => {
                    // Resolve tag attribute with namespace inheritance
                    let tag_attr = Self::resolve_tag_attribute(&oneof_def.meta, ns_tag);
                    Self::validate_oneof_tag_constraints(&tag_attr, &oneof_def.def.value)?;
                },
                NamespaceChild::Type(type_def) => {
                    // Check if it's a union type
                    if let Type::Union { .. } = &type_def.def.value.ty.value {
                        // Resolve tag attribute with namespace inheritance
                        let tag_attr = Self::resolve_tag_attribute(&type_def.meta, ns_tag);
                        Self::validate_union_tag_constraints(&tag_attr)?;
                    }
                },
                NamespaceChild::Error(error_def) => {
                    // Error types also support tagging
                    let tag_attr = Self::resolve_tag_attribute(&error_def.meta, ns_tag);
                    Self::validate_error_tag_constraints(&tag_attr, &error_def.def.value)?;
                },
                _ => {},
            }
        }

        tracing::debug!("validate_tagging: phase 4.5 complete");
        Ok(())
    }

    /// Resolve tag attribute with namespace inheritance per SPEC-0016 Phase 4.
    ///
    /// Resolution order:
    /// 1. Type-level `#[tag(...)]` attribute wins
    /// 2. Namespace-level `#![tag(...)]` default
    /// 3. Global default: `TagAttribute::default()` (TypeHint with type_hint=true)
    fn resolve_tag_attribute(
        meta: &[Spanned<CommentOrMeta>],
        ns_tag: Option<&TagAttribute>,
    ) -> TagAttribute {
        // 1. Type-level tag wins
        if let Some(tag) = Self::extract_tag_attribute_from_meta(meta) {
            return tag;
        }

        // 2. Namespace-level tag
        if let Some(tag) = ns_tag {
            return tag.clone();
        }

        // 3. Global default
        TagAttribute::default()
    }

    /// Extract TagAttribute from CommentOrMeta vector (type-level only, no inheritance)
    fn extract_tag_attribute_from_meta(meta: &[Spanned<CommentOrMeta>]) -> Option<TagAttribute> {
        for item in meta {
            if let CommentOrMeta::Meta(item_meta) = &item.value {
                if let Some(tag) = Self::extract_tag_from_item_meta(&item_meta.value) {
                    return Some(tag);
                }
            }
        }
        None
    }

    /// Extract TagAttribute from ItemMeta
    fn extract_tag_from_item_meta(meta: &ItemMeta) -> Option<TagAttribute> {
        for item in &meta.meta {
            if let ItemMetaItem::Tag(tag_meta) = item {
                return Some(tag_meta.value.clone());
            }
        }
        None
    }

    /// Validate tag constraints for oneof definitions
    fn validate_oneof_tag_constraints(
        tag_attr: &TagAttribute,
        oneof_def: &OneOf,
    ) -> crate::Result<()> {
        match &tag_attr.style {
            TagStyle::Adjacent { name, content } => {
                // Adjacent: name != content
                if name == content {
                    return Err(crate::Error::AdjacentTagConflict {
                        name: name.clone(),
                        content: content.clone(),
                    });
                }
            },
            TagStyle::Internal { name } => {
                // Internal: tag field must not conflict with variant fields
                // Also: tuple variants must reference struct types (TSY-0013)
                for variant in &oneof_def.variants.values {
                    Self::check_internal_tag_field_conflict(name, &variant.value)?;
                    Self::check_internal_tuple_variant_constraint(&variant.value)?;
                }
            },
            TagStyle::Untagged => {
                // Untagged: variants must be distinguishable (TSY-0013)
                Self::validate_untagged_distinguishability(&oneof_def.variants.values)?;
            },
            // TypeHint, External, Index - no structural conflicts
            TagStyle::TypeHint | TagStyle::External | TagStyle::Index { .. } => {},
        }
        Ok(())
    }

    /// Validate tag constraints for union types (type X = oneof A | B)
    fn validate_union_tag_constraints(tag_attr: &TagAttribute) -> crate::Result<()> {
        match &tag_attr.style {
            TagStyle::Adjacent { name, content } => {
                if name == content {
                    return Err(crate::Error::AdjacentTagConflict {
                        name: name.clone(),
                        content: content.clone(),
                    });
                }
            },
            // For union types, internal tagging validation requires resolving type refs
            // which happens in earlier phases. Here we just check structural constraints.
            TagStyle::Internal { .. } => {
                // todo: cross-check with resolved types if available
            },
            TagStyle::TypeHint
            | TagStyle::External
            | TagStyle::Untagged
            | TagStyle::Index { .. } => {},
        }
        Ok(())
    }

    /// Validate tag constraints for error definitions
    fn validate_error_tag_constraints(
        tag_attr: &TagAttribute,
        error_def: &ErrorType,
    ) -> crate::Result<()> {
        match &tag_attr.style {
            TagStyle::Adjacent { name, content } => {
                if name == content {
                    return Err(crate::Error::AdjacentTagConflict {
                        name: name.clone(),
                        content: content.clone(),
                    });
                }
            },
            TagStyle::Internal { name } => {
                for variant in &error_def.variants.values {
                    Self::check_internal_tag_field_conflict(name, &variant.value)?;
                }
            },
            TagStyle::TypeHint
            | TagStyle::External
            | TagStyle::Untagged
            | TagStyle::Index { .. } => {},
        }
        Ok(())
    }

    /// Check if internal tag field conflicts with variant fields
    fn check_internal_tag_field_conflict(
        tag_field: &str,
        variant: &Variant,
    ) -> crate::Result<()> {
        match variant {
            Variant::LocalStruct { name, inner, .. } => {
                for field in &inner.value.fields.values {
                    let field_name = field.value.name.borrow_string();
                    if field_name == tag_field {
                        return Err(crate::Error::InternalTagFieldConflict {
                            tag_field: tag_field.to_string(),
                            variant: name.borrow_string().clone(),
                        });
                    }
                }
            },
            Variant::Tuple { .. } => {
                // Tuple variants have no named fields - no conflict possible
            },
        }
        Ok(())
    }

    /// Check tuple variant constraint for internal tagging (TSY-0013).
    ///
    /// Internal tagging inserts a tag field into the variant content.
    /// For tuple variants (e.g., `Success(MyType)`), the referenced type must be a struct.
    fn check_internal_tuple_variant_constraint(variant: &Variant) -> crate::Result<()> {
        if let Variant::Tuple { name, inner, .. } = variant {
            // The inner type should resolve to a struct for internal tagging
            // to work (field insertion). Primitive types, arrays, etc. won't work.
            match inner {
                // Ident references (type names) will be checked at resolution time
                Type::Ident { .. } => {
                    // Type references will be checked at resolution time
                    // Here we just mark the constraint - full validation requires type resolution
                },
                // Struct type is valid
                Type::Struct { .. } => {},
                // Everything else is invalid for internal tagging
                Type::Array { .. }
                | Type::Builtin { .. }
                | Type::Union { .. }
                | Type::UnionOr { .. }
                | Type::OneOf { .. }
                | Type::Paren { .. }
                | Type::Result { .. }
                | Type::TypeExpr { .. } => {
                    return Err(crate::Error::InternalTagTupleNotStruct {
                        variant: name.borrow_string().to_string(),
                    });
                },
            }
        }
        Ok(())
    }

    /// Validate untagged variant distinguishability per TSY-0013.
    ///
    /// Rules:
    /// 1. No duplicate types - each variant must have a unique type signature
    /// 2. Struct variants must have distinguishable required field sets
    fn validate_untagged_distinguishability(
        variants: &[RepeatedItem<Variant, crate::tokens::toks::CommaToken>]
    ) -> crate::Result<()> {
        // Track type signatures to detect duplicates
        let mut type_signatures: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, variant) in variants.iter().enumerate() {
            let sig = Self::compute_type_signature(&variant.value.value);
            type_signatures
                .entry(sig)
                .or_default()
                .push(idx);
        }

        // Check for duplicate types (Rule 1)
        for (type_name, indices) in &type_signatures {
            if indices.len() > 1 {
                return Err(crate::Error::UntaggedDuplicateType {
                    type_name: type_name.clone(),
                    indices: indices
                        .iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                });
            }
        }

        // Check for indistinguishable structs (Rule 2)
        // Collect struct field signatures (only required fields)
        let mut struct_field_sigs: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, variant) in variants.iter().enumerate() {
            if let Some(sig) = Self::compute_struct_field_signature(&variant.value.value) {
                struct_field_sigs
                    .entry(sig)
                    .or_default()
                    .push(idx);
            }
        }

        for (_, indices) in &struct_field_sigs {
            if indices.len() > 1 {
                return Err(crate::Error::UntaggedIndistinguishable {
                    indices: indices
                        .iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                });
            }
        }

        Ok(())
    }

    /// Compute a type signature for duplicate detection.
    /// Returns a string that uniquely identifies the type for comparison.
    fn compute_type_signature(variant: &Variant) -> String {
        match variant {
            Variant::LocalStruct { name, .. } => {
                // Named inline structs use their variant name as signature
                format!("struct:{}", name.borrow_string())
            },
            Variant::Tuple { inner, .. } => {
                // Tuple variants use the inner type as signature
                Self::type_to_signature(inner)
            },
        }
    }

    /// Compute a signature for struct field distinguishability.
    /// Returns None for non-struct types (they don't participate in field-based distinguishability).
    fn compute_struct_field_signature(variant: &Variant) -> Option<String> {
        match variant {
            Variant::LocalStruct { inner, .. } => {
                // Collect required field names (non-optional using Sep::Required)
                let mut required_fields: Vec<String> = inner
                    .value
                    .fields
                    .values
                    .iter()
                    .filter(|f| matches!(f.value.value.sep.value, Sep::Required { .. }))
                    .map(|f| {
                        f.value
                            .value
                            .name
                            .borrow_string()
                            .to_string()
                    })
                    .collect();
                required_fields.sort();
                Some(required_fields.join(","))
            },
            Variant::Tuple { .. } => {
                // Tuple variants don't have named fields
                None
            },
        }
    }

    /// Convert a Type to a signature string for comparison
    fn type_to_signature(ty: &Type) -> String {
        match ty {
            Type::Builtin { ty: builtin } => {
                // Use the variant name for builtin types via formatting
                format!("builtin:{}", Self::builtin_name(&builtin.value))
            },
            Type::Ident { to } => format!("ref:{}", to),
            Type::Array { ty } => format!("array:{}", ty.value.type_name()),
            Type::Struct { ty } => {
                // Struct signature based on fields
                let mut fields: Vec<String> = ty
                    .value
                    .fields
                    .values
                    .iter()
                    .map(|f| {
                        format!(
                            "{}:{}",
                            f.value.value.name.borrow_string(),
                            Self::type_to_signature(&f.value.value.typ)
                        )
                    })
                    .collect();
                fields.sort();
                format!("struct:{{{}}}", fields.join(","))
            },
            Type::Union { .. } => {
                // Union signature - use stable identifier based on span
                "union".to_string()
            },
            Type::UnionOr { .. } => "union_or".to_string(),
            Type::OneOf { .. } => "oneof".to_string(),
            Type::Paren { ty, .. } => Self::type_to_signature(&ty.value),
            Type::Result { ty, .. } => format!("result:{}", Self::type_to_signature(&ty.value)),
            Type::TypeExpr { .. } => {
                // Type expressions need resolution - use placeholder
                "type_expr".to_string()
            },
        }
    }

    /// Get string name for builtin type variant
    fn builtin_name(builtin: &crate::ast::ty::Builtin) -> &'static str {
        use crate::ast::ty::Builtin;
        match builtin {
            Builtin::I8(_) => "i8",
            Builtin::I16(_) => "i16",
            Builtin::I32(_) => "i32",
            Builtin::I64(_) => "i64",
            Builtin::U8(_) => "u8",
            Builtin::U16(_) => "u16",
            Builtin::U32(_) => "u32",
            Builtin::U64(_) => "u64",
            Builtin::Usize(_) => "usize",
            Builtin::F16(_) => "f16",
            Builtin::F32(_) => "f32",
            Builtin::F64(_) => "f64",
            Builtin::Bool(_) => "bool",
            Builtin::Str(_) => "str",
            Builtin::DateTime(_) => "datetime",
            Builtin::Complex(_) => "complex",
            Builtin::Binary(_) => "binary",
            Builtin::Base64(_) => "base64",
            Builtin::Never(_) => "never",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tokens::tokenize;

    #[test]
    fn test_adjacent_same_fields_parses() {
        // Adjacent tagging with name == content should parse - validation happens in resolve
        let source = r#"
            namespace test;

            struct Foo { value: i32 };
            struct Bar { message: str };

            #[tag(name = "t", content = "t")]
            type Response = oneof Foo | Bar;
        "#;

        let mut tokens = tokenize(source).unwrap();
        let items: Vec<crate::defs::Spanned<crate::ast::items::Items>> =
            crate::tokens::Parse::parse(&mut tokens).unwrap();

        // The parsing should succeed - validation happens in resolve phase
        assert!(!items.is_empty());
    }

    #[test]
    fn test_internal_tag_field_parses() {
        // Internal tagging parses even with conflicts - validation happens in resolve
        let source = r#"
            namespace test;

            #[tag(name = "kind")]
            oneof Response {
                Success { kind: str, message: str },
                Error { kind: i32, reason: str }
            };
        "#;

        let mut tokens = tokenize(source).unwrap();
        let items: Vec<crate::defs::Spanned<crate::ast::items::Items>> =
            crate::tokens::Parse::parse(&mut tokens).unwrap();

        assert!(!items.is_empty());
    }

    #[test]
    fn test_valid_internal_tagging_parses() {
        // Internal tagging with non-conflicting field name should parse
        let source = r#"
            namespace test;

            #[tag(name = "type")]
            oneof Response {
                Success { message: str },
                Error { code: i32 }
            };
        "#;

        let mut tokens = tokenize(source).unwrap();
        let items: Vec<crate::defs::Spanned<crate::ast::items::Items>> =
            crate::tokens::Parse::parse(&mut tokens).unwrap();

        assert!(!items.is_empty());
    }

    // Integration tests that use the TypeResolver

    #[tokio::test]
    async fn test_namespace_tag_inheritance() {
        // Test that namespace-level tag is inherited by types without explicit tag
        let source = r#"
            #![tag(name = "kind")]
            namespace test;

            oneof Response {
                Success { message: str },
                Error { code: i32 }
            };
        "#;

        let resolver = crate::tst::resolver_from_source(source)
            .await
            .unwrap();
        // This should complete without errors - namespace tag is inherited
        let result = resolver.resolve().await;
        assert!(result.is_ok(), "Namespace tag inheritance should work");
    }

    #[tokio::test]
    async fn test_type_tag_overrides_namespace() {
        // Test that type-level tag overrides namespace-level tag
        let source = r#"
            #![tag(name = "kind")]
            namespace test;

            #[tag(external)]
            oneof Response {
                Success { message: str },
                Error { code: i32 }
            };
        "#;

        let resolver = crate::tst::resolver_from_source(source)
            .await
            .unwrap();
        // Type-level tag (external) should override namespace tag (internal with name="kind")
        let result = resolver.resolve().await;
        assert!(result.is_ok(), "Type tag override should work");
    }

    #[tokio::test]
    async fn test_untagged_duplicate_type_error() {
        // Test that duplicate types in untagged oneof produce an error
        let source = r#"
            namespace test;

            #[tag(untagged)]
            oneof Value {
                First(str),
                Second(str)
            };
        "#;

        let resolver = crate::tst::resolver_from_source(source)
            .await
            .unwrap();
        let result = resolver.resolve().await;
        // Should fail with duplicate type error
        match result {
            Ok(_) => panic!("Duplicate types in untagged should fail"),
            Err(e) => {
                let err_str = format!("{:?}", e);
                assert!(
                    err_str.contains("UntaggedDuplicateType") || err_str.contains("duplicate"),
                    "Error should mention duplicate type: {}",
                    err_str
                );
            },
        }
    }

    #[tokio::test]
    async fn test_untagged_indistinguishable_structs_error() {
        // Test that structurally identical structs in untagged oneof produce an error
        let source = r#"
            namespace test;

            #[tag(untagged)]
            oneof Response {
                Success { id: i64 },
                Error { id: i64 }
            };
        "#;

        let resolver = crate::tst::resolver_from_source(source)
            .await
            .unwrap();
        let result = resolver.resolve().await;
        // Should fail with indistinguishable variants error
        match result {
            Ok(_) => panic!("Indistinguishable structs in untagged should fail"),
            Err(e) => {
                let err_str = format!("{:?}", e);
                assert!(
                    err_str.contains("Indistinguishable") || err_str.contains("indistinguish"),
                    "Error should mention indistinguishable: {}",
                    err_str
                );
            },
        }
    }

    #[tokio::test]
    async fn test_internal_tuple_must_be_struct() {
        // Test that internal tagging with tuple variant referencing non-struct produces error
        let source = r#"
            namespace test;

            #[tag(name = "type")]
            oneof Value {
                Number(i32),
                Text { message: str }
            };
        "#;

        let resolver = crate::tst::resolver_from_source(source)
            .await
            .unwrap();
        let result = resolver.resolve().await;
        // Should fail because i32 is not a struct and internal tagging requires field insertion
        match result {
            Ok(_) => panic!("Internal tagging with non-struct tuple should fail"),
            Err(e) => {
                let err_str = format!("{:?}", e);
                assert!(
                    err_str.contains("InternalTagTupleNotStruct") || err_str.contains("tuple"),
                    "Error should mention tuple not struct: {}",
                    err_str
                );
            },
        }
    }

    #[tokio::test]
    async fn test_untagged_distinct_types_ok() {
        // Test that distinct types in untagged oneof is valid
        let source = r#"
            namespace test;

            #[tag(untagged)]
            oneof Value {
                Number(i32),
                Text(str)
            };
        "#;

        let resolver = crate::tst::resolver_from_source(source)
            .await
            .unwrap();
        let result = resolver.resolve().await;
        assert!(result.is_ok(), "Distinct types in untagged should work");
    }

    #[tokio::test]
    async fn test_internal_tag_field_conflict_error() {
        // Test that internal tag field conflicting with variant field produces error
        let source = r#"
            namespace test;

            #[tag(name = "kind")]
            oneof Response {
                Success { kind: str, message: str },
                Error { code: i32 }
            };
        "#;

        let resolver = crate::tst::resolver_from_source(source)
            .await
            .unwrap();
        let result = resolver.resolve().await;
        // Should fail with tag field conflict
        match result {
            Ok(_) => panic!("Internal tag field conflict should fail"),
            Err(e) => {
                let err_str = format!("{:?}", e);
                assert!(
                    err_str.contains("InternalTagFieldConflict") || err_str.contains("conflict"),
                    "Error should mention conflict: {}",
                    err_str
                );
            },
        }
    }

    #[tokio::test]
    async fn test_adjacent_same_field_names_error() {
        // Test that adjacent tagging with same tag and content field names produces error
        let source = r#"
            namespace test;

            #[tag(name = "data", content = "data")]
            oneof Response {
                Success { message: str },
                Error { code: i32 }
            };
        "#;

        let resolver = crate::tst::resolver_from_source(source)
            .await
            .unwrap();
        let result = resolver.resolve().await;
        // Should fail with adjacent tag conflict
        match result {
            Ok(_) => panic!("Adjacent with same field names should fail"),
            Err(e) => {
                let err_str = format!("{:?}", e);
                assert!(
                    err_str.contains("AdjacentTagConflict") || err_str.contains("adjacent"),
                    "Error should mention adjacent conflict: {}",
                    err_str
                );
            },
        }
    }
}
