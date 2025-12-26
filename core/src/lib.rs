#![allow(clippy::iter_kv_map, clippy::result_large_err)]

pub mod context;

pub mod checks;
pub mod convert;
pub mod namespace;
pub mod ty;
pub(crate) mod utils;
use std::marker::PhantomData;

pub mod protocol;
pub use paste::paste;
pub use ty::*;

pub mod declare {
    //! Re-exports of parser declaration types for unified type system.
    //!
    //! These types represent the canonical declaration format used for code generation
    //! and schema serialization.
    pub use kintsu_parser::declare::{
        Builtin, DeclArg, DeclComment, DeclEnum, DeclEnumDef, DeclEnumValueType, DeclError,
        DeclField, DeclIntVariant, DeclNamedItemContext, DeclNamespace, DeclOneOf,
        DeclOneOfVariant, DeclOperation, DeclRefContext, DeclStringVariant, DeclStruct, DeclType,
        DeclTypeAlias, DeclarationBundle, DeclarationVersion, Meta as DeclMeta, TypeDefinition,
        TypeRegistryDeclaration,
    };
}

#[cfg(feature = "generate")]
pub mod generate;

#[macro_export]
macro_rules! ty {
    ($t: tt) => {
        <$t as $crate::Typed>::ty()
    };
}

/// Trait for types that have a legacy `Definitions` descriptor.
///
/// This trait is used by derive macros to provide type metadata.
/// For new code, consider using `DeclDefined` which returns the
/// unified `TypeDefinition` type.
pub trait Defined: Sized + Send + Sync {
    fn definition() -> &'static Definitions;
}

/// Trait for types that provide a `TypeDefinition` descriptor.
///
/// This is the new, unified way to get type metadata using parser types.
/// Types can implement both `Defined` and `DeclDefined` for compatibility.
pub trait DeclDefined: Sized + Send + Sync {
    fn type_definition() -> declare::TypeDefinition;
}

/// Blanket implementation: any type with `Defined` can provide `TypeDefinition`.
impl<T: Defined> DeclDefined for T {
    fn type_definition() -> declare::TypeDefinition {
        T::definition().into()
    }
}

// pub trait Error: Defined {}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("toml error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("toml error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("{tag} {name} already exists in {ns}")]
    NamespaceConflict {
        name: Ident,
        tag: &'static str,
        ns: Ident,
    },

    #[error("{name} is not found in {ns}")]
    NameNotFound { name: Ident, ns: Ident },

    #[error("config error: {0}")]
    Config(#[from] ::config::ConfigError),

    #[error("[{src}] {error}")]
    SourceFile { error: Box<Self>, src: String },

    #[error("'{ident}' is not contiguous with {desc}")]
    ContiguousError { ident: Ident, desc: String },

    #[error("{0}")]
    Validation(#[from] validator::ValidationError),

    #[error("{0}")]
    Validations(#[from] validator::ValidationErrors),

    #[error("{0}")]
    Manifests(#[from] kintsu_manifests::Error),

    #[error("{0}")]
    Fs(#[from] kintsu_fs::Error),

    #[error("{0}")]
    Parsing(#[from] kintsu_parser::Error),

    #[error("{0}")]
    Miette(miette::Error),

    #[error("{0}")]
    Client(#[from] kintsu_env_client::Error),
}

impl From<miette::Error> for Error {
    fn from(value: miette::Error) -> Self {
        Self::Miette(value)
    }
}

impl Error {
    pub fn with_source(
        self,
        src: String,
    ) -> Self {
        Self::SourceFile {
            error: Box::new(self),
            src,
        }
    }

    pub fn with_source_init(src: String) -> impl FnOnce(Self) -> Self {
        |err| err.with_source(src)
    }

    pub fn from_with_source_init<E: Into<Self>>(src: String) -> impl FnOnce(E) -> Self {
        |err| Self::with_source_init(src)(err.into())
    }
}

impl From<Error> for kintsu_errors::CompilerError {
    fn from(err: Error) -> Self {
        use kintsu_errors::{FilesystemError, InternalError, PackageError, TypeDefError};
        match err {
            Error::Io(e) => FilesystemError::io_error(e.to_string()).into(),
            Error::TomlDe(e) => PackageError::parse_error(e.to_string()).into(),
            Error::TomlSer(e) => PackageError::parse_error(e.to_string()).into(),
            Error::Json(e) => PackageError::manifest_error(e.to_string()).into(),
            Error::Yaml(e) => PackageError::manifest_error(e.to_string()).into(),
            Error::NamespaceConflict { name, tag, ns } => {
                TypeDefError::ident_conflict(ns.to_string(), tag, name.to_string()).into()
            },
            Error::NameNotFound { name, ns } => {
                kintsu_errors::ResolutionError::undefined_type(format!("{ns}::{name}")).into()
            },
            Error::Config(e) => PackageError::parse_error(e.to_string()).into(),
            Error::SourceFile { error, .. } => (*error).into(),
            Error::ContiguousError { ident, desc } => {
                InternalError::internal(format!("'{ident}' is not contiguous with {desc}")).into()
            },
            Error::Validation(e) => PackageError::manifest_error(e.to_string()).into(),
            Error::Validations(e) => PackageError::manifest_error(e.to_string()).into(),
            Error::Manifests(e) => e.into(),
            Error::Fs(e) => e.into(),
            Error::Parsing(e) => e.into(),
            Error::Miette(e) => InternalError::internal(e.to_string()).into(),
            Error::Client(e) => InternalError::internal(e.to_string()).into(),
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
// #[macro_export]
// macro_rules! submit {
//     ($item: tt) => {
//         inventory::submit! {
//             $item
//         }
//     };
// }

// pub fn definitions() -> Vec<&'static Definitions> {
//     let mut v = Vec::new();
//     for it in inventory::iter::<Definitions>() {
//         v.push(it);
//     }
//     v
// }

pub struct Ctx;

#[derive(Default)]
pub struct Data<T> {
    ph: PhantomData<T>,
}

#[cfg(test)]
mod test {

    use crate::{
        CompoundType, Definitions, Enum, Field, FieldOrRef, FieldsList, Meta, Named, Operation,
        Struct, Type, Typed, VariantKind, Version, map, ty,
    };

    const BASIC_STRUCT: &str = include_str!("../../samples/basic-struct.toml");
    const BASIC_OP: &str = include_str!("../../samples/basic-op.toml");
    const BASIC_ENUM: &str = include_str!("../../samples/basic-enum.toml");

    fn empty_meta() -> Meta<Option<crate::Ident>, Option<crate::Ident>, Option<Version>> {
        Meta::builder()
            .name(None)
            .namespace(None)
            .version(None)
            .build()
    }

    #[test]
    fn test_de_basic_struct() {
        let namespace = crate::Ident::new("abc.corp.namespace");
        let s: Definitions = toml::from_str(BASIC_STRUCT).unwrap();
        let s2 = Definitions::StructV1(Struct {
            meta: Meta {
                name: "some_struct".into(),
                description: None,
                namespace: namespace.clone(),
                version: 2_usize.into(),
            },
            fields: FieldsList::new(map!({
                a: Field::builder().ty(Type::I32).optional(false).meta(empty_meta()).build(),
                b: Field::builder().ty(Type::F32).optional(true).meta(empty_meta()).build(),
                c: Field::builder().ty(
                    ty!([[f32; 4]; 4])
                ).optional(false).meta(empty_meta()).build(),
                d: FieldOrRef::Ref{
                    to: "c".into()
                }
            })),
        });
        assert_eq!(s, s2)
    }

    #[test]
    fn test_de_basic_op() {
        let s: Definitions = toml::from_str(BASIC_OP).unwrap();
        let expect = Definitions::OperationV1(
            Operation::builder()
            .meta(Meta::builder().name("add".into())
            .version(1_usize.into())
            .namespace("abc.corp.namespace".into())
            .description("add a sequence of numbers together".into()).build() )
                .inputs(FieldsList::new(map!({
                    values: Field::builder().ty(<Vec<u32>>::ty()).optional(false).meta(empty_meta()).build()
                })))
                .outputs(FieldsList::new(map!({
                    value: Field::builder().ty(ty!(u32)).optional(false).meta(empty_meta()).build()
                })))
                .infallible(true)
                .build(),
        );
        assert_eq!(s, expect);
    }

    #[test]
    fn test_de_basic_enum() {
        let s: Definitions = toml::from_str(BASIC_ENUM).unwrap();
        let expect = Definitions::EnumV1(
            Enum::builder()
                .meta(
                    Meta::builder()
                        .name("some_enum".into())
                        .version(1_usize.into())
                        .namespace("abc.corp.namespace".into())
                        .description("some enum description".into())
                        .build(),
                )
                .variants(Named::new([
                    (
                        "A".into(),
                        VariantKind {
                            meta: Meta::builder()
                                .name("A".into())
                                .namespace(None)
                                .version(None)
                                .build(),
                            value: crate::StrOrInt::Int(1),
                        },
                    ),
                    (
                        "B".into(),
                        VariantKind {
                            meta: Meta::builder()
                                .description("Some b variant = 2".into())
                                .name("B".into())
                                .namespace(None)
                                .version(None)
                                .build(),
                            value: crate::StrOrInt::Int(2),
                        },
                    ),
                ]))
                .build(),
        );

        assert_eq!(s, expect);
    }

    #[test]
    fn ty_resolution() {
        let t = ty! {
            i32
        };

        assert_eq!(t, Type::I32);

        let t = <Option<i32>>::ty();

        assert_eq!(
            t,
            Type::CompoundType(CompoundType::Option {
                ty: Box::new(Type::I32)
            })
        );

        let t = ty! {
            [i32; 4]
        };

        assert_eq!(
            t,
            Type::CompoundType(CompoundType::SizedArray {
                size: 4,
                ty: Box::new(Type::I32)
            })
        );

        let t = <Vec<i32>>::ty();

        assert_eq!(
            t,
            Type::CompoundType(CompoundType::Array {
                ty: Box::new(Type::I32)
            })
        );
    }
}
