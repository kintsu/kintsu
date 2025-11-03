use crate::{
    Token,
    ast::{self, comment::CommentStream, meta::ItemMeta},
    bail_unchecked,
    defs::{Span, Spanned},
    tokens::{ImplDiagnostic, LexingError, Parse, Peek, SemiToken, ToTokens, straight_through},
};

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum CommentOrMeta {
    Comments(CommentStream),
    Meta(Spanned<ItemMeta>),
}

impl Parse for CommentOrMeta {
    #[tracing::instrument(skip(stream))]
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, LexingError> {
        Ok(if stream.peek::<Token![#]>() {
            tracing::debug!("parse meta");
            Self::Meta(stream.parse()?)
        } else {
            tracing::debug!("parse comments");
            Self::Comments(CommentStream::parse(stream)?)
        })
    }
}

impl Peek for CommentOrMeta {
    fn peek(stream: &crate::tokens::TokenStream) -> bool {
        let mut fork = stream.fork();

        if stream.peek::<Token![#]>() {
            let meta: Spanned<ItemMeta> = bail_unchecked!(fork.parse(); false);
            tracing::trace!(
                "CommentOrMeta::peek parsed meta with {} items",
                meta.value.meta.len()
            );
            !meta.value.meta.is_empty()
        } else {
            let strm = bail_unchecked!(CommentStream::parse(&mut fork); false);
            !strm.comments.is_empty()
        }
    }
}

impl ToTokens for CommentOrMeta {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            Self::Comments(c) => c.write(tt),
            Self::Meta(m) => m.write(tt),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Item<T: Parse> {
    pub meta: Vec<Spanned<CommentOrMeta>>,
    pub def: Spanned<T>,
    // ;
    pub end: Spanned<SemiToken>,
}

impl<T: Parse> Item<T> {
    pub fn def_span(&self) -> &Span {
        &self.def.span
    }

    pub fn meta(&self) -> Vec<&Spanned<ItemMeta>> {
        let mut meta = vec![];
        for it in &self.meta {
            if let CommentOrMeta::Meta(m) = &it.value {
                meta.push(m)
            }
        }
        meta
    }

    pub fn comments(&self) -> Vec<&CommentStream> {
        let mut cmt = vec![];
        for it in &self.meta {
            if let CommentOrMeta::Comments(m) = &it.value {
                cmt.push(m)
            }
        }
        cmt
    }
}

impl<T: Parse> Parse for Item<T> {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        Ok(Self {
            meta: Vec::parse(stream)?,
            def: stream.parse()?,
            end: stream.parse()?,
        })
    }
}

straight_through! {
    Item<T> {
        meta, def, end
    }
}

pub type NamespaceDef = Item<super::namespace::Namespace>;
pub type SpannedNamespaceDef = Item<super::namespace::SpannedNamespace>;
pub type UseDef = Item<super::import::Use>;
pub type OneOfDef = Item<super::one_of::OneOf>;
pub type EnumDef = Item<super::enm::Enum>;
pub type StructDef = Item<super::strct::Struct>;
pub type TypeDef = Item<super::ty_def::NamedType>;
pub type ErrorDef = Item<super::err::ErrorType>;
pub type OperationDef = Item<super::op::Operation>;

impl UseDef {
    /// get the root identifier of the use statement
    pub fn root_ident(&self) -> &str {
        self.def.path.value.root_ident()
    }

    /// check if this is a single-segment import
    pub fn is_single_segment(&self) -> bool {
        self.def.path.value.is_single_segment()
    }

    /// check if this has nested items
    pub fn has_nested_items(&self) -> bool {
        self.def.path.value.has_nested_items()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub enum Items {
    Use(UseDef),
    OneOf(OneOfDef),
    Enum(EnumDef),
    Struct(StructDef),
    Type(TypeDef),
    Error(ErrorDef),
    Operation(OperationDef),
    Namespace(NamespaceDef),
    SpannedNamespace(SpannedNamespaceDef),
}

impl Parse for Items {
    #[tracing::instrument(skip(stream))]
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        tracing::trace!("try parse meta");
        let meta = Vec::parse(stream)?;
        tracing::trace!("done parsing meta");

        Ok(if stream.peek::<ast::namespace::Namespace>() {
            Self::Namespace(NamespaceDef {
                meta,
                def: stream.parse()?,
                end: stream.parse()?,
            })
        } else if stream.peek::<ast::import::Use>() {
            Self::Use(UseDef {
                meta,
                def: stream.parse()?,
                end: stream.parse()?,
            })
        } else if stream.peek::<ast::one_of::OneOf>() {
            Self::OneOf(OneOfDef {
                meta,
                def: stream.parse()?,
                end: stream.parse()?,
            })
        } else if stream.peek::<ast::err::ErrorType>() {
            Self::Error(ErrorDef {
                meta,
                def: stream.parse()?,
                end: stream.parse()?,
            })
        } else if stream.peek::<ast::enm::Enum>() {
            Self::Enum(EnumDef {
                meta,
                def: stream.parse()?,
                end: stream.parse()?,
            })
        } else if stream.peek::<ast::strct::Struct>() {
            Self::Struct(StructDef {
                meta,
                def: stream.parse()?,
                end: stream.parse()?,
            })
        } else if stream.peek::<ast::ty_def::NamedType>() {
            Self::Type(TypeDef {
                meta,
                def: stream.parse()?,
                end: stream.parse()?,
            })
        } else if stream.peek::<ast::op::Operation>() {
            Self::Operation(OperationDef {
                meta,
                def: stream.parse()?,
                end: stream.parse()?,
            })
        } else if stream.peek::<ast::namespace::SpannedNamespace>() {
            Self::SpannedNamespace(SpannedNamespaceDef {
                meta,
                def: stream.parse()?,
                end: stream.parse()?,
            })
        } else {
            let expect = vec![
                <Token![namespace]>::fmt(),
                <Token![use]>::fmt(),
                <Token![oneof]>::fmt(),
                <Token![enum]>::fmt(),
                <Token![struct]>::fmt(),
                <Token![error]>::fmt(),
                <Token![type]>::fmt(),
                <Token![operation]>::fmt(),
            ];
            return Err(if let Some(next) = stream.next() {
                LexingError::expected_oneof(expect, next.value)
            } else {
                LexingError::empty_oneof(expect)
            });
        })
    }
}

impl Peek for Items {
    fn peek(stream: &crate::tokens::TokenStream) -> bool {
        tracing::trace!("Items::peek called");
        let mut fork = stream.fork();
        let meta_result = Vec::<Spanned<CommentOrMeta>>::parse(&mut fork);
        tracing::trace!(
            "Items::peek meta parse result: {:?}",
            meta_result.as_ref().map(|v| v.len())
        );

        if let Some(token) = fork.next() {
            tracing::trace!("Items::peek next token: {:?}", token);
            let token = &token.value;
            <Token![namespace]>::is(token)
                || <Token![use]>::is(token)
                || <Token![oneof]>::is(token)
                || <Token![enum]>::is(token)
                || <Token![struct]>::is(token)
                || <Token![error]>::is(token)
                || <Token![type]>::is(token)
                || <Token![operation]>::is(token)
        } else {
            tracing::trace!("Items::peek no token found, returning false");
            false
        }
    }
}

impl ToTokens for Items {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        use Items::*;
        match self {
            Use(def) => tt.write(def),
            OneOf(def) => tt.write(def),
            Enum(def) => tt.write(def),
            Struct(def) => tt.write(def),
            Type(def) => tt.write(def),
            Error(def) => tt.write(def),
            Operation(def) => tt.write(def),
            Namespace(def) => tt.write(def),
            SpannedNamespace(def) => tt.write(def),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::defs::Spanned;

    #[test_case::test_case(
        "
namespace test;

#[version(1)]
struct test_arrays {
    a: str[]
};", 2; "parses struct"
    )]
    #[test_case::test_case(
        "
namespace test;

#[version(1)]
oneof test_oneof {
    a(i32),
    b { desc: i32[] },
};", 2; "parses named oneof"
    )]
    #[test_case::test_case(
        "
namespace test;

#[version(1)]
error test_error {
    a(i32),
    b { desc: i32[] },
};", 2; "parses named error"
    )]
    #[test_case::test_case(
        "
namespace test;
use abc::foo;
", 2; "parses use"
    )]
    #[test_case::test_case(
        "
namespace test;

// an infallible operation
operation add(a: i32, b: i32) -> i32;
", 2; "parses 2 arg operation without result type"
    )]
    #[test_case::test_case(
        "
namespace test;

operation foo() -> i32!;
", 2; "parses 0 arg operation with result type"
    )]
    #[test_case::test_case(
        "
namespace test;

#[err(MyError)]
operation foo() -> i32!;
", 2; "parses 0 arg operation with result type and meta"
    )]
    #[test_case::test_case(
        "
namespace test {
    operation foo() -> i32!;
};
", 1; "parses nested namespace"
    )]
    #[test_case::test_case(
        "
enum Foo {
    Abc
};
", 1; "parses enum with default"
    )]
    #[test_case::test_case(
        "
enum Foo {
    Abc = 1
};
", 1; "parses int enum"
    )]
    #[test_case::test_case(
        "
enum Foo {
    Abc = \"life\"
};
", 1; "parses str enum"
    )]
    fn basic_smoke(
        src: &str,
        n_items: usize,
    ) {
        let items: Vec<Spanned<super::Items>> = crate::tst::basic_smoke(src).unwrap();
        assert_eq!(items.len(), n_items);
    }
}
