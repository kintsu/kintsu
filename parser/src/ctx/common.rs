use std::{ops::Deref, path::PathBuf, sync::Arc};

use crate::{
    SpannedToken,
    ast::{
        comment::CommentStream,
        items::{
            EnumDef, ErrorDef, NamespaceDef, OneOfDef, OperationDef, StructDef, TypeDef, UseDef,
        },
        meta::{ErrorMeta, VersionMeta},
    },
    defs::{Span, Spanned, Spans},
};

pub enum NamespaceChild {
    Namespace(Box<super::NamespaceCtx>),
    OneOf(OneOfDef),
    Enum(EnumDef),
    Struct(StructDef),
    Type(TypeDef),
    Error(ErrorDef),
    Operation(OperationDef),
}

impl NamespaceChild {
    pub fn type_name(&self) -> String {
        match self {
            NamespaceChild::Namespace(_) => "namespace".to_string(),
            NamespaceChild::OneOf(_) => "oneof".to_string(),
            NamespaceChild::Enum(_) => "enum".to_string(),
            NamespaceChild::Struct(_) => "struct".to_string(),
            NamespaceChild::Type(t) => t.def.ty.type_name(),
            NamespaceChild::Error(_) => "error".to_string(),
            NamespaceChild::Operation(_) => "operation".to_string(),
        }
    }
}

#[derive(Clone)]
pub enum Definition {
    Struct(Arc<StructDef>),
    Enum(Arc<EnumDef>),
    OneOf(Arc<OneOfDef>),
    Error(Arc<ErrorDef>),
    TypeAlias(Arc<TypeDef>),
    Operation(Arc<OperationDef>),
}

#[derive(Clone)]
pub struct ResolvedType {
    pub kind: Definition,
    pub qualified_path: super::paths::NamedItemContext,
}

#[derive(Clone)]
pub struct FromNamedSource<T> {
    pub source: PathBuf,
    pub value: T,
}

pub trait WithSource: Sized + Spans {
    fn with_source(
        self,
        source: PathBuf,
    ) -> FromNamedSource<Self> {
        FromNamedSource {
            source,
            value: self,
        }
    }

    fn with_source_and_span(
        self,
        source: PathBuf,
        span: Span,
    ) -> SourceSpanned<Self> {
        FromNamedSource {
            source,
            value: self.with_span(span),
        }
    }
}

impl<T: Sized + Spans> WithSource for T {}

pub type NamedCommentStream = FromNamedSource<CommentStream>;
pub type NamedErrorMeta = FromNamedSource<ErrorMeta>;
pub type NamedVersionMeta = FromNamedSource<VersionMeta>;
pub type NamedNamespaceDef = FromNamedSource<NamespaceDef>;
pub type NamedUseDef = FromNamedSource<UseDef>;
pub type NamedNamespaceChild = FromNamedSource<NamespaceChild>;
pub type IdentMap<T> = std::collections::BTreeMap<SpannedToken![ident], T>;

pub type SourceSpanned<T> = FromNamedSource<Spanned<T>>;

impl NamedVersionMeta {
    pub fn version_value(&self) -> i32 {
        *self.value.value.value.borrow_i32()
    }

    pub fn version_spanned(&self) -> crate::defs::Spanned<u32> {
        let val = *self.value.value.value.borrow_i32();
        let span = self.value.span();
        crate::defs::Spanned::new(span.start, span.end, val as u32)
    }
}

impl NamedErrorMeta {
    pub fn error_name(&self) -> &crate::ast::ty::PathOrIdent {
        &self.value.value.value.value
    }

    pub fn error_name_spanned(&self) -> crate::defs::Spanned<crate::ast::ty::PathOrIdent> {
        let name = self.error_name().clone();
        let span = self.value.span();
        crate::defs::Spanned::new(span.start, span.end, name)
    }
}

impl<T> Spanned<FromNamedSource<T>> {
    pub fn into_source_spanned(self) -> SourceSpanned<T> {
        SourceSpanned {
            source: self.value.source,
            value: Spanned {
                span: self.span,
                value: self.value.value,
            },
        }
    }
}

impl<T> Deref for SourceSpanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value.value
    }
}

impl<T> AsRef<Span> for SourceSpanned<T> {
    fn as_ref(&self) -> &Span {
        &self.value.span
    }
}

impl<T> SourceSpanned<T> {
    pub fn span(&self) -> &Span {
        self.as_ref()
    }

    pub fn source(&self) -> &PathBuf {
        &self.source
    }

    pub fn into_spanned(self) -> Spanned<T> {
        self.value
    }

    pub fn spanned(&self) -> &Spanned<T> {
        &self.value
    }
}
