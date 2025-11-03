use crate::{
    Token,
    ast::ty::PathOrIdent,
    bail_unchecked,
    defs::Spanned,
    tokens::{
        self, BangToken, Bracket, HashToken, IdentToken, Paren, Parse, Peek, ToTokens, bracket,
        paren,
    },
};

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Meta<Value: Parse> {
    pub open: Spanned<HashToken>,
    pub inner: Option<Spanned<BangToken>>,
    pub bracket: Bracket,
    pub name: Spanned<IdentToken>,
    pub paren: Paren,
    pub value: Spanned<Value>,
}

impl<Value: Parse + Peek> tokens::Peek for Meta<Value> {
    fn peek(stream: &tokens::TokenStream) -> bool {
        let mut stream = stream.fork();

        let mut bracket;
        let mut paren;
        let open = bail_unchecked!(stream.parse(); false);

        let inner = bail_unchecked!(Option::parse(&mut stream); false);

        let bracket_tok = bracket!(bracket in stream; false);

        let name = bail_unchecked!(bracket.parse(); false);

        let paren_tok = paren!(paren in bracket; false);

        let _ = Self {
            open,
            inner,
            bracket: bracket_tok,
            name,
            paren: paren_tok,
            value: bail_unchecked!(paren.parse(); false),
        };

        true
    }
}

impl<Value: Parse + Peek> tokens::Parse for Meta<Value> {
    fn parse(stream: &mut tokens::TokenStream) -> Result<Self, tokens::LexingError> {
        let mut bracket;
        let mut paren;
        Ok(Self {
            open: stream.parse()?,
            inner: Option::parse(stream)?,
            bracket: bracket!(bracket in stream),
            name: bracket.parse()?,
            paren: paren!(paren in bracket),
            value: paren.parse()?,
        })
    }
}

impl<Value: Parse + Peek> ToTokens for Meta<Value>
where
    Spanned<Value>: ToTokens,
{
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.open);
        tt.write(&self.inner);
        self.bracket.write_with(tt, |tt| {
            tt.write(&self.name);
            self.paren
                .write_with(tt, |tt| tt.write(&self.value));
        });
        tt.add_newline();
    }
}

pub type IntMeta = Meta<Token![number]>;
pub type StrMeta = Meta<Token![string]>;
pub type IdentMeta = Meta<PathOrIdent>;

pub type VersionMeta = Spanned<IntMeta>;
pub type ErrorMeta = Spanned<IdentMeta>;

impl VersionMeta {
    pub fn version_value(&self) -> i32 {
        *self.value.value.borrow_i32()
    }

    pub fn version_spanned(&self) -> Spanned<u32> {
        let val = *self.value.value.borrow_i32();
        Spanned::new(self.span().start, self.span().end, val as u32)
    }
}

impl ErrorMeta {
    pub fn error_name(&self) -> &PathOrIdent {
        &self.value.value.value
    }

    pub fn error_name_spanned(&self) -> Spanned<PathOrIdent> {
        let name = self.error_name().clone();
        Spanned::new(self.span().start, self.span().end, name)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "meta_type", rename_all = "snake_case")]
pub enum ItemMetaItem {
    Version(VersionMeta),
    Error(ErrorMeta),
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ItemMeta {
    pub meta: Vec<ItemMetaItem>,
}

impl Default for ItemMeta {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemMeta {
    pub fn new() -> Self {
        Self { meta: vec![] }
    }
}

impl Parse for ItemMeta {
    fn parse(stream: &mut tokens::TokenStream) -> Result<Self, tokens::LexingError> {
        let mut meta = vec![];
        loop {
            if stream.peek::<IntMeta>() {
                let this: Spanned<IntMeta> = stream.parse()?;
                match this.name.borrow_string().as_ref() {
                    "version" => meta.push(ItemMetaItem::Version(this)),
                    unknown => {
                        return Err(crate::LexingError::unknown_meta(
                            vec!["version"],
                            unknown.into(),
                            &this.name.span,
                        ));
                    },
                }
            } else if stream.peek::<StrMeta>() {
                let this: Spanned<StrMeta> = stream.parse()?;
                return Err(crate::LexingError::unknown_meta(
                    vec!["<placeholder>"],
                    this.name.borrow_string().into(),
                    &this.name.span,
                ));
            } else if stream.peek::<IdentMeta>() {
                let this: Spanned<IdentMeta> = stream.parse()?;
                match this.name.borrow_string().as_ref() {
                    "err" => meta.push(ItemMetaItem::Error(this)),
                    unknown => {
                        return Err(crate::LexingError::unknown_meta(
                            vec!["err"],
                            unknown.into(),
                            &this.name.span,
                        ));
                    },
                }
            } else {
                break;
            }
        }

        Ok(Self { meta })
    }
}

impl ToTokens for ItemMetaItem {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            ItemMetaItem::Version(m) => tt.write(m),
            ItemMetaItem::Error(m) => tt.write(m),
        }
    }
}

impl ToTokens for ItemMeta {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        for m in &self.meta {
            tt.write(m);
        }
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use crate::{
        Error,
        tokens::{AstResult, Parse, tokenize},
    };

    use super::*;

    fn parse_meta<Value: Parse + Peek>(input: &str) -> AstResult<Meta<Value>> {
        let mut stream = tokenize(input)?;
        Meta::parse(&mut stream)
    }

    #[test_case::test_case("#[abc(\"value\")]", "abc", tokens::StringToken::new("value".into()) ; "string value")]
    #[test_case::test_case("#[abc(1)]", "abc", tokens::NumberToken::new(1) ; "int value")]
    #[test_case::test_case("#![abc(\"value\")]", "abc", tokens::StringToken::new("value".into()) ; "string value with bang")]
    #[test_case::test_case("#![abc(42)]", "abc", tokens::NumberToken::new(42) ; "int value with bang")]
    fn test_meta_parse<Value: Peek + Parse + PartialEq + Debug>(
        input: &str,
        expected_name: &str,
        expected_value: Value,
    ) {
        let meta: Meta<Value> = parse_meta(input).expect("Should parse");
        assert_eq!(meta.name.value.borrow_string(), expected_name);
        assert_eq!(meta.value.value, expected_value);
    }

    #[test_case::test_case("#[version(1)]", 1; "version from outer")]
    #[test_case::test_case("#![version(2)]", 2; "version from inner")]
    fn test_item_parse(
        src: &str,
        expect_version: i32,
    ) {
        let mut tt = tokenize(src).expect("Should parse");
        let meta: Spanned<ItemMeta> = tt.parse().unwrap();
        let version = match meta.meta.first().unwrap() {
            ItemMetaItem::Version(ver) => ver,
            #[allow(unreachable_patterns)]
            _ => panic!("not version"),
        };
        assert_eq!(*version.value.value.borrow_i32(), expect_version);
    }

    #[test_case::test_case("#[err(MyError)]", "MyError"; "error from outer")]
    #[test_case::test_case("#![err(MyError)]", "MyError"; "error from inner")]
    fn test_err_parse(
        src: &str,
        expect_error: &str,
    ) {
        let mut tt = tokenize(src).expect("Should parse");
        println!("{tt:#?}");
        let meta: Spanned<ItemMeta> = tt.parse().unwrap();
        assert!(!meta.meta.is_empty(), "meta.meta is empty!");
        let error = match meta.meta.first().unwrap() {
            ItemMetaItem::Error(err) => err,
            #[allow(unreachable_patterns)]
            _ => panic!("not error"),
        };
        let error_name = match &error.value.value.value {
            crate::ast::ty::PathOrIdent::Ident(ident) => ident.borrow_string(),
            crate::ast::ty::PathOrIdent::Path(..) => unreachable!(),
        };
        assert_eq!(error_name, expect_error);
    }

    #[test_case::test_case("#[unknown(1)]", vec![
        "unknown meta attribute, 'unknown'. expected one of version",
        "1:3"
    ]; "unknown int meta")]
    #[test_case::test_case("#[foo(\"bar\")]", vec![
        "unknown meta attribute, 'foo'. expected one of <placeholder>",
        "1:3"
    ]; "unknown string meta")]
    #[test_case::test_case("#![baz(42)]", vec![
        "unknown meta attribute, 'baz'. expected one of version",
        "1:4"
    ]; "unknown int meta with bang")]
    #[test_case::test_case("#![qux(\"val\")]", vec![
        "unknown meta attribute, 'qux'. expected one of <placeholder>",
        "1:4"
    ]; "unknown string meta with bang")]
    fn test_unknown_meta(
        src: &str,
        expected_diag: Vec<&str>,
    ) {
        let mut tt = tokenize(src).expect("Should tokenize");
        let p = std::path::Path::new("foo.ks");
        let err = (match tt
            .parse::<Spanned<ItemMeta>>()
            .map_err(Error::from)
        {
            Ok(_) => panic!("ok"),
            Err(e) => e,
        })
        .to_report_with(p, &tt.source, None);
        let err = format!("{err:?}");
        eprintln!("{err}");

        for d in expected_diag {
            assert!(
                err.contains(d),
                "Expected diagnostic to contain:\n{}\nActual diagnostic:\n{}",
                d,
                err
            );
        }
    }
}
