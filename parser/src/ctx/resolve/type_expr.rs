//! Type Expression Resolution Phase (Phase 3.6)
//!
//! Resolves `TypeExpr` nodes by evaluating type transformation operators.
//! This phase runs after `resolve_union_or` and before `validate_unions`.
//!
//! **Spec references:** RFC-0018, SPEC-0017, TSY-0014
//!
//! # Design Notes
//!
//! Type expressions are FULLY RESOLVED at compile time - declarations never
//! contain DeclType::TypeExpr. For example:
//! - `type UserSummary = Pick[User, id | name]` resolves to a struct with only id and name fields
//! - `type SuccessOnly = Extract[Result, Success]` resolves to a oneof with only Success variant

use std::collections::{BTreeMap, HashSet};

use crate::{
    Token,
    ast::{
        one_of::AnonymousOneOf,
        strct::{Arg, Sep},
        ty::{PathOrIdent, Type},
        type_expr::{SelectorList, TypeExpr, TypeExprOp, VariantList},
    },
    ctx::{NamespaceCtx, common::NamespaceChild, resolve::TypeResolver},
    defs::Spanned,
    tokens::{Brace, IdentToken, KwOneofToken, Repeated, RepeatedItem},
};

pub fn selector_list_to_strings(list: &SelectorList) -> Vec<String> {
    list.fields
        .iter()
        .map(|f| f.value.borrow_string().clone())
        .collect()
}

pub fn variant_list_to_strings(list: &VariantList) -> Vec<String> {
    list.variants
        .iter()
        .map(|v| v.value.borrow_string().clone())
        .collect()
}

impl TypeResolver {
    /// Resolve type expressions (Phase 3.6)
    ///
    /// Fully resolves type expression operators per RFC-0018, SPEC-0017, TSY-0014.
    /// Type expressions are evaluated at compile time.
    pub(super) async fn resolve_type_expressions(&mut self) -> crate::Result<()> {
        tracing::debug!("resolve_type_expressions: starting phase 3.6");

        let ns = self.namespace.lock().await;

        // Find all type aliases that contain type expressions
        let type_expr_aliases: Vec<_> = ns
            .children
            .iter()
            .filter_map(|(ctx, child)| {
                if let NamespaceChild::Type(type_def) = &child.value {
                    if contains_type_expr(&type_def.def.value.ty.value) {
                        return Some((
                            ctx.name.borrow_string().clone(),
                            type_def.def.value.ty.clone(),
                            child.source.clone(),
                        ));
                    }
                }
                None
            })
            .collect();

        drop(ns);

        // Track which aliases we're currently resolving (for cycle detection)
        let mut resolving: HashSet<String> = HashSet::new();

        for (alias_name, type_spanned, _source) in type_expr_aliases {
            tracing::debug!(
                "resolve_type_expressions: processing alias '{}'",
                alias_name
            );

            // Check for cycles
            if resolving.contains(&alias_name) {
                return Err(crate::Error::TypeExprCycle {
                    chain: vec![alias_name],
                });
            }
            resolving.insert(alias_name.clone());

            // Acquire namespace lock for resolution
            let ns = self.namespace.lock().await;

            // Resolve the type expression
            let resolved_type = self
                .resolve_type_expr_in_type(&type_spanned.value, &ns)
                .await?;

            drop(ns);

            // Store the resolved type
            self.resolution
                .resolved_aliases
                .insert(alias_name.clone(), Spanned::call_site(resolved_type));

            resolving.remove(&alias_name);
        }

        tracing::debug!("resolve_type_expressions: phase 3.6 complete");
        Ok(())
    }

    /// Recursively resolve a Type that may contain TypeExpr
    fn resolve_type_expr_in_type<'a>(
        &'a self,
        typ: &'a Type,
        ns: &'a NamespaceCtx,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<Type>> + Send + 'a>> {
        Box::pin(async move {
            match typ {
                Type::TypeExpr { expr } => {
                    self.resolve_type_expr_node(&expr.value, ns)
                        .await
                },
                Type::Paren { paren, ty } => {
                    let inner = self
                        .resolve_type_expr_in_type(&ty.value, ns)
                        .await?;
                    Ok(Type::Paren {
                        paren: paren.clone(),
                        ty: Spanned::call_site(Box::new(inner)),
                    })
                },
                Type::Array { ty } => {
                    // Handle Array enum - can be Unsized or Sized
                    let resolved = match &ty.value {
                        crate::ast::array::Array::Unsized {
                            ty: inner_ty,
                            bracket,
                        } => {
                            let inner = self
                                .resolve_type_expr_in_type(&inner_ty.value, ns)
                                .await?;
                            crate::ast::array::Array::Unsized {
                                ty: Box::new(Spanned::call_site(inner)),
                                bracket: bracket.clone(),
                            }
                        },
                        crate::ast::array::Array::Sized {
                            ty: inner_ty,
                            bracket,
                            size,
                        } => {
                            let inner = self
                                .resolve_type_expr_in_type(&inner_ty.value, ns)
                                .await?;
                            crate::ast::array::Array::Sized {
                                ty: Box::new(Spanned::call_site(inner)),
                                bracket: bracket.clone(),
                                size: size.clone(),
                            }
                        },
                    };
                    Ok(Type::Array {
                        ty: Spanned::call_site(resolved),
                    })
                },
                // Non-type-expr types pass through unchanged
                _ => Ok(typ.clone()),
            }
        })
    }

    /// Resolve a TypeExpr AST node to a concrete Type
    async fn resolve_type_expr_node(
        &self,
        expr: &TypeExpr,
        ns: &NamespaceCtx,
    ) -> crate::Result<Type> {
        match expr {
            TypeExpr::TypeRef { reference } => {
                Ok(Type::Ident {
                    to: reference.clone(),
                })
            },
            TypeExpr::FieldAccess { .. } => {
                Err(crate::Error::InternalError {
                    message: "Field access in type expressions not yet supported".into(),
                })
            },
            TypeExpr::Op(spanned_op) => {
                self.resolve_type_expr_op(&spanned_op.value, ns)
                    .await
            },
        }
    }

    /// Resolve a type expression operator
    async fn resolve_type_expr_op(
        &self,
        op: &TypeExprOp,
        ns: &NamespaceCtx,
    ) -> crate::Result<Type> {
        match op {
            TypeExprOp::Pick { target, fields } => self.resolve_pick(target, fields, ns).await,
            TypeExprOp::Omit { target, fields } => self.resolve_omit(target, fields, ns).await,
            TypeExprOp::Partial { target, fields } => {
                self.resolve_partial(target, fields.as_ref(), ns)
                    .await
            },
            TypeExprOp::Required { target, fields } => {
                self.resolve_required(target, fields.as_ref(), ns)
                    .await
            },
            TypeExprOp::Exclude { target, variants } => {
                self.resolve_exclude(target, variants, ns)
                    .await
            },
            TypeExprOp::Extract { target, variants } => {
                self.resolve_extract(target, variants, ns)
                    .await
            },
            TypeExprOp::ArrayItem { target } => self.resolve_array_item(target, ns).await,
        }
    }

    /// Resolve Pick[T, f1 | f2]: Select specific fields from struct
    async fn resolve_pick(
        &self,
        target: &TypeExpr,
        fields: &SelectorList,
        ns: &NamespaceCtx,
    ) -> crate::Result<Type> {
        let struct_fields = self.get_struct_fields(target, ns).await?;
        let selected: HashSet<_> = selector_list_to_strings(fields)
            .into_iter()
            .collect();
        let target_name = type_expr_name(target);

        // Validate all selectors exist
        for field_name in &selected {
            if !struct_fields.contains_key(field_name) {
                return Err(crate::Error::TypeExprFieldNotFound {
                    operator: "Pick".into(),
                    field: field_name.clone(),
                    type_name: target_name,
                });
            }
        }

        // Build struct with only selected fields
        let picked: Vec<_> = struct_fields
            .into_iter()
            .filter(|(name, _)| selected.contains(name))
            .map(|(_, (arg, sep))| RepeatedItem { value: arg, sep })
            .collect();

        if picked.is_empty() {
            return Err(crate::Error::TypeExprNoFieldsRemain {
                operator: "Pick".into(),
            });
        }

        Ok(build_struct_type(picked))
    }

    /// Resolve Omit[T, f1 | f2]: Remove specific fields from struct
    async fn resolve_omit(
        &self,
        target: &TypeExpr,
        fields: &SelectorList,
        ns: &NamespaceCtx,
    ) -> crate::Result<Type> {
        let struct_fields = self.get_struct_fields(target, ns).await?;
        let omitted: HashSet<_> = selector_list_to_strings(fields)
            .into_iter()
            .collect();
        let target_name = type_expr_name(target);

        // Validate all selectors exist
        for field_name in &omitted {
            if !struct_fields.contains_key(field_name) {
                return Err(crate::Error::TypeExprFieldNotFound {
                    operator: "Omit".into(),
                    field: field_name.clone(),
                    type_name: target_name,
                });
            }
        }

        // Build struct without omitted fields
        let remaining: Vec<_> = struct_fields
            .into_iter()
            .filter(|(name, _)| !omitted.contains(name))
            .map(|(_, (arg, sep))| RepeatedItem { value: arg, sep })
            .collect();

        if remaining.is_empty() {
            return Err(crate::Error::TypeExprNoFieldsRemain {
                operator: "Omit".into(),
            });
        }

        Ok(build_struct_type(remaining))
    }

    /// Resolve Partial[T] or Partial[T, f1 | f2]: Make fields optional
    async fn resolve_partial(
        &self,
        target: &TypeExpr,
        fields: Option<&SelectorList>,
        ns: &NamespaceCtx,
    ) -> crate::Result<Type> {
        let struct_fields = self.get_struct_fields(target, ns).await?;
        let target_name = type_expr_name(target);

        let selected: Option<HashSet<_>> = fields.map(|f| {
            selector_list_to_strings(f)
                .into_iter()
                .collect()
        });

        // Validate selectors if provided
        if let Some(ref sel) = selected {
            for field_name in sel {
                if !struct_fields.contains_key(field_name) {
                    return Err(crate::Error::TypeExprFieldNotFound {
                        operator: "Partial".into(),
                        field: field_name.clone(),
                        type_name: target_name,
                    });
                }
            }
        }

        // Build struct with selected fields made optional
        let result: Vec<_> = struct_fields
            .into_iter()
            .map(|(name, (arg, sep))| {
                let make_optional = selected
                    .as_ref()
                    .map_or(true, |s| s.contains(&name));
                let new_sep = if make_optional {
                    Spanned::call_site(Sep::Optional {
                        q: Spanned::call_site(<Token![?]>::new()),
                        sep: Spanned::call_site(<Token![:]>::new()),
                    })
                } else {
                    arg.value.sep.clone()
                };

                RepeatedItem {
                    value: Spanned::call_site(Arg {
                        comments: arg.value.comments.clone(),
                        name: arg.value.name.clone(),
                        sep: new_sep,
                        typ: arg.value.typ.clone(),
                    }),
                    sep,
                }
            })
            .collect();

        Ok(build_struct_type(result))
    }

    /// Resolve Required[T] or Required[T, f1 | f2]: Make fields required
    async fn resolve_required(
        &self,
        target: &TypeExpr,
        fields: Option<&SelectorList>,
        ns: &NamespaceCtx,
    ) -> crate::Result<Type> {
        let struct_fields = self.get_struct_fields(target, ns).await?;
        let target_name = type_expr_name(target);

        let selected: Option<HashSet<_>> = fields.map(|f| {
            selector_list_to_strings(f)
                .into_iter()
                .collect()
        });

        // Validate selectors if provided
        if let Some(ref sel) = selected {
            for field_name in sel {
                if !struct_fields.contains_key(field_name) {
                    return Err(crate::Error::TypeExprFieldNotFound {
                        operator: "Required".into(),
                        field: field_name.clone(),
                        type_name: target_name,
                    });
                }
            }
        }

        // Build struct with selected fields made required
        let result: Vec<_> = struct_fields
            .into_iter()
            .map(|(name, (arg, sep))| {
                let make_required = selected
                    .as_ref()
                    .map_or(true, |s| s.contains(&name));
                let new_sep = if make_required {
                    Spanned::call_site(Sep::Required {
                        sep: Spanned::call_site(<Token![:]>::new()),
                    })
                } else {
                    arg.value.sep.clone()
                };

                RepeatedItem {
                    value: Spanned::call_site(Arg {
                        comments: arg.value.comments.clone(),
                        name: arg.value.name.clone(),
                        sep: new_sep,
                        typ: arg.value.typ.clone(),
                    }),
                    sep,
                }
            })
            .collect();

        Ok(build_struct_type(result))
    }

    /// Resolve Exclude[T, V1 | V2]: Remove variants from oneof
    async fn resolve_exclude(
        &self,
        target: &TypeExpr,
        variants: &VariantList,
        ns: &NamespaceCtx,
    ) -> crate::Result<Type> {
        let oneof_variants = self.get_oneof_variants(target, ns).await?;
        let excluded: HashSet<_> = variant_list_to_strings(variants)
            .into_iter()
            .collect();
        let _target_name = type_expr_name(target);

        // Build oneof without excluded variants
        let remaining: Vec<_> = oneof_variants
            .into_iter()
            .filter(|(name, _)| !excluded.contains(name))
            .map(|(_, item)| item)
            .collect();

        if remaining.is_empty() {
            return Err(crate::Error::TypeExprNoVariantsRemain {
                operator: "Exclude".into(),
            });
        }

        Ok(build_oneof_type(remaining))
    }

    /// Resolve Extract[T, V1 | V2]: Keep only selected variants
    async fn resolve_extract(
        &self,
        target: &TypeExpr,
        variants: &VariantList,
        ns: &NamespaceCtx,
    ) -> crate::Result<Type> {
        let oneof_variants = self.get_oneof_variants(target, ns).await?;
        let selected: HashSet<_> = variant_list_to_strings(variants)
            .into_iter()
            .collect();
        let _target_name = type_expr_name(target);

        // Build oneof with only selected variants
        let extracted: Vec<_> = oneof_variants
            .into_iter()
            .filter(|(name, _)| selected.contains(name))
            .map(|(_, item)| item)
            .collect();

        if extracted.is_empty() {
            return Err(crate::Error::TypeExprNoVariantsRemain {
                operator: "Extract".into(),
            });
        }

        Ok(build_oneof_type(extracted))
    }

    /// Resolve ArrayItem[T[]]: Get element type of array
    async fn resolve_array_item(
        &self,
        target: &TypeExpr,
        ns: &NamespaceCtx,
    ) -> crate::Result<Type> {
        let target_type = self.resolve_type_ref(target, ns).await?;

        match &target_type {
            Type::Array { ty } => {
                match &ty.value {
                    crate::ast::array::Array::Unsized { ty: inner, .. } => Ok(inner.value.clone()),
                    crate::ast::array::Array::Sized { ty: inner, .. } => Ok(inner.value.clone()),
                }
            },
            _ => {
                Err(crate::Error::TypeExprTargetKindMismatch {
                    operator: "ArrayItem".into(),
                    expected: "array".into(),
                    actual: target_type.type_name(),
                })
            },
        }
    }

    /// Get struct fields from a type expression target
    async fn get_struct_fields(
        &self,
        expr: &TypeExpr,
        ns: &NamespaceCtx,
    ) -> crate::Result<BTreeMap<String, (Spanned<Arg>, Option<Spanned<Token![,]>>)>> {
        let target_type = self.resolve_type_ref(expr, ns).await?;
        extract_struct_fields(&target_type, ns, &self.resolution.resolved_aliases).await
    }

    /// Get named oneof variants from a type expression target (for Exclude/Extract)
    /// Returns a list of (variant_name, RepeatedItem) pairs
    async fn get_oneof_variants(
        &self,
        expr: &TypeExpr,
        ns: &NamespaceCtx,
    ) -> crate::Result<Vec<(String, RepeatedItem<Type, Token![|]>)>> {
        // For named types, go directly to lookup_named_variants to preserve variant names
        match expr {
            TypeExpr::TypeRef { reference } => {
                let ident_name = match reference {
                    PathOrIdent::Ident(ident) => ident.borrow_string().clone(),
                    PathOrIdent::Path(path) => {
                        path.value
                            .borrow_path_inner()
                            .segments()
                            .last()
                            .cloned()
                            .unwrap_or_default()
                    },
                };
                // Go directly to lookup_named_variants to preserve variant names
                lookup_named_variants(&ident_name, ns, &self.resolution.resolved_aliases).await
            },
            TypeExpr::Op(op) => {
                // Nested type expression - resolve then extract
                let typ = Box::pin(self.resolve_type_expr_op(&op.value, ns)).await?;
                extract_oneof_variants(&typ, ns, &self.resolution.resolved_aliases, "anonymous")
                    .await
            },
            TypeExpr::FieldAccess { .. } => {
                Err(crate::Error::InternalError {
                    message: "Field access in type expressions not yet supported".into(),
                })
            },
        }
    }

    /// Resolve a TypeExpr to its target Type
    async fn resolve_type_ref(
        &self,
        expr: &TypeExpr,
        ns: &NamespaceCtx,
    ) -> crate::Result<Type> {
        match expr {
            TypeExpr::TypeRef { reference } => {
                let ident_name = match reference {
                    PathOrIdent::Ident(ident) => ident.borrow_string().clone(),
                    PathOrIdent::Path(path) => {
                        // For paths, use the last segment
                        path.value
                            .borrow_path_inner()
                            .segments()
                            .last()
                            .cloned()
                            .unwrap_or_default()
                    },
                };

                // Check resolved aliases first
                if let Some(resolved) = self
                    .resolution
                    .resolved_aliases
                    .get(&ident_name)
                {
                    return Ok(resolved.value.clone());
                }

                // Look up in namespace
                lookup_type(&ident_name, ns, &self.resolution.resolved_aliases).await
            },
            TypeExpr::Op(op) => {
                // Nested type expression - resolve recursively
                Box::pin(self.resolve_type_expr_op(&op.value, ns)).await
            },
            TypeExpr::FieldAccess { .. } => {
                Err(crate::Error::InternalError {
                    message: "Field access in type expressions not yet supported".into(),
                })
            },
        }
    }
}

/// Check if a type contains any TypeExpr nodes
fn contains_type_expr(typ: &Type) -> bool {
    match typ {
        Type::TypeExpr { .. } => true,
        Type::Paren { ty, .. } => contains_type_expr(&ty.value),
        Type::Array { ty } => {
            match &ty.value {
                crate::ast::array::Array::Unsized { ty: inner, .. } => {
                    contains_type_expr(&inner.value)
                },
                crate::ast::array::Array::Sized { ty: inner, .. } => {
                    contains_type_expr(&inner.value)
                },
            }
        },
        Type::Result { ty, .. } => contains_type_expr(&ty.value),
        // Union types are composed of identifiers/anonymous structs, not arbitrary Types
        // They cannot directly contain TypeExpr
        Type::Union { .. } => false,
        Type::UnionOr { lhs, rhs, .. } => {
            contains_type_expr(&lhs.value) || contains_type_expr(&rhs.value)
        },
        _ => false,
    }
}

/// Get name for a type expression (for error messages)
fn type_expr_name(expr: &TypeExpr) -> String {
    match expr {
        TypeExpr::TypeRef { reference } => reference.to_string(),
        TypeExpr::FieldAccess { base, field, .. } => {
            format!("{}::{}", type_expr_name(&base.value), field.borrow_string())
        },
        TypeExpr::Op(op) => {
            let kw = match &op.value {
                TypeExprOp::Pick { .. } => "Pick",
                TypeExprOp::Omit { .. } => "Omit",
                TypeExprOp::Partial { .. } => "Partial",
                TypeExprOp::Required { .. } => "Required",
                TypeExprOp::Exclude { .. } => "Exclude",
                TypeExprOp::Extract { .. } => "Extract",
                TypeExprOp::ArrayItem { .. } => "ArrayItem",
            };
            format!("{}[...]", kw)
        },
    }
}

/// Build a struct Type from fields
fn build_struct_type(fields: Vec<RepeatedItem<Arg, Token![,]>>) -> Type {
    Type::Struct {
        ty: Spanned::call_site(crate::ast::anonymous::AnonymousStruct {
            brace: Brace::call_site(),
            fields: Spanned::call_site(Repeated { values: fields }),
        }),
    }
}

/// Build an anonymous oneof Type from type variants
fn build_oneof_type(variants: Vec<RepeatedItem<Type, Token![|]>>) -> Type {
    Type::OneOf {
        ty: Spanned::call_site(AnonymousOneOf {
            kw: Spanned::call_site(KwOneofToken::new()),
            variants: Spanned::call_site(Repeated { values: variants }),
        }),
    }
}

/// Look up a type definition by name
async fn lookup_type(
    name: &str,
    ns: &NamespaceCtx,
    resolved_aliases: &BTreeMap<String, Spanned<Type>>,
) -> crate::Result<Type> {
    let child_ctx = ns
        .ctx
        .item(Spanned::call_site(IdentToken::new(name.to_string())));

    if let Some(child) = ns.children.get(&child_ctx) {
        match &child.value {
            NamespaceChild::Struct(struct_def) => {
                Ok(Type::Struct {
                    ty: Spanned::call_site(crate::ast::anonymous::AnonymousStruct {
                        brace: Brace::call_site(),
                        fields: Spanned::call_site(Repeated {
                            values: struct_def.def.value.args.values.clone(),
                        }),
                    }),
                })
            },
            NamespaceChild::OneOf(oneof_def) => {
                // Convert named variants to anonymous oneof types
                // For LocalStruct variants, use a reference to the extracted named struct
                let type_variants: Vec<_> = oneof_def
                    .def
                    .value
                    .variants
                    .values
                    .iter()
                    .map(|item| {
                        let typ = match &item.value.value {
                            crate::ast::variadic::Variant::Tuple { inner, .. } => inner.clone(),
                            crate::ast::variadic::Variant::LocalStruct {
                                name: variant_name,
                                ..
                            } => {
                                // Reference the extracted named struct (e.g., StatusPending)
                                let struct_name =
                                    format!("{}{}", name, variant_name.borrow_string());
                                Type::Ident {
                                    to: PathOrIdent::Ident(Spanned::call_site(IdentToken::new(
                                        struct_name,
                                    ))),
                                }
                            },
                        };
                        RepeatedItem {
                            value: Spanned::call_site(typ),
                            sep: item
                                .sep
                                .as_ref()
                                .map(|_| Spanned::call_site(<Token![|]>::new())),
                        }
                    })
                    .collect();

                Ok(Type::OneOf {
                    ty: Spanned::call_site(AnonymousOneOf {
                        kw: Spanned::call_site(KwOneofToken::new()),
                        variants: Spanned::call_site(Repeated {
                            values: type_variants,
                        }),
                    }),
                })
            },
            NamespaceChild::Type(type_def) => {
                // Check if already resolved
                if let Some(resolved) = resolved_aliases.get(name) {
                    return Ok(resolved.value.clone());
                }
                // Follow the type alias
                Ok(type_def.def.value.ty.value.clone())
            },
            _ => {
                Err(crate::Error::TypeExprTargetKindMismatch {
                    operator: "<lookup>".into(),
                    expected: "struct or oneof".into(),
                    actual: child.value.type_name(),
                })
            },
        }
    } else {
        Err(crate::Error::UndefinedType {
            name: name.to_string(),
        })
    }
}

/// Extract fields from a struct type
async fn extract_struct_fields(
    typ: &Type,
    ns: &NamespaceCtx,
    resolved_aliases: &BTreeMap<String, Spanned<Type>>,
) -> crate::Result<BTreeMap<String, (Spanned<Arg>, Option<Spanned<Token![,]>>)>> {
    match typ {
        Type::Struct { ty } => {
            Ok(ty
                .value
                .fields
                .value
                .values
                .iter()
                .map(|item| {
                    (
                        item.value.name.borrow_string().clone(),
                        (item.value.clone(), item.sep.clone()),
                    )
                })
                .collect())
        },
        Type::Ident { to } => {
            let ident_name = match to {
                PathOrIdent::Ident(ident) => ident.borrow_string().clone(),
                PathOrIdent::Path(_) => {
                    return Err(crate::Error::TypeExprTargetKindMismatch {
                        operator: "<struct extraction>".into(),
                        expected: "struct".into(),
                        actual: "path reference".into(),
                    });
                },
            };

            // Check resolved aliases
            if let Some(resolved) = resolved_aliases.get(&ident_name) {
                return Box::pin(extract_struct_fields(&resolved.value, ns, resolved_aliases))
                    .await;
            }

            // Look up and extract
            let resolved = lookup_type(&ident_name, ns, resolved_aliases).await?;
            Box::pin(extract_struct_fields(&resolved, ns, resolved_aliases)).await
        },
        Type::Paren { ty, .. } => {
            Box::pin(extract_struct_fields(&ty.value, ns, resolved_aliases)).await
        },
        _ => {
            Err(crate::Error::TypeExprTargetKindMismatch {
                operator: "<struct extraction>".into(),
                expected: "struct".into(),
                actual: typ.type_name(),
            })
        },
    }
}

/// Extract named variants from a oneof type for Exclude/Extract operations
/// Returns (variant_name, RepeatedItem) pairs
async fn extract_oneof_variants(
    typ: &Type,
    ns: &NamespaceCtx,
    resolved_aliases: &BTreeMap<String, Spanned<Type>>,
    oneof_name: &str,
) -> crate::Result<Vec<(String, RepeatedItem<Type, Token![|]>)>> {
    match typ {
        Type::OneOf { ty } => {
            // For anonymous oneofs, use type_name() as variant name
            Ok(ty
                .value
                .variants
                .value
                .values
                .iter()
                .map(|item| (item.value.value.type_name(), item.clone()))
                .collect())
        },
        Type::Ident { to } => {
            let ident_name = match to {
                PathOrIdent::Ident(ident) => ident.borrow_string().clone(),
                PathOrIdent::Path(_) => {
                    return Err(crate::Error::TypeExprTargetKindMismatch {
                        operator: "<oneof variant extraction>".into(),
                        expected: "oneof".into(),
                        actual: "path reference".into(),
                    });
                },
            };

            // Check resolved aliases
            if let Some(resolved) = resolved_aliases.get(&ident_name) {
                return Box::pin(extract_oneof_variants(
                    &resolved.value,
                    ns,
                    resolved_aliases,
                    &ident_name,
                ))
                .await;
            }

            // Look up in namespace to get named variants directly
            lookup_named_variants(&ident_name, ns, resolved_aliases).await
        },
        Type::Paren { ty, .. } => {
            Box::pin(extract_oneof_variants(
                &ty.value,
                ns,
                resolved_aliases,
                oneof_name,
            ))
            .await
        },
        _ => {
            Err(crate::Error::TypeExprTargetKindMismatch {
                operator: "<oneof variant extraction>".into(),
                expected: "oneof".into(),
                actual: typ.type_name(),
            })
        },
    }
}

/// Look up a named OneOf and return its variants with their names
async fn lookup_named_variants(
    name: &str,
    ns: &NamespaceCtx,
    resolved_aliases: &BTreeMap<String, Spanned<Type>>,
) -> crate::Result<Vec<(String, RepeatedItem<Type, Token![|]>)>> {
    let child_ctx = ns
        .ctx
        .item(Spanned::call_site(IdentToken::new(name.to_string())));

    if let Some(child) = ns.children.get(&child_ctx) {
        match &child.value {
            NamespaceChild::OneOf(oneof_def) => {
                // Return variant names paired with their types
                // For LocalStruct variants, use a reference to the extracted named struct
                // (e.g., Status.Pending { since: i64 } -> StatusPending)
                Ok(oneof_def
                    .def
                    .value
                    .variants
                    .values
                    .iter()
                    .map(|item| {
                        let variant_name = match &item.value.value {
                            crate::ast::variadic::Variant::Tuple { name, .. } => {
                                name.borrow_string().clone()
                            },
                            crate::ast::variadic::Variant::LocalStruct { name, .. } => {
                                name.borrow_string().clone()
                            },
                        };
                        let typ = match &item.value.value {
                            crate::ast::variadic::Variant::Tuple { inner, .. } => inner.clone(),
                            crate::ast::variadic::Variant::LocalStruct {
                                name: variant_name_tok,
                                ..
                            } => {
                                // Reference the extracted named struct (e.g., StatusPending)
                                let struct_name =
                                    format!("{}{}", name, variant_name_tok.borrow_string());
                                Type::Ident {
                                    to: PathOrIdent::Ident(Spanned::call_site(IdentToken::new(
                                        struct_name,
                                    ))),
                                }
                            },
                        };
                        (
                            variant_name,
                            RepeatedItem {
                                value: Spanned::call_site(typ),
                                sep: item
                                    .sep
                                    .as_ref()
                                    .map(|_| Spanned::call_site(<Token![|]>::new())),
                            },
                        )
                    })
                    .collect())
            },
            NamespaceChild::Type(type_def) => {
                // Follow type alias
                if let Some(resolved) = resolved_aliases.get(name) {
                    return Box::pin(extract_oneof_variants(
                        &resolved.value,
                        ns,
                        resolved_aliases,
                        name,
                    ))
                    .await;
                }
                Box::pin(extract_oneof_variants(
                    &type_def.def.value.ty.value,
                    ns,
                    resolved_aliases,
                    name,
                ))
                .await
            },
            _ => {
                Err(crate::Error::TypeExprTargetKindMismatch {
                    operator: "<named variant lookup>".into(),
                    expected: "oneof".into(),
                    actual: child.value.type_name(),
                })
            },
        }
    } else {
        Err(crate::Error::UndefinedType {
            name: name.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::tokenize;

    #[test]
    fn pick_type_alias_parses() {
        let source = "type UserSummary = Pick[User, id | name];";
        let _ = tokenize(source).unwrap();
    }

    #[test]
    fn nested_type_expr_parses() {
        let source = "type StrictUser = Required[Omit[User, password_hash]];";
        let _ = tokenize(source).unwrap();
    }
}
