use super::{
    DeclarationVersion,
    comments::DeclComment,
    context::{DeclNamedItemContext, DeclRefContext},
    definitions::{
        DeclEnumDef, DeclError, DeclOneOf, DeclOneOfVariant, DeclOperation, DeclStruct,
        DeclTypeAlias, TypeDefinition,
    },
    enums::{DeclEnum, DeclIntVariant, DeclStringVariant},
    fields::{DeclArg, DeclField},
    meta::Meta,
    namespace::DeclNamespace,
    root::{DeclarationBundle, TypeRegistryDeclaration},
    types::{Builtin, DeclType},
};
use convert_case::{Case, Casing};
use futures_util::future::{BoxFuture, FutureExt};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Token,
    ast::{
        comment::{CommentAst, CommentStream},
        enm::Enum,
        strct::{Arg, Sep},
        ty::{PathOrIdent, Type as AstType},
        variadic::Variant,
    },
    ctx::{ResolvedType, *},
    defs::{Span, Spanned, Spans},
    tokens::toks::IdentToken,
};

fn extract_comments(comment_stream: &CommentStream) -> DeclComment {
    let comments: Vec<String> = comment_stream
        .comments
        .iter()
        .map(|spanned_comment| {
            match &spanned_comment.value {
                CommentAst::SingleLine(token) => token.borrow_string().to_string(),
                CommentAst::MultiLine(token) => token.borrow_string().to_string(),
            }
        })
        .collect();

    DeclComment::from_vec(comments)
}

impl CompileCtx {
    async fn convert_schema_to_declaration(
        schema: &SchemaCtx,
        registry: &crate::ctx::registry::TypeRegistry,
    ) -> crate::Result<TypeRegistryDeclaration> {
        let package_name = schema.package.package.name.clone();
        let mut external_refs = BTreeSet::new();

        let mut namespaces = BTreeMap::new();
        for ns_arc in schema.namespaces.values() {
            let ns_ctx = ns_arc.lock().await;
            let decl_ns =
                Self::convert_namespace(&ns_ctx, registry, &package_name, &mut external_refs)
                    .await?;
            let ns_name = decl_ns.name.clone();
            namespaces.insert(ns_name, decl_ns);
        }

        let mut declaration = TypeRegistryDeclaration::new(package_name);
        declaration.namespaces = namespaces;
        declaration.extend_refs(external_refs);

        Ok(declaration)
    }

    pub async fn emit_declarations(&self) -> crate::Result<DeclarationVersion> {
        let root_declaration =
            Self::convert_schema_to_declaration(&self.root, &self.type_registry()).await?;

        let root_pkg_name = &root_declaration.package;
        let mut dependency_packages: BTreeSet<String> = BTreeSet::new();

        let all_types = self.type_registry().all_types();
        for (named_ctx, _source, _span) in all_types {
            let pkg_name = &named_ctx.context.package;
            if pkg_name != root_pkg_name {
                dependency_packages.insert(pkg_name.clone());
            }
        }

        let mut dependencies = BTreeMap::new();
        for pkg_name in dependency_packages {
            if let Some(dep_schema) = self.get_dependency(&pkg_name).await {
                let dep_declaration =
                    Self::convert_schema_to_declaration(&dep_schema, &self.type_registry()).await?;
                dependencies.insert(pkg_name, dep_declaration);
            }
        }

        let bundle = DeclarationBundle {
            root: root_declaration,
            dependencies,
        };

        Ok(DeclarationVersion::V1(bundle))
    }

    fn convert_namespace<'a>(
        ns_ctx: &'a NamespaceCtx,
        registry: &'a crate::ctx::registry::TypeRegistry,
        root_package: &'a str,
        external_refs: &'a mut BTreeSet<DeclNamedItemContext>,
    ) -> BoxFuture<'a, crate::Result<DeclNamespace>> {
        let root_package = root_package.to_case(Case::Snake);

        async move {
            let ns_name = ns_ctx
                .namespace
                .value
                .def
                .name
                .borrow_string()
                .clone();

            let version = ns_ctx
                .version
                .as_ref()
                .map(|v| v.version_value() as u32);

            let error = ns_ctx
                .error
                .as_ref()
                .and_then(|e| Self::resolve_path_or_ident(e.error_name(), ns_ctx).ok());

            let mut types = Vec::new();
            let mut nested_namespaces = BTreeMap::new();

            for (named_ctx, child) in &ns_ctx.children {
                if named_ctx.context.package != root_package {
                    continue;
                }

                match &child.value {
                    NamespaceChild::Namespace(nested_ns_ctx) => {
                        let nested_decl = Box::new(
                            Self::convert_namespace(
                                nested_ns_ctx,
                                registry,
                                &root_package,
                                external_refs,
                            )
                            .await?,
                        );
                        let nested_name = nested_decl.name.clone();
                        nested_namespaces.insert(nested_name, nested_decl);
                    },
                    _ => {
                        let Some(resolved) = registry.get(named_ctx) else {
                            return Err(crate::Error::InternalError {
                                message: format!(
                                    "Type '{}' not found in registry during declaration conversion. This is a compiler error.",
                                    named_ctx.name.borrow_string()
                                ),
                            });
                        };
                        types.push(Self::convert_type(
                            named_ctx,
                            &resolved.value.value,
                            ns_ctx,
                            external_refs,
                        )?);
                    },
                }
            }

            // Extract namespace comments from meta
            let mut namespace_comments = DeclComment::new();
            for comment_stream in ns_ctx.namespace.value.comments() {
                namespace_comments.merge(extract_comments(comment_stream));
            }

            Ok(DeclNamespace {
                name: ns_name,
                version,
                error,
                types,
                namespaces: nested_namespaces,
                comments: namespace_comments,
            })
        }
        .boxed()
    }

    fn convert_type(
        named_ctx: &NamedItemContext,
        resolved: &ResolvedType,
        ns_ctx: &NamespaceCtx,
        external_refs: &mut BTreeSet<DeclNamedItemContext>,
    ) -> crate::Result<TypeDefinition> {
        let item_name = named_ctx.name.borrow_string().clone();

        let meta = Meta::from_resolved_version(&item_name, &ns_ctx.resolved_versions)
            .unwrap_or_else(|| Meta::new(1));

        match &resolved.kind {
            Definition::Struct(struct_def) => {
                let fields =
                    Self::convert_struct_fields(&struct_def.def.value.args, ns_ctx, external_refs)?;

                let mut type_comments = DeclComment::new();
                for comment_stream in struct_def.comments() {
                    type_comments.merge(extract_comments(comment_stream));
                }

                Ok(TypeDefinition::Struct(DeclStruct {
                    name: item_name,
                    fields,
                    meta,
                    comments: type_comments,
                }))
            },
            Definition::Enum(enum_def) => {
                let enum_decl = Self::convert_enum(&enum_def.def.value)?;

                let mut type_comments = DeclComment::new();
                for comment_stream in enum_def.comments() {
                    type_comments.merge(extract_comments(comment_stream));
                }

                Ok(TypeDefinition::Enum(DeclEnumDef {
                    name: item_name,
                    enum_def: enum_decl,
                    meta,
                    comments: type_comments,
                }))
            },
            Definition::OneOf(oneof_def) => {
                let variants = Self::convert_oneof_variants(
                    &oneof_def.def.value.variants,
                    ns_ctx,
                    external_refs,
                )?;

                // Extract type-level comments from Item<OneOf>
                let mut type_comments = DeclComment::new();
                for comment_stream in oneof_def.comments() {
                    type_comments.merge(extract_comments(comment_stream));
                }

                Ok(TypeDefinition::OneOf(DeclOneOf {
                    name: item_name,
                    variants,
                    meta,
                    comments: type_comments,
                }))
            },
            Definition::TypeAlias(typedef) => {
                // Check if the type alias targets an anonymous oneof
                // If so, convert it to a TypeDefinition::OneOf instead
                if let AstType::OneOf { ty } = &typedef.def.value.ty.value {
                    let variants =
                        Self::convert_anonymous_oneof_variants(&ty.value, ns_ctx, external_refs)?;

                    let mut type_comments = DeclComment::new();
                    for comment_stream in typedef.comments() {
                        type_comments.merge(extract_comments(comment_stream));
                    }

                    return Ok(TypeDefinition::OneOf(DeclOneOf {
                        name: item_name,
                        variants,
                        meta,
                        comments: type_comments,
                    }));
                }

                let target =
                    Self::convert_ast_type(&typedef.def.value.ty.value, ns_ctx, external_refs)?;

                // Extract type-level comments from Item<TypeAlias>
                let mut type_comments = DeclComment::new();
                for comment_stream in typedef.comments() {
                    type_comments.merge(extract_comments(comment_stream));
                }

                Ok(TypeDefinition::TypeAlias(DeclTypeAlias {
                    name: item_name,
                    target,
                    meta,
                    comments: type_comments,
                }))
            },
            Definition::Error(error_def) => {
                let variants = Self::convert_oneof_variants(
                    &error_def.def.value.variants,
                    ns_ctx,
                    external_refs,
                )?;

                let mut type_comments = DeclComment::new();
                for comment_stream in error_def.comments() {
                    type_comments.merge(extract_comments(comment_stream));
                }

                Ok(TypeDefinition::Error(DeclError {
                    name: item_name,
                    variants,
                    meta,
                    comments: type_comments,
                }))
            },
            Definition::Operation(op_def) => {
                let args =
                    Self::convert_operation_args(&op_def.def.value.args, ns_ctx, external_refs)?;

                let return_type = Self::convert_operation_return_type(
                    &op_def.def.value.return_type.value,
                    &item_name,
                    ns_ctx,
                    external_refs,
                )?;

                let mut type_comments = DeclComment::new();
                for comment_stream in op_def.comments() {
                    type_comments.merge(extract_comments(comment_stream));
                }

                Ok(TypeDefinition::Operation(DeclOperation {
                    name: item_name,
                    args,
                    return_type,
                    meta,
                    comments: type_comments,
                }))
            },
        }
    }

    fn convert_ast_type(
        ast_type: &AstType,
        ns_ctx: &NamespaceCtx,
        external_refs: &mut BTreeSet<DeclNamedItemContext>,
    ) -> crate::Result<DeclType> {
        match ast_type {
            AstType::Builtin { ty } => {
                Ok(DeclType::Builtin {
                    ty: Builtin::from_ast_builtin(ty),
                })
            },

            AstType::Ident { to } => {
                let reference = Self::resolve_path_or_ident(to, ns_ctx)?;

                if reference.is_external(&ns_ctx.ctx.package) {
                    external_refs.insert(reference.clone());
                }

                Ok(DeclType::Named { reference })
            },

            AstType::Array { ty } => {
                use crate::ast::array::Array;

                match &ty.value {
                    Array::Unsized { ty: inner_ty, .. } => {
                        let element_type =
                            Self::convert_ast_type(&inner_ty.value, ns_ctx, external_refs)?;
                        Ok(DeclType::Array {
                            element_type: Box::new(element_type),
                        })
                    },
                    Array::Sized {
                        ty: inner_ty, size, ..
                    } => {
                        let element_type =
                            Self::convert_ast_type(&inner_ty.value, ns_ctx, external_refs)?;
                        let size_value = *size.value.borrow_i32() as u64;
                        Ok(DeclType::SizedArray {
                            element_type: Box::new(element_type),
                            size: size_value,
                        })
                    },
                }
            },

            AstType::Result { .. } => {
                Err(crate::Error::InternalError {
                    message: "Result type conversion requires operation context".into(),
                })
            },

            AstType::Paren { ty, .. } => {
                let inner = Self::convert_ast_type(&ty.value, ns_ctx, external_refs)?;
                Ok(DeclType::Paren {
                    inner_type: Box::new(inner),
                })
            },

            AstType::Union { .. } => {
                Err(crate::Error::InternalError {
                    message: "Unexpected anonymous union in declaration extraction".into(),
                })
            },

            AstType::Struct { .. } => {
                Err(crate::Error::InternalError {
                    message: "Unexpected anonymous struct in declaration extraction".into(),
                })
            },

            AstType::OneOf { .. } => {
                Err(crate::Error::InternalError {
                    message: "Unexpected anonymous oneof in declaration extraction".into(),
                })
            },
        }
    }

    fn convert_operation_return_type(
        ast_type: &AstType,
        operation_name: &str,
        ns_ctx: &NamespaceCtx,
        external_refs: &mut BTreeSet<DeclNamedItemContext>,
    ) -> crate::Result<DeclType> {
        match ast_type {
            AstType::Result { ty, .. } => {
                let ok_type = Self::convert_ast_type(&ty.value, ns_ctx, external_refs)?;

                let error_name = ns_ctx
                    .resolved_errors
                    .get(operation_name)
                    .ok_or_else(|| {
                        crate::Error::InternalError {
                            message: format!(
                                "Missing resolved error for operation {}",
                                operation_name
                            ),
                        }
                    })?;

                let error_ctx = ns_ctx
                    .ctx
                    .item(IdentToken::new(error_name.value.clone()).with_span(Span::CallSite));
                let error_ref = DeclNamedItemContext::from_named_item_context(&error_ctx);

                if error_ref.is_external(&ns_ctx.ctx.package) {
                    external_refs.insert(error_ref.clone());
                }

                Ok(DeclType::Result {
                    ok_type: Box::new(ok_type),
                    error: error_ref,
                })
            },
            _ => Self::convert_ast_type(ast_type, ns_ctx, external_refs),
        }
    }

    fn resolve_path_or_ident(
        path_or_ident: &PathOrIdent,
        ns_ctx: &NamespaceCtx,
    ) -> crate::Result<DeclNamedItemContext> {
        match path_or_ident {
            PathOrIdent::Ident(ident) => {
                let ident_str = ident.borrow_string();

                for import in &ns_ctx.imports {
                    match &import.value {
                        crate::ctx::RefOrItemContext::Ref(ref_ctx) => {
                            if let Some(last_segment) = ref_ctx.namespace.last() {
                                if last_segment == ident_str {
                                    let mut namespace = ref_ctx.namespace.clone();
                                    let name = namespace.pop().unwrap();

                                    return Ok(DeclNamedItemContext {
                                        context: DeclRefContext {
                                            package: ref_ctx.package.clone(),
                                            namespace,
                                        },
                                        name,
                                    });
                                }
                            }
                        },
                        crate::ctx::RefOrItemContext::Item(item_ctx) => {
                            if item_ctx.name.borrow_string() == ident_str {
                                return Ok(DeclNamedItemContext::from_named_item_context(item_ctx));
                            }
                        },
                    }
                }

                let item_ctx = ns_ctx.ctx.item(ident.clone());
                Ok(DeclNamedItemContext::from_named_item_context(&item_ctx))
            },
            PathOrIdent::Path(path) => {
                let path_inner = path.value.borrow_path_inner();
                let path_str = path_inner.to_string();
                let parts: Vec<&str> = path_str.split("::").collect();

                if parts.is_empty() {
                    return Err(crate::Error::InternalError {
                        message: format!("Empty path: {}", path_str),
                    });
                }

                let package = parts[0].to_string();
                let (namespace, name) = if parts.len() == 1 {
                    (ns_ctx.ctx.namespace.clone(), parts[0].to_string())
                } else {
                    // package::ns1::ns2::...::name
                    let namespace = parts[1..parts.len() - 1]
                        .iter()
                        .map(|s| s.to_string())
                        .collect();
                    let name = parts[parts.len() - 1].to_string();
                    (namespace, name)
                };

                Ok(DeclNamedItemContext {
                    context: DeclRefContext { package, namespace },
                    name,
                })
            },
        }
    }

    fn convert_struct_fields(
        args: &crate::tokens::Repeated<Arg, Token![,]>,
        ns_ctx: &NamespaceCtx,
        external_refs: &mut BTreeSet<DeclNamedItemContext>,
    ) -> crate::Result<Vec<DeclField>> {
        let mut decl_fields = Vec::new();

        for arg in &args.values {
            let field_ty = Self::convert_ast_type(&arg.value.typ, ns_ctx, external_refs)?;

            decl_fields.push(DeclField {
                name: arg.value.name.borrow_string().clone(),
                ty: field_ty,
                default_value: None, // TODO: default values
                optional: matches!(arg.value.sep.value, Sep::Optional { .. }),
                comments: extract_comments(&arg.value.comments),
            });
        }

        Ok(decl_fields)
    }

    fn convert_operation_args(
        args: &Option<Spanned<crate::tokens::Repeated<Arg, Token![,]>>>,
        ns_ctx: &NamespaceCtx,
        external_refs: &mut BTreeSet<DeclNamedItemContext>,
    ) -> crate::Result<Vec<DeclArg>> {
        let mut decl_args = Vec::new();

        if let Some(args) = args {
            for arg in &args.value.values {
                let arg_ty = Self::convert_ast_type(&arg.value.typ, ns_ctx, external_refs)?;

                decl_args.push(DeclArg {
                    name: arg.value.name.borrow_string().clone(),
                    ty: arg_ty,
                    default_value: None, // TODO: Extract default value
                    comments: extract_comments(&arg.value.comments),
                });
            }
        }

        Ok(decl_args)
    }

    fn convert_enum(enum_def: &Enum) -> crate::Result<DeclEnum> {
        match enum_def {
            Enum::Int(typed_enum) => {
                let mut variants = Vec::new();
                for variant in &typed_enum.variants.value.values {
                    let enum_variant = &variant.value.value;
                    let value = enum_variant
                        .enum_value()
                        .map(|ev| *ev.inner().borrow_i32() as u32)
                        .unwrap_or(0);

                    variants.push(DeclIntVariant {
                        name: enum_variant.name().to_string(),
                        value,
                        comments: extract_comments(&enum_variant.comments.value),
                    });
                }
                Ok(DeclEnum::Int(variants))
            },
            Enum::Str(typed_enum) => {
                let mut variants = Vec::new();
                for variant in &typed_enum.variants.value.values {
                    let enum_variant = &variant.value.value;
                    let value = enum_variant
                        .enum_value()
                        .map(|ev| ev.inner().borrow_string().clone())
                        .unwrap_or_default();

                    variants.push(DeclStringVariant {
                        name: enum_variant.name().to_string(),
                        value,
                        comments: extract_comments(&enum_variant.comments.value),
                    });
                }
                Ok(DeclEnum::String(variants))
            },
        }
    }

    fn convert_oneof_variants(
        variants: &crate::tokens::Repeated<Variant, Token![,]>,
        ns_ctx: &NamespaceCtx,
        external_refs: &mut BTreeSet<DeclNamedItemContext>,
    ) -> crate::Result<Vec<DeclOneOfVariant>> {
        let mut decl_variants = Vec::new();

        for variant in &variants.values {
            let variant_ty = match &variant.value.value {
                Variant::Tuple { inner, .. } => {
                    Self::convert_ast_type(inner, ns_ctx, external_refs)?
                },
                Variant::LocalStruct { .. } => {
                    return Err(crate::Error::InternalError {
                        message: "Unexpected anonymous struct in oneof variant".into(),
                    });
                },
            };

            let name = match &variant.value.value {
                Variant::Tuple { name, .. } | Variant::LocalStruct { name, .. } => {
                    name.borrow_string().clone()
                },
            };

            let comments = match &variant.value.value {
                Variant::Tuple { comments, .. } | Variant::LocalStruct { comments, .. } => {
                    extract_comments(comments)
                },
            };

            decl_variants.push(DeclOneOfVariant {
                name,
                ty: variant_ty,
                comments,
            });
        }

        Ok(decl_variants)
    }

    /// Convert an anonymous oneof (e.g., `oneof i32 | str`) to DeclOneOfVariant list.
    /// Each variant gets an auto-generated name based on the type.
    fn convert_anonymous_oneof_variants(
        oneof: &crate::ast::one_of::AnonymousOneOf,
        ns_ctx: &NamespaceCtx,
        external_refs: &mut BTreeSet<DeclNamedItemContext>,
    ) -> crate::Result<Vec<DeclOneOfVariant>> {
        let mut decl_variants = Vec::new();

        for (idx, variant) in oneof
            .variants
            .value
            .values
            .iter()
            .enumerate()
        {
            let variant_ty = Self::convert_ast_type(&variant.value.value, ns_ctx, external_refs)?;

            // Generate variant name based on the type
            let name = Self::generate_variant_name_from_type(&variant.value.value, idx);

            decl_variants.push(DeclOneOfVariant {
                name,
                ty: variant_ty,
                comments: DeclComment::new(),
            });
        }

        Ok(decl_variants)
    }

    /// Generate a variant name from a type for anonymous oneofs.
    /// Uses the type name with proper capitalization, or falls back to positional naming.
    fn generate_variant_name_from_type(
        ty: &AstType,
        idx: usize,
    ) -> String {
        match ty {
            AstType::Builtin { ty } => {
                use crate::ast::ty::Builtin;
                // Use the builtin type name as variant name
                match &ty.value {
                    Builtin::I8(_) => "I8".to_string(),
                    Builtin::I16(_) => "I16".to_string(),
                    Builtin::I32(_) => "I32".to_string(),
                    Builtin::I64(_) => "I64".to_string(),
                    Builtin::U8(_) => "U8".to_string(),
                    Builtin::U16(_) => "U16".to_string(),
                    Builtin::U32(_) => "U32".to_string(),
                    Builtin::U64(_) => "U64".to_string(),
                    Builtin::Usize(_) => "Usize".to_string(),
                    Builtin::F16(_) => "F16".to_string(),
                    Builtin::F32(_) => "F32".to_string(),
                    Builtin::F64(_) => "F64".to_string(),
                    Builtin::Bool(_) => "Bool".to_string(),
                    Builtin::Str(_) => "Str".to_string(),
                    Builtin::DateTime(_) => "DateTime".to_string(),
                    Builtin::Complex(_) => "Complex".to_string(),
                    Builtin::Binary(_) => "Binary".to_string(),
                    Builtin::Base64(_) => "Base64".to_string(),
                    Builtin::Never(_) => "Never".to_string(),
                }
            },
            AstType::Ident { to } => {
                // Use the identifier name as the variant name
                // For paths like foo::Bar, use the last segment
                match to {
                    crate::ast::ty::PathOrIdent::Ident(ident) => ident.borrow_string().clone(),
                    crate::ast::ty::PathOrIdent::Path(path) => {
                        let path_inner = path.value.borrow_path_inner();
                        path_inner
                            .segments()
                            .last()
                            .cloned()
                            .unwrap_or_else(|| format!("Variant{}", idx))
                    },
                }
            },
            AstType::Array { .. } => format!("Array{}", idx),
            AstType::Paren { ty, .. } => Self::generate_variant_name_from_type(&ty.value, idx),
            _ => format!("Variant{}", idx),
        }
    }
}
