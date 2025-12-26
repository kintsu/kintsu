use crate::{
    ast::{
        items::{Item, StructDef},
        namespace::Namespace,
    },
    ctx::{
        FromNamedSource, NamedItemContext, NamespaceChild, NamespaceCtx, RefContext, SchemaCtx,
        WithSource, compile::schema_compiler::SchemaCompiler, registry::TypeRegistry,
        resolve::TypeResolver,
    },
    defs::Spanned,
    fmt::{FormatConfig, Printer},
    tokens::{self, AstResult, IdentToken, ToTokens, tokenize},
};
use kintsu_manifests::{
    package::{FileConfig, PackageManifest, PackageManifests, PackageMeta},
    version::{VersionSerde, parse_version},
};
pub use kintsu_testing::logging;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

pub fn debug<T: tokens::Parse + serde::Serialize>(t: &T) {
    let dbg = serde_json::to_string(&t).unwrap();
    tracing::info!(data = dbg, "parsed {}", std::any::type_name::<T>());
}

pub fn round_trip<T: tokens::Parse + ToTokens + serde::Serialize>(src: &str) -> AstResult<T> {
    logging();

    let mut tt = tokenize(src)?;

    let t = T::parse(&mut tt)?;

    debug(&t);

    let mut w = Printer::new(&FormatConfig::default());

    w.write(&t);

    let fmt = w.buf.clone();

    let expected = src.replace("    ", "\t");

    assert_eq!(expected, fmt, "source:\n{src}\ngen:\n{fmt}");

    tt.ensure_consumed().unwrap();

    Ok(t)
}

pub fn basic_smoke<T: tokens::Parse + serde::Serialize>(src: &str) -> AstResult<T> {
    logging();

    let mut tt = tokenize(src)?;

    let t = T::parse(&mut tt)?;

    debug(&t);

    tt.ensure_consumed().unwrap();

    Ok(t)
}

pub fn test_ctx(name: &str) -> NamedItemContext {
    logging();

    let item_name = Spanned::call_site(IdentToken::new(name.to_string()));
    NamedItemContext::new(
        item_name,
        RefContext {
            package: "test_package".to_string(),
            namespace: vec![],
        },
    )
}

#[allow(dead_code)]
/// Helper to parse a struct definition from source
pub fn parse_struct(src: &str) -> StructDef {
    basic_smoke(src).expect("struct def")
}

#[allow(dead_code)]
pub fn create_raw_namespace(name: &str) -> NamespaceCtx {
    logging();
    let ctx = RefContext::new("test_package".to_string(), vec![name.to_string()]);

    NamespaceCtx {
        ctx: ctx.clone(),
        sources: Default::default(),
        comments: vec![],
        error: None,
        version: None,
        tag: None,
        namespace: crate::tst::basic_smoke::<Item<Namespace>>(&format!("namespace {name};"))
            .unwrap()
            .with_source("foo.ks".into()), // placeholder
        imports: Vec::new(),
        children: Default::default(),
        registry: TypeRegistry::new(),
        resolved_errors: Default::default(),
        resolved_aliases: Default::default(),
        resolved_versions: Default::default(),
    }
}

#[allow(dead_code)]
pub fn create_test_namespace(name: &str) -> Arc<Mutex<NamespaceCtx>> {
    logging();

    Arc::new(Mutex::new(create_raw_namespace(name)))
}

pub async fn resolver_with(items: Vec<(&str, FromNamedSource<NamespaceChild>)>) -> TypeResolver {
    resolver_with_checked(items)
        .await
        .expect("TypeResolver creation failed when it should have passed")
}

pub async fn resolver_with_checked(
    items: Vec<(&str, FromNamedSource<NamespaceChild>)>
) -> crate::Result<TypeResolver> {
    let mut ns = create_raw_namespace("test");

    for (name, child) in items {
        let child_ctx = ns
            .ctx
            .item(Spanned::call_site(IdentToken::new(name.to_string())));
        ns.children.insert(child_ctx, child);
    }

    let registry = ns.registry.clone();
    let schema = SchemaCtx {
        package: PackageManifests::V1(PackageManifest {
            package: PackageMeta {
                name: "test_package".to_string(),
                version: VersionSerde(parse_version("0.1.0").unwrap()),
                description: None,
                authors: vec![],
                homepage: None,
                keywords: vec![],
                license: None,
                readme: None,
                repository: None,
            },
            files: FileConfig::default(),
            dependencies: BTreeMap::new(),
        }),
        namespaces: vec![("test".into(), Arc::new(Mutex::new(ns)))]
            .into_iter()
            .collect(),
        root_path: PathBuf::from("."),
        registry,
    };

    let schema = Arc::new(schema);

    SchemaCompiler::register_types_recursive(&schema, "test", 0).await?;

    Ok(TypeResolver::new(schema.get_namespace("test").unwrap()))
}

macro_rules! entry_helper {
    ($func_name:ident, $item_type:ty, $ns_child_variant:ident) => {
        paste::paste!{
            #[allow(unused)]
            pub async fn [<add_ $func_name>](
                ns: &mut NamespaceCtx,
                name: &str,
                src: &str,
            ) {
                let struct_def = basic_smoke::<$item_type>(src).unwrap();
                let item_ctx = ns
                    .ctx
                    .item(Spanned::call_site(IdentToken::new(name.to_string())));

                ns.children.insert(
                    item_ctx,
                    NamespaceChild::$ns_child_variant(struct_def).with_source(PathBuf::from("test.ks")),
                );
            }
        }

        #[allow(unused)]
        pub fn $func_name<'a>(
            name: &'a str,
            src: &str,
        ) -> (&'a str, FromNamedSource<NamespaceChild>) {
            let item_def: $item_type = crate::tst::basic_smoke(src).unwrap();
            (
                name,
                NamespaceChild::$ns_child_variant(item_def).with_source(PathBuf::from("test.ks")),
            )
        }
    };
    ($(($func_name:ident, $item_type:ty, $ns_child_variant:ident)), + $(,)?) => {
        $(entry_helper!($func_name, $item_type, $ns_child_variant);)+
    };
}

entry_helper!(
    (error_def, crate::ast::items::ErrorDef, Error),
    (operation_def, crate::ast::items::OperationDef, Operation),
    (struct_def, crate::ast::items::StructDef, Struct),
    (type_alias, crate::ast::items::TypeDef, Type),
    (enum_def, crate::ast::items::EnumDef, Enum),
    (oneof_def, crate::ast::items::OneOfDef, OneOf)
);

// Union Or types use `type_alias` since they're parsed as TypeDef with Type::UnionOr
// Example: type_alias("Combined", "type Combined = A &| B;")
//
// Tagging tests use `oneof_def` with meta attributes when parsing is complete
// Example: oneof_def("Status", "#[tag(name = \"kind\")]\ntype Status = oneof Ok | Err;")
//
// Type expressions use `type_alias` since they're parsed as TypeDef with TypeExpr
// Example: type_alias("UserView", "type UserView = Pick[User, id | name];")
// Full TypeExpr parsing added in Phase 4 (RFC-0018)

pub async fn register_namespace_types(ns: NamespaceCtx) -> crate::Result<Arc<Mutex<NamespaceCtx>>> {
    let registry = ns.registry.clone();

    let schema = SchemaCtx {
        package: PackageManifests::V1(PackageManifest {
            package: PackageMeta {
                name: "test_package".to_string(),
                version: VersionSerde(parse_version("0.1.0").unwrap()),
                description: None,
                authors: vec![],
                homepage: None,
                keywords: vec![],
                license: None,
                readme: None,
                repository: None,
            },
            files: FileConfig::default(),
            dependencies: BTreeMap::new(),
        }),
        namespaces: vec![("test".into(), Arc::new(Mutex::new(ns)))]
            .into_iter()
            .collect(),
        root_path: PathBuf::from("."),
        registry,
    };

    let schema = Arc::new(schema);
    SchemaCompiler::register_types_recursive(&schema, "test", 0).await?;

    let ns = schema.get_namespace("test").unwrap();

    Ok(ns)
}

/// Create a TypeResolver from raw source code (including namespace declaration)
///
/// This parses the full source including namespace declaration and creates a resolver.
/// Use this for integration tests that need to test the full parsing and resolution flow.
pub async fn resolver_from_source(source: &str) -> crate::Result<TypeResolver> {
    use crate::ast::AstStream;
    use std::sync::Arc;

    let source = Arc::new(source.to_string());
    let mut tt = crate::tokens::tokenize(&source).map_err(|e| {
        crate::Error::from(e).with_source(PathBuf::from("test.ks"), Arc::clone(&source))
    })?;

    let ast = AstStream::from_tokens_with(&PathBuf::from("test.ks"), &mut tt)?;

    let ref_ctx = RefContext::new("test_package".to_string(), vec![]);
    let registry = TypeRegistry::new();

    let ns = NamespaceCtx::from_ast_stream(
        ref_ctx,
        ast,
        PathBuf::from("test.ks"),
        Arc::clone(&source),
        registry.clone(),
    )?;

    let ns_name = ns
        .namespace
        .value
        .def
        .name
        .borrow_string()
        .to_string();

    let schema = SchemaCtx {
        package: PackageManifests::V1(PackageManifest {
            package: PackageMeta {
                name: "test_package".to_string(),
                version: VersionSerde(parse_version("0.1.0").unwrap()),
                description: None,
                authors: vec![],
                homepage: None,
                keywords: vec![],
                license: None,
                readme: None,
                repository: None,
            },
            files: FileConfig::default(),
            dependencies: BTreeMap::new(),
        }),
        namespaces: vec![(ns_name.clone(), Arc::new(Mutex::new(ns)))]
            .into_iter()
            .collect(),
        root_path: PathBuf::from("."),
        registry,
    };

    let schema = Arc::new(schema);
    SchemaCompiler::register_types_recursive(&schema, &ns_name, 0).await?;

    Ok(TypeResolver::new(schema.get_namespace(&ns_name).unwrap()))
}
