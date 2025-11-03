use std::{
    collections::BTreeMap,
    fmt::Display,
    io::Write,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use convert_case::Casing;

#[cfg(feature = "generate")]
use crate::generate::RustConfig;
use crate::{namespace::Namespace, trace_replace};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ident(String);

impl Ident {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self::from(s)
    }
}

impl Display for Ident {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<S: Into<String>> From<S> for Ident {
    fn from(value: S) -> Self {
        Self(value.into())
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(usize);

impl Version {
    pub const fn new(version: usize) -> Self {
        Self(version)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self(1)
    }
}

impl<S: Into<usize>> From<S> for Version {
    fn from(value: S) -> Self {
        Self(value.into())
    }
}

pub trait Contiguous<T> {
    fn is_contiguous(
        &self,
        parent: &Ident,
    ) -> crate::Result<T>;
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CompoundType {
    Option {
        #[serde(rename = "type")]
        ty: Box<Type>,
    },
    Array {
        #[serde(rename = "type")]
        ty: Box<Type>,
    },
    SizedArray {
        size: usize,
        #[serde(rename = "type")]
        ty: Box<Type>,
    },
    Enum {
        #[serde(rename = "ref")]
        to: Ident,
    },
    OneOf {
        #[serde(rename = "ref")]
        to: Ident,
    },
    Union {
        lhs: Box<Type>,
        rhs: Box<Type>,
    },
    Struct {
        #[serde(rename = "ref")]
        to: Ident,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    U8,
    U16,
    U32,
    U64,

    Usize,

    I8,
    I16,
    I32,
    I64,

    F32,
    F64,

    Bool,

    DateTime,

    Complex,

    String,

    Binary,

    Never,

    CompoundType(CompoundType),
}

impl Display for CompoundType {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Array { ty } => format!("[{ty}]"),
                Self::SizedArray { ty, size } => format!("[{ty}; {size}]"),
                Self::Option { ty } => format!("{ty} | never"),
                Self::Enum { to } => format!("{to}"),
                Self::Struct { to } => format!("{to}"),
                Self::OneOf { to } => format!("{to}"),
                Self::Union { .. } => todo!(),
            }
        )
    }
}

impl Display for Type {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::U8 => "u8".to_string(),
                Self::U16 => "u16".to_string(),
                Self::U32 => "u32".to_string(),
                Self::U64 => "u64".to_string(),
                Self::Usize => "usize".to_string(),
                Self::I8 => "i8".to_string(),
                Self::I16 => "i16".to_string(),
                Self::I32 => "i32".to_string(),
                Self::I64 => "i64".to_string(),
                Self::F32 => "f32".to_string(),
                Self::F64 => "f64".to_string(),
                Self::Bool => "bool".to_string(),
                Self::DateTime => "datetime".to_string(),
                Self::Complex => "complex".to_string(),
                Self::String => "string".to_string(),
                Self::Never => "never".to_string(),
                Self::Binary => "binary".to_string(),
                Self::CompoundType(ty) => ty.to_string(),
            }
        )
    }
}

impl Type {
    pub fn simplify(&self) -> Self {
        match self {
            Type::CompoundType(orig) => {
                match orig {
                    CompoundType::Option { ty } => {
                        match ty.as_ref() {
                            Type::Never => {
                                Type::CompoundType(CompoundType::Option {
                                    ty: Box::new(Self::Never),
                                })
                            },
                            Type::CompoundType(CompoundType::Option { ty }) => {
                                // flatten nested options and collapse to Never when inner is Array/SizedArray<Never>
                                let inner = ty.simplify();
                                match &inner {
                                    Type::Never => {
                                        trace_replace!(
                                            self;
                                            Self::CompoundType(CompoundType::Option {
                                                ty: Box::new(Self::Never),
                                            })
                                        )
                                    },
                                    Type::CompoundType(CompoundType::Option { ty }) => {
                                        trace_replace!(
                                            self;
                                            Self::CompoundType(CompoundType::Option {
                                                ty: Box::new(ty.simplify()),
                                            })
                                        )
                                    },
                                    Type::CompoundType(CompoundType::Array { ty }) => {
                                        trace_replace!(
                                            self;
                                            if let Type::Never = ty.as_ref() {
                                                Self::Never
                                            } else {
                                                Self::CompoundType(CompoundType::Option {
                                                    ty: Box::new(inner),
                                                })
                                            }
                                        )
                                    },
                                    Type::CompoundType(CompoundType::SizedArray { ty, .. }) => {
                                        trace_replace!(
                                            self;
                                            if let Type::Never = ty.as_ref() {
                                                Self::Never
                                            } else {
                                                Self::CompoundType(CompoundType::Option {
                                                    ty: Box::new(inner),
                                                })
                                            }
                                        )
                                    },
                                    _ => {
                                        Self::CompoundType(CompoundType::Option {
                                            ty: Box::new(inner),
                                        })
                                    },
                                }
                            },
                            rest => {
                                trace_replace!(
                                    self;
                                    Self::CompoundType(CompoundType::Option {
                                        ty: Box::new(rest.simplify()),
                                    })
                                )
                            },
                        }
                    },
                    CompoundType::Array { ty } => {
                        match ty.as_ref() {
                            Self::U8 => {
                                trace_replace!(
                                    self; Self::Binary
                                )
                            },
                            _ => self.clone(),
                        }
                    },
                    rest => Self::CompoundType(rest.clone()),
                }
            },
            _ => self.clone(),
        }
    }
}

pub trait Typed {
    fn ty() -> Type;
}

macro_rules! transparent {
    ($ty: ty) => {
        impl Typed for $ty {
            fn ty() -> Type {
                paste::paste! {
                    Type::[<$ty:camel>]
                }
            }
        }
    };
    ($(
        $ty: ty
    ), + $(,)?) => {
        $(
            transparent!($ty);
        )*
    };
}

transparent! {
    u8,
    u16,
    u32,
    u64,

    usize,

    i8,
    i16,
    i32,
    i64,

    f32,
    f64,

    bool,

    String,
}

impl<T: Typed> Typed for Vec<T> {
    fn ty() -> Type {
        Type::CompoundType(CompoundType::Array {
            ty: Box::new(T::ty()),
        })
    }
}

impl<T: Typed> Typed for Option<T> {
    fn ty() -> Type {
        if std::any::type_name::<T>() == "()" {
            Type::Never
        } else {
            Type::CompoundType(CompoundType::Option {
                ty: Box::new(T::ty()),
            })
        }
    }
}

impl<T: Typed, const S: usize> Typed for [T; S] {
    fn ty() -> Type {
        Type::CompoundType(CompoundType::SizedArray {
            ty: Box::new(T::ty()),
            size: S,
        })
    }
}

#[cfg(feature = "chrono")]
impl Typed for chrono::DateTime<chrono::Utc> {
    fn ty() -> Type {
        Type::DateTime
    }
}

#[cfg(feature = "time")]
impl Typed for time::UtcDateTime {
    fn ty() -> Type {
        Type::DateTime
    }
}

impl Typed for () {
    fn ty() -> Type {
        Type::Never
    }
}

impl From<Field<Ident>> for Field<Option<Ident>> {
    fn from(value: Field<Ident>) -> Self {
        Self {
            meta: Meta {
                name: Some(value.meta.name),
                namespace: Some(value.meta.namespace),
                description: value.meta.description,
                version: value.meta.version,
            },
            ty: value.ty,
            optional: value.optional,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, bon::Builder, Clone)]
pub struct Field<Ns> {
    #[serde(flatten)]
    pub meta: Meta<Ns, Ns, Option<Version>>,

    #[serde(rename = "type")]
    pub ty: Type,

    #[serde(default = "crate::utils::default_no")]
    pub optional: bool,
}

impl Type {
    #[allow(clippy::only_used_in_recursion)]
    #[cfg(feature = "generate")]
    pub fn ty(
        &self,
        opts: &RustConfig,
    ) -> proc_macro2::TokenStream {
        match self {
            Type::Complex => todo!(),

            Type::Bool => quote::quote!(bool),

            Type::I8 => quote::quote!(i8),
            Type::I16 => quote::quote!(i8),
            Type::I32 => quote::quote!(i32),
            Type::I64 => quote::quote!(i64),

            Type::U8 => quote::quote!(u8),
            Type::U16 => quote::quote!(u8),
            Type::U32 => quote::quote!(u32),
            Type::U64 => quote::quote!(u64),
            Type::Usize => quote::quote!(usize),

            Type::F32 => quote::quote!(f32),
            Type::F64 => quote::quote!(f64),

            Type::Binary => quote::quote!(Vec<u8>),
            Type::String => quote::quote!(String),

            Type::DateTime => {
                use crate::generate::DateTimeLibrary;

                match opts.time {
                    DateTimeLibrary::Chrono => {
                        cfg_if::cfg_if! {
                            if #[cfg(feature = "chrono")] {
                                quote::quote!(chrono::DateTime::<chrono::Utc>)
                            } else {
                                panic!("feature 'chrono' must be enabled to emit chrono::DateTime")
                            }
                        }
                    },
                    DateTimeLibrary::Time => {
                        cfg_if::cfg_if! {
                            if #[cfg(feature = "time")] {
                                quote::quote!(time::UtcDateTime)
                            } else {
                                panic!("feature 'time' must be enabled to emit time::UtcDateTime")
                            }
                        }
                    },
                }
            },
            Type::Never => quote::quote!(),
            Type::CompoundType(outer_ty) => {
                match outer_ty {
                    CompoundType::Enum { to } => {
                        let as_rs_ref = super::generate::rust::ident(to.to_string());
                        quote::quote!(#as_rs_ref)
                    },
                    CompoundType::OneOf { to } => {
                        let as_rs_ref = super::generate::rust::ident(to.to_string());
                        quote::quote!(#as_rs_ref)
                    },
                    CompoundType::Struct { to } => {
                        let as_rs_ref = super::generate::rust::ident(to.to_string());
                        quote::quote!(#as_rs_ref)
                    },
                    CompoundType::Array { ty } => {
                        let inner = ty.ty(opts);
                        quote::quote!(
                            Vec<#inner>
                        )
                    },
                    CompoundType::Option { ty } => {
                        let inner = ty.ty(opts);
                        quote::quote!(
                            Option<#inner>
                        )
                    },
                    CompoundType::SizedArray { size, ty } => {
                        let inner = ty.ty(opts);
                        let size = crate::generate::rust::lit(format!("{size}"));
                        quote::quote!(
                            [#inner; #size]
                        )
                    },
                    CompoundType::Union { .. } => todo!(),
                }
            },
        }
    }

    pub fn rust_attrs(&self) -> proc_macro2::TokenStream {
        match self {
            Self::CompoundType(CompoundType::Enum { .. }) => quote::quote!(#[fields(enm)]),
            Self::CompoundType(CompoundType::OneOf { .. }) => quote::quote!(#[fields(one_of)]),
            _ => quote::quote! {},
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Reffable<T> {
    Value(T),
    Ref {
        #[serde(rename = "ref")]
        to: Ident,
    },
}

pub type FieldOrRef = Reffable<Field<Option<Ident>>>;
pub type EnumOrRef = Reffable<Enum>;

impl<T> Reffable<T> {
    pub fn unwrap_value(&self) -> &T {
        match self {
            Self::Value(v) => v,
            _ => panic!("expected value, got ref"),
        }
    }
}
impl<T> From<T> for Reffable<T> {
    fn from(value: T) -> Self {
        Self::Value(value)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Named<T>(BTreeMap<Ident, T>);

pub type FieldsList = Named<FieldOrRef>;

impl<T> Named<T> {
    pub fn new<M: Into<BTreeMap<Ident, T>>>(map: M) -> Self {
        Self(map.into())
    }
}

impl<T> DerefMut for Named<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Deref for Named<T> {
    type Target = BTreeMap<Ident, T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged, rename_all = "snake_case")]
pub enum StrOrInt {
    String(String),
    Int(usize),
}

#[derive(PartialEq, Debug)]
pub enum EnumValueType {
    String,
    Int,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct VariantKind {
    #[serde(flatten)]
    pub meta: Meta<Ident, Option<Ident>, Option<Version>>,
    pub value: StrOrInt,
}

impl Named<VariantKind> {
    fn ty_of(
        &self,
        it: &VariantKind,
    ) -> EnumValueType {
        match &it.value {
            StrOrInt::Int(_) => EnumValueType::Int,
            StrOrInt::String(_) => EnumValueType::String,
        }
    }
}

impl Contiguous<EnumValueType> for Named<VariantKind> {
    fn is_contiguous(
        &self,
        parent: &Ident,
    ) -> crate::Result<EnumValueType> {
        let outer_ty = self.ty_of(self.0.values().next().unwrap());
        for (ident, variant) in &self.0 {
            let ty = self.ty_of(variant);
            if outer_ty != ty {
                return Err(crate::Error::ContiguousError {
                    ident: format!("{parent}::{ident}").into(),
                    desc: format!("expected {outer_ty:#?}, received {ty:#?}"),
                });
            }
        }
        Ok(outer_ty)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, bon::Builder, Clone)]
pub struct Meta<IdentTy, NsTy, Version> {
    pub name: IdentTy,
    pub namespace: NsTy,

    #[serde(default)]
    pub description: Option<String>,

    pub version: Version,
}

#[cfg(feature = "generate")]
impl<IdentTy, NsTy, Version> Meta<IdentTy, NsTy, Version> {
    pub fn doc_comment(&self) -> proc_macro2::TokenStream {
        crate::generate::rust::comment(&self.description)
    }

    pub fn op_comment(&self) -> proc_macro2::TokenStream {
        match &self.description {
            Some(desc) => {
                quote::quote!(
                    #[fields(
                        describe(text = #desc)
                    )]
                )
            },
            None => quote::quote!(),
        }
    }
}

#[cfg(feature = "generate")]
impl<IdentTy, Ns> Meta<IdentTy, Ns, Version> {
    pub fn version(&self) -> proc_macro2::TokenStream {
        crate::generate::rust::lit(self.version.0.to_string())
    }
}

#[cfg(feature = "generate")]
impl<IdentTy: ToString, Ns, Version> Meta<IdentTy, Ns, Version> {
    pub fn ident_as_pascal(&self) -> syn::Ident {
        crate::generate::rust::ident(
            self.name
                .to_string()
                .to_case(convert_case::Case::Pascal),
        )
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, bon::Builder, Clone)]
pub struct Struct {
    #[serde(flatten)]
    pub meta: Meta<Ident, Ident, Version>,
    pub fields: FieldsList,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, bon::Builder, Clone)]
pub struct Operation {
    #[serde(flatten)]
    pub meta: Meta<Ident, Ident, Version>,

    #[serde(default)]
    pub infallible: bool,

    #[serde(default)]
    pub error: Option<Ident>,

    pub inputs: FieldsList,
    pub outputs: FieldsList,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, bon::Builder, Clone)]
pub struct Enum {
    #[serde(flatten)]
    pub meta: Meta<Ident, Ident, Version>,

    pub variants: Named<VariantKind>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, bon::Builder, Clone)]
pub struct OneOfVariant {
    pub name: Ident,
    #[serde(default)]
    pub description: Option<String>,
    pub ty: Type,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, bon::Builder, Clone)]
pub struct OneOf {
    #[serde(flatten)]
    pub meta: Meta<Ident, Ident, Version>,

    pub variants: Named<OneOfVariant>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, bon::Builder, Clone)]
pub struct ErrorTy {
    #[serde(flatten)]
    pub meta: Meta<Ident, Ident, Version>,

    pub variants: Named<OneOfVariant>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum Definitions {
    #[serde(rename = "field@v1", alias = "field")]
    FieldV1(Field<Ident>),

    #[serde(rename = "struct@v1", alias = "struct")]
    StructV1(Struct),

    #[serde(rename = "operation@v1", alias = "operation")]
    OperationV1(Operation),

    #[serde(rename = "enum@v1", alias = "enum")]
    EnumV1(Enum),

    #[serde(rename = "one_of@v1", alias = "one_of")]
    OneOfV1(OneOf),

    #[serde(rename = "error@v1", alias = "error")]
    ErrorV1(ErrorTy),

    #[serde(rename = "namespace@v1", alias = "namespace")]
    NamespaceV1(Namespace),
}

impl Definitions {
    pub fn name(&self) -> &Ident {
        match self {
            Self::FieldV1(v) => &v.meta.name,
            Self::StructV1(v) => &v.meta.name,
            Self::OperationV1(v) => &v.meta.name,
            Self::EnumV1(v) => &v.meta.name,
            Self::OneOfV1(v) => &v.meta.name,
            Self::ErrorV1(v) => &v.meta.name,
            Self::NamespaceV1(ns) => &ns.name,
        }
    }

    pub fn namespace(&self) -> &Ident {
        match self {
            Self::FieldV1(v) => &v.meta.namespace,
            Self::StructV1(v) => &v.meta.namespace,
            Self::OperationV1(v) => &v.meta.namespace,
            Self::EnumV1(v) => &v.meta.namespace,
            Self::OneOfV1(v) => &v.meta.namespace,
            Self::ErrorV1(v) => &v.meta.namespace,
            Self::NamespaceV1(ns) => &ns.name,
        }
    }

    pub fn load_data(
        data: Vec<u8>,
        ext: &str,
    ) -> crate::Result<Self> {
        Ok(match ext {
            "yaml" | "yml" => serde_yaml::from_slice(&data)?,
            "json" => serde_json::from_slice(&data)?,
            "toml" => toml::from_slice(&data)?,
            ext => unimplemented!("{ext} is not implemented"),
        })
    }

    pub fn write_data<W: Write>(
        &self,
        w: &mut W,
        ext: &str,
    ) -> crate::Result<()> {
        let _: () = match ext {
            "yaml" | "yml" => serde_yaml::to_writer(w, self)?,
            "json" => serde_json::to_writer(w, self)?,
            "toml" => {
                let buf = toml::to_string(self)?.as_bytes().to_vec();
                let wrote = w.write(&buf)?;
                if wrote != buf.len() {
                    return Err(crate::Error::Io(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "failed to write toml file",
                    )));
                }
                w.flush()?;
            },
            ext => unimplemented!("{ext} is not implemented"),
        };
        Ok(())
    }

    pub fn load_from_path(path: PathBuf) -> crate::Result<Self> {
        Self::load_data(
            std::fs::read(&path)?,
            path.extension().unwrap().to_str().unwrap(),
        )
        .map_err(crate::Error::from_with_source_init(
            path.display().to_string(),
        ))
    }

    pub fn export(
        &self,
        path: PathBuf,
    ) -> crate::Result<()> {
        self.write_data(
            &mut std::fs::File::create(&path)?,
            path.extension().unwrap().to_str().unwrap(),
        )
    }
}

pub trait DefinedError: Into<ErrorTy> {}

#[cfg(test)]
mod test {
    use std::marker::PhantomData;

    use crate::Typed;

    use super::{CompoundType, Type};
    use test_case::test_case;

    fn opt(inner: Type) -> Type {
        Type::CompoundType(CompoundType::Option {
            ty: Box::new(inner),
        })
    }

    fn array(inner: Type) -> Type {
        Type::CompoundType(CompoundType::Array {
            ty: Box::new(inner),
        })
    }

    fn sized_array(
        inner: Type,
        size: usize,
    ) -> Type {
        Type::CompoundType(CompoundType::SizedArray {
            ty: Box::new(inner),
            size,
        })
    }

    fn nest_opts(
        mut inner: Type,
        count: usize,
    ) -> Type {
        for _ in 0..count {
            inner = opt(inner);
        }
        inner
    }

    fn id(s: &str) -> super::Ident {
        super::Ident::from(s.to_string())
    }

    #[test_case(Type::U8, Type::U8; "u8 stays u8")]
    #[test_case(Type::String, Type::String; "string stays string")]
    #[test_case(opt(Type::U8), opt(Type::U8); "single opt u8")]
    #[test_case(opt(Type::Never), opt(Type::Never); "single opt never")]
    #[test_case(opt(opt(Type::U8)), opt(Type::U8); "double opt u8 collapses")]
    #[test_case(opt(opt(opt(Type::U8))), opt(Type::U8); "triple opt u8 collapses")]
    #[test_case(nest_opts(Type::U16, 5), opt(Type::U16); "5 opts u16 collapse")]
    #[test_case(nest_opts(Type::F64, 10), opt(Type::F64); "10 opts f64 collapse")]
    #[test_case(nest_opts(Type::Never, 1), opt(Type::Never); "1 opt never")]
    #[test_case(nest_opts(Type::Never, 2), opt(Type::Never); "2 opts never")]
    #[test_case(nest_opts(Type::Never, 3), opt(Type::Never); "3 opts never")]
    #[test_case(nest_opts(Type::Never, 4), opt(Type::Never); "4 opts never")]
    #[test_case(nest_opts(Type::Never, 5), opt(Type::Never); "5 opts never")]
    #[test_case(nest_opts(Type::Never, 6), opt(Type::Never); "6 opts never")]
    #[test_case(nest_opts(Type::Never, 7), opt(Type::Never); "7 opts never")]
    #[test_case(nest_opts(Type::Never, 8), opt(Type::Never); "8 opts never")]
    #[test_case(nest_opts(Type::Never, 9), opt(Type::Never); "9 opts never")]
    #[test_case(nest_opts(Type::Never, 10), opt(Type::Never); "10 opts never")]
    #[test_case(array(Type::U8), Type::Binary; "vec u8 -> binary")]
    #[test_case(array(opt(Type::U8)), array(opt(Type::U8)); "array of opt<u8> unchanged")]
    #[test_case(opt(array(Type::U8)), opt(Type::Binary); "opt<array<u8>> to opt<binary>")]
    #[test_case(sized_array(opt(Type::U8), 3), sized_array(opt(Type::U8), 3); "sized array of opt<u8> unchanged")]
    #[test_case(opt(sized_array(opt(Type::U8), 3)), opt(sized_array(opt(Type::U8), 3)); "opt<sized array<opt<u8>>> unchanged")]
    #[test_case(opt(opt(array(Type::Never))), Type::Never; "opt<opt<array<never>>> into never")]
    fn ty_simplify_case(
        input: Type,
        expected: Type,
    ) {
        assert_eq!(input.simplify(), expected);
    }

    #[test_case(Type::U8, "u8"; "u8")]
    #[test_case(Type::U16, "u16"; "u16")]
    #[test_case(Type::U32, "u32"; "u32")]
    #[test_case(Type::U64, "u64"; "u64")]
    #[test_case(Type::Usize, "usize"; "usize")]
    #[test_case(Type::I8, "i8"; "i8")]
    #[test_case(Type::I16, "i16"; "i16")]
    #[test_case(Type::I32, "i32"; "i32")]
    #[test_case(Type::I64, "i64"; "i64")]
    #[test_case(Type::F32, "f32"; "f32")]
    #[test_case(Type::F64, "f64"; "f64")]
    #[test_case(Type::Bool, "bool"; "bool")]
    #[test_case(Type::String, "string"; "string")]
    #[test_case(Type::DateTime, "datetime"; "datetime")]
    #[test_case(Type::Complex, "complex"; "complex")]
    #[test_case(Type::Never, "never"; "never")]
    #[test_case(Type::Binary, "binary"; "binary")]
    #[test_case(array(Type::U8), "[u8]"; "array u8")]
    #[test_case(array(Type::String), "[string]"; "array string")]
    #[test_case(sized_array(Type::I16, 4), "[i16; 4]"; "sized array i16")]
    #[test_case(opt(Type::U32), "u32 | never"; "opt u32")]
    #[test_case(opt(opt(Type::U8)), "u8 | never | never"; "nested opt u8")]
    #[test_case(opt(opt(opt(Type::F64))), "f64 | never | never | never"; "triple opt f64")]
    #[test_case(array(opt(Type::U16)), "[u16 | never]"; "array of opt u16")]
    #[test_case(opt(array(Type::U8)), "[u8] | never"; "opt array u8")]
    #[test_case(sized_array(opt(Type::String), 3), "[string | never; 3]"; "sized array of opt string")]
    #[test_case(opt(sized_array(Type::I32, 8)), "[i32; 8] | never"; "opt sized array i32")]
    #[test_case(opt(array(opt(Type::I32))), "[i32 | never] | never"; "opt array opt i32")]
    #[test_case(Type::CompoundType(CompoundType::Struct { to: id("User") }), "User"; "struct ref")]
    #[test_case(Type::CompoundType(CompoundType::Enum { to: id("Color") }), "Color"; "enum ref")]
    #[test_case(Type::CompoundType(CompoundType::OneOf { to: id("Payload") }), "Payload"; "one_of ref")]
    fn display_case(
        input: Type,
        expected: &str,
    ) {
        assert_eq!(input.to_string(), expected);
    }

    #[test_case(array(Type::U8), "binary"; "vec u8 simplifies to binary")]
    #[test_case(opt(opt(array(Type::Never))), "never"; "opt opt array never simplifies to never")]
    fn simplify_then_display_case(
        input: Type,
        expected: &str,
    ) {
        assert_eq!(input.simplify().to_string(), expected);
    }

    #[test_case(
        PhantomData::<i16>, Type::I16; "i16 into i16"
        )]
    #[test_case(
        PhantomData::<i32>, Type::I32; "i32 into i32"
        )]
    #[test_case(
        PhantomData::<i64>, Type::I64; "i64 into i64"
        )]
    #[test_case(
        PhantomData::<u16>, Type::U16; "u16 into u16"
        )]
    #[test_case(
        PhantomData::<u32>, Type::U32; "u32 into u32"
        )]
    #[test_case(
        PhantomData::<u64>, Type::U64; "u64 into u64"
        )]
    #[test_case(
        PhantomData::<usize>, Type::Usize; "usize into usize"
        )]
    #[test_case(
        PhantomData::<f32>, Type::F32; "f32 into f32"
        )]
    #[test_case(
        PhantomData::<f64>, Type::F64; "f64 into f64"
        )]
    #[test_case(
        PhantomData::<bool>, Type::Bool; "bool into bool"
        )]
    #[test_case(
        PhantomData::<String>, Type::String; "string into string"
        )]
    #[test_case(
        PhantomData::<()>, Type::Never; "unit into never"
        )]
    #[test_case(
        PhantomData::<Option<u8>>, opt(Type::U8); "option<u8> into option<u8>"
        )]
    #[test_case(
        PhantomData::<Option<()>>, Type::Never; "option<()> into never"
        )]
    #[test_case(
        PhantomData::<Option<Option<i32>>>, opt(opt(Type::I32)); "option<option<i32>> nested"
        )]
    #[test_case(
        PhantomData::<Option<Option<Option<u8>>>>, opt(opt(opt(Type::U8))); "triple option<u8>"
        )]
    #[test_case(
        PhantomData::<Vec<u8>>, array(Type::U8); "vec<u8> into array<u8>"
        )]
    #[test_case(
        PhantomData::<Vec<Vec<u16>>>, array(array(Type::U16)); "vec<vec<u16>> nested arrays"
        )]
    #[test_case(
        PhantomData::<Vec<Option<u8>>>, array(opt(Type::U8)); "vec<option<u8>>"
        )]
    #[test_case(
        PhantomData::<Vec<()>>, array(Type::Never); "vec<()> -> array<never>"
        )]
    #[test_case(
        PhantomData::<Option<Vec<u8>>>, opt(array(Type::U8)); "option<vec<u8>>"
        )]
    #[test_case(
        PhantomData::<[u8; 16]>, sized_array(Type::U8, 16); "[u8; 16] sized array"
        )]
    #[test_case(
        PhantomData::<[Option<i32>; 4]>, sized_array(opt(Type::I32), 4); "[option<i32>; 4] sized array"
        )]
    #[test_case(
        PhantomData::<Option<[u8; 4]>>, opt(sized_array(Type::U8, 4)); "option<[u8; 4]>"
        )]
    #[test_case(
        PhantomData::<[[u16; 2]; 3]>, sized_array(sized_array(Type::U16, 2), 3); "[[u16; 2]; 3] nested sized arrays"
        )]
    #[test_case(
        PhantomData::<Vec<[u8; 4]>>, array(sized_array(Type::U8, 4)); "vec<[u8; 4]>"
        )]
    #[test_case(
        PhantomData::<[Vec<u8>; 2]>, sized_array(array(Type::U8), 2); "[vec<u8>; 2]"
        )]
    #[test_case(
        PhantomData::<Option<Vec<[Option<u16>; 2]>>>, opt(array(sized_array(opt(Type::U16), 2))); "option<vec<[option<u16>; 2]>>"
        )]
    #[test_case(
        PhantomData::<u8>, Type::U8; "u8 into u8"

    )]
    fn from_rust_type<T: Typed>(
        _: PhantomData<T>,
        expect: Type,
    ) {
        assert_eq!(T::ty(), expect)
    }
}
