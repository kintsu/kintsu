pub mod anonymous;
pub mod array;
pub mod comment;
pub mod enm;
pub mod err;
pub mod import;
pub mod items;
pub mod meta;
pub mod namespace;
pub mod one_of;
pub mod op;
pub mod path;
pub mod strct;
pub mod ty;
pub mod ty_def;
pub mod union;
pub mod variadic;

use std::{path::Path, sync::Arc};

use miette::IntoDiagnostic;

use crate::{
    Parse,
    ast::comment::CommentStream,
    defs::Spanned,
    tokens::{AstResult, TokenStream, tokenize},
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct AstStream {
    pub module_comments: CommentStream,
    pub module_meta: Spanned<meta::ItemMeta>,
    pub nodes: Vec<Spanned<items::Items>>,
}

impl crate::Parse for AstStream {
    fn parse(stream: &mut crate::tokens::TokenStream) -> AstResult<Self> {
        Ok(Self {
            module_comments: CommentStream::parse(stream)?,
            module_meta: stream.parse()?,
            nodes: Vec::parse(stream)?,
        })
    }
}

impl AstStream {
    pub fn format(
        &self,
        cfg: &crate::fmt::FormatConfig,
    ) -> String {
        let mut p = crate::fmt::Printer::new(cfg);
        p.write(self);
        p.buf
    }

    pub fn from_tokens(tt: &mut TokenStream) -> AstResult<Self> {
        let ast = Self::parse(tt)?;
        tt.ensure_consumed()?;
        Ok(ast)
    }

    pub fn from_tokens_with(
        path: impl AsRef<Path>,
        tt: &mut TokenStream,
    ) -> crate::Result<Self> {
        Self::from_tokens(tt).map_err(|lex| {
            let crate_err: crate::Error = lex.into();
            crate_err.with_source(
                path.as_ref().to_path_buf(),
                Arc::new(String::from(tt.source())),
            )
        })
    }

    pub fn from_string(src: &str) -> AstResult<Self> {
        let mut tt = tokenize(src)?;
        Self::from_tokens(&mut tt)
    }

    pub fn from_string_with(
        path: impl AsRef<Path>,

        data: &str,
    ) -> miette::Result<Self> {
        Self::from_string(data).map_err(|lex| {
            let crate_err: crate::Error = lex.into();
            crate_err.to_report_with(path.as_ref(), data, None)
        })
    }

    pub fn from_file_sync(path: impl AsRef<Path>) -> miette::Result<Self> {
        let data = std::fs::read_to_string(path.as_ref()).into_diagnostic()?;

        Self::from_string_with(path, &data)
    }

    pub async fn from_file(path: impl AsRef<Path>) -> miette::Result<Self> {
        let data = tokio::fs::read_to_string(path.as_ref())
            .await
            .into_diagnostic()?;

        Self::from_string_with(path, &data)
    }
}

impl IntoIterator for AstStream {
    type Item = Spanned<items::Items>;
    type IntoIter = <Vec<Spanned<items::Items>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}

impl crate::tokens::ToTokens for AstStream {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        if !self.module_comments.comments.is_empty() {
            tt.write(&self.module_comments);
        }

        if !self.module_meta.meta.is_empty() {
            tt.write(&self.module_meta);
        }

        for (i, node) in self.nodes.iter().enumerate() {
            tt.write(node);
            tt.add_newline();
            if i < self.nodes.len() - 1 {
                tt.buf.push('\n');
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::fmt::FormatConfig;

    use super::*;

    #[test_case::test_case("samples/array.ks")]
    #[test_case::test_case("samples/complex_union.ks")]
    #[test_case::test_case("samples/enum.ks")]
    #[test_case::test_case("samples/error.ks")]
    #[test_case::test_case("samples/explicit_oneof.ks")]
    #[test_case::test_case("samples/message_with_enum.ks")]
    #[test_case::test_case("samples/mod.ks")]
    #[test_case::test_case("samples/ns.ks")]
    #[test_case::test_case("samples/op.ks")]
    #[test_case::test_case("samples/some_import.ks")]
    #[test_case::test_case("samples/test_message.ks")]
    fn round_trip(path: &str) {
        crate::tst::logging();

        let data = std::fs::read_to_string(path)
            .unwrap()
            .replace("    ", "\t");
        let ast = AstStream::from_file_sync(path).unwrap();
        let cfg = FormatConfig::default();
        let fmt = ast.format(&cfg);

        assert_eq!(data, fmt, "expected:\n{}\ngot:\n{}", data, fmt);
    }
}
