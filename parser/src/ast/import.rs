use crate::{
    ImplDiagnostic, SpannedToken, Token,
    ast::ty::PathOrIdent,
    ctx::{RefContext, RefOrItemContext},
    defs::Spanned,
    tokens::{self, Brace, Parse, Peek, Repeated, ToTokens, Token, brace},
    utils::guard_schema,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub enum FinalOrNested {
    Nest(UsePath),
    Final(PathOrIdent),
}

impl ImplDiagnostic for FinalOrNested {
    fn fmt() -> &'static str {
        "`object` or `leading::trail::{object, next_object}`"
    }
}

impl Parse for FinalOrNested {
    fn parse(stream: &mut tokens::TokenStream) -> Result<Self, tokens::LexingError> {
        let leading = PathOrIdent::parse(stream)?;

        Ok(if stream.peek::<UseWithItems>() {
            Self::Nest(UsePath {
                leading,
                items: Option::parse(stream)?,
            })
        } else {
            Self::Final(leading)
        })
    }
}

impl Peek for FinalOrNested {
    fn is(token: &Token) -> bool {
        PathOrIdent::is(token)
    }

    fn peek(stream: &tokens::TokenStream) -> bool {
        PathOrIdent::peek(stream)
    }
}

impl ToTokens for FinalOrNested {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            Self::Final(id) => tt.write(id),
            Self::Nest(p) => tt.write(p),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UseWithItems {
    pub fish: SpannedToken![::],
    pub brace: Brace,
    pub inner: Repeated<FinalOrNested, Token![,]>,
}

impl Peek for UseWithItems {
    fn is(token: &Token) -> bool {
        <Token![::]>::is(token)
    }
}

impl Parse for UseWithItems {
    fn parse(stream: &mut tokens::TokenStream) -> Result<Self, tokens::LexingError> {
        let mut braced;
        Ok(Self {
            fish: stream.parse()?,
            brace: brace!(braced in stream),
            inner: Repeated::parse(&mut braced)?,
        })
    }
}

impl ToTokens for UseWithItems {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.fish);
        if self.inner.values.len() != 1 {
            tt.open_block();

            tt.write_comma_separated(self.inner.values.iter().map(|v| &v.value));

            tt.close_block();
        } else {
            tt.write(&self.inner.values[0].value)
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UsePath {
    pub leading: PathOrIdent,
    pub items: Option<Spanned<UseWithItems>>,
}

impl UsePath {
    #[tracing::instrument(skip(self))]
    fn paths_with_context(
        &self,
        ref_ctx: &RefContext,
    ) -> Vec<RefOrItemContext> {
        tracing::debug!("resolving paths with context");

        let mut paths = Vec::new();

        if let Some(items) = &self.items {
            for item in &items.value.inner.values {
                match item.value.value {
                    FinalOrNested::Final(ref term) => {
                        match term {
                            PathOrIdent::Ident(id) => {
                                paths.push(ref_ctx.enter(id.borrow_string()).into());
                            },
                            PathOrIdent::Path(p) => {
                                let new_ctx = ref_ctx.extend(p.borrow_path_inner().segments());
                                paths.push(new_ctx.into());
                            },
                        }
                    },
                    FinalOrNested::Nest(ref np) => {
                        let nested_paths = np.paths_with_context(ref_ctx);
                        paths.extend(nested_paths);
                    },
                }
            }
        }
        paths
    }

    /// get the root identifier of the use path (first segment)
    pub fn root_ident(&self) -> &str {
        match &self.leading {
            PathOrIdent::Ident(ident) => ident.borrow_string(),
            PathOrIdent::Path(path) => {
                // extract first segment from path
                let path_inner = path.borrow_path_inner();
                match path_inner {
                    crate::ast::path::Path::Local { bits }
                    | crate::ast::path::Path::Ambiguous { bits } => {
                        bits.first()
                            .map(|s| s.as_str())
                            .unwrap_or("")
                    },
                }
            },
        }
    }

    /// check if this is a single-segment path (no ::)
    pub fn is_single_segment(&self) -> bool {
        matches!(&self.leading, PathOrIdent::Ident(_)) && self.items.is_none()
    }

    /// check if this has nested items (::{ ... })
    pub fn has_nested_items(&self) -> bool {
        self.items.is_some()
    }

    #[tracing::instrument(skip(self), fields(
        leading = self.leading.display(),
        items = self.items.as_ref().map(|i| i.display())
    ))]
    pub fn qualified_paths(
        &self,
        pkg: String,
    ) -> Vec<RefOrItemContext> {
        tracing::debug!("resolving qualified paths for use statement");

        let mut paths = Vec::new();
        let ref_ctx = {
            let mut bits = vec![];
            let pkg = match self.leading {
                PathOrIdent::Ident(ref id) => guard_schema(&pkg, id.borrow_string()),
                PathOrIdent::Path(ref p) => {
                    let mut segments = p.borrow_path_inner().segments().clone();
                    let first = segments.remove(0);

                    bits.append(&mut segments);

                    guard_schema(&pkg, &first)
                },
            };

            RefContext::new(pkg, bits)
        };

        if self.items.is_none() {
            paths.push(ref_ctx.clone().into());
        } else {
            paths.extend(self.paths_with_context(&ref_ctx));
        }

        paths
    }
}

impl Parse for UsePath {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        let leading = PathOrIdent::parse(stream)?;
        Ok(if stream.peek::<UseWithItems>() {
            Self {
                leading,
                items: Option::parse(stream)?,
            }
        } else {
            Self {
                leading,
                items: None,
            }
        })
    }
}

impl ToTokens for UsePath {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.leading);
        tt.write(&self.items);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Use {
    pub kw: SpannedToken![use],
    pub path: Spanned<UsePath>,
}

impl Use {
    pub fn root_ident(&self) -> &str {
        self.path.value.root_ident()
    }

    pub fn is_single_segment(&self) -> bool {
        self.path.value.is_single_segment()
    }

    pub fn has_nested_items(&self) -> bool {
        self.path.value.has_nested_items()
    }
}

impl Parse for Use {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        Ok(Self {
            kw: stream.parse()?,
            path: stream.parse()?,
        })
    }
}

impl ToTokens for Use {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.kw);
        tt.space();
        tt.write(&self.path);
    }
}

impl Peek for Use {
    fn is(token: &Token) -> bool {
        <Token![use]>::is(token)
    }
}

#[cfg(test)]
mod test {
    use crate::tokens::ToTokens;

    #[test_case::test_case("use foo"; "use one ident")]
    #[test_case::test_case("use foo::bar"; "use one path")]
    #[test_case::test_case("use foo::bar::baz"; "use one object")]
    #[test_case::test_case("use bar_corp::baz::BazOrString"; "use corp object")]
    #[test_case::test_case("use foo::bar::{\n\tbaz,\n\tbin\n}"; "use two objects")]
    fn rt(src: &str) {
        crate::tst::round_trip::<super::Use>(src).unwrap();
    }

    #[test_case::test_case("use foo", vec!["foo"]; "use one ident")]
    #[test_case::test_case("use foo::bar", vec!["foo::bar"]; "use one path")]
    #[test_case::test_case("use foo::bar::baz", vec!["foo::bar::baz"]; "use one object")]
    #[test_case::test_case("use bar_corp::baz::BazOrString", vec!["bar_corp::baz::BazOrString"]; "use corp object")]
    #[test_case::test_case("use foo::bar::{baz, Bin}", vec!["foo::bar::baz", "foo::bar::Bin"]; "simple nested use")]
    #[test_case::test_case("use schema::bar::{baz, bin}", vec!["my_pkg::bar::baz", "my_pkg::bar::bin"]; "local use with schema")]
    fn resolved_use_paths(
        src: &str,
        expected: Vec<&str>,
    ) {
        let use_stmt = crate::tst::basic_smoke::<super::Use>(src).unwrap();
        let paths = use_stmt
            .path
            .value
            .qualified_paths("my_pkg".to_string());

        let path_strs: Vec<String> = paths
            .into_iter()
            .map(|p| p.display())
            .collect();

        let expected_strs: Vec<String> = expected
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(path_strs, expected_strs);
    }
}
