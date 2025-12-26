use crate::{
    Token,
    ast::ty::PathOrIdent,
    bail_unchecked,
    defs::Spanned,
    tokens::{
        self, BangToken, Bracket, EqToken, HashToken, IdentToken, Paren, Parse, Peek, ToTokens,
        bracket, paren,
    },
};

/// Raw content for `#[tag(...)]` attributes.
/// Parses the content inside the parentheses as comma-separated key-value pairs.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct RawTagContent {
    /// Parsed tag arguments, either keywords or key=value pairs
    pub args: Vec<TagArg>,
}

/// A single argument in a tag attribute.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum TagArg {
    /// Simple keyword: `external`, `untagged`, `index`, `type_hint`
    Keyword(Spanned<IdentToken>),
    /// Key-value pair with string: `name = "kind"`
    StringValue {
        key: Spanned<IdentToken>,
        eq: Spanned<EqToken>,
        value: Spanned<crate::tokens::StringToken>,
    },
    /// Key-value pair with bool: `type_hint = false`
    BoolValue {
        key: Spanned<IdentToken>,
        eq: Spanned<EqToken>,
        value: Spanned<IdentToken>,
    },
}

impl Peek for RawTagContent {
    fn peek(stream: &crate::tokens::TokenStream) -> bool {
        // Tag content can start with an identifier (keyword or key)
        stream.peek::<IdentToken>()
    }
}

impl Parse for RawTagContent {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        let mut args = vec![];

        while stream.peek::<IdentToken>() {
            let ident: Spanned<IdentToken> = stream.parse()?;

            // Check if this is key=value or just a keyword
            if stream.peek::<EqToken>() {
                let eq: Spanned<EqToken> = stream.parse()?;

                // Value can be string or bool identifier (true/false)
                if stream.peek::<crate::tokens::StringToken>() {
                    let value: Spanned<crate::tokens::StringToken> = stream.parse()?;
                    args.push(TagArg::StringValue {
                        key: ident,
                        eq,
                        value,
                    });
                } else {
                    // Must be bool identifier (true/false) or another keyword
                    let value: Spanned<IdentToken> = stream.parse()?;
                    args.push(TagArg::BoolValue {
                        key: ident,
                        eq,
                        value,
                    });
                }
            } else {
                // Just a keyword
                args.push(TagArg::Keyword(ident));
            }

            // Consume optional comma separator
            if stream.peek::<crate::tokens::CommaToken>() {
                let _: Spanned<crate::tokens::CommaToken> = stream.parse()?;
            } else {
                break;
            }
        }

        Ok(Self { args })
    }
}

impl ToTokens for RawTagContent {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                tt.word(", ");
            }
            match arg {
                TagArg::Keyword(k) => tt.write(k),
                TagArg::StringValue { key, value, .. } => {
                    tt.write(key);
                    tt.word(" = ");
                    tt.write(value);
                },
                TagArg::BoolValue { key, value, .. } => {
                    tt.write(key);
                    tt.word(" = ");
                    tt.write(value);
                },
            }
        }
    }
}

/// Type alias for raw tag meta parsing
pub type RawTagMeta = Meta<RawTagContent>;
/// Type alias for rename meta - takes a single string argument
pub type RawRenameMeta = Meta<crate::tokens::StringToken>;

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
    /// Tagging attribute: `#[tag(...)]` or `#![tag(...)]`
    /// Full parsing support added in Phase 3 (Tagging implementation)
    Tag(TagMeta),
    /// Rename attribute: `#[rename("name")]`
    /// Full parsing support added in Phase 3 (Tagging implementation)
    Rename(RenameMeta),
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
            // Check if this looks like a meta attribute (starts with #)
            if !stream.peek::<HashToken>() {
                break;
            }

            // Peek at the attribute name to decide how to parse
            let meta_name = peek_meta_name(stream);

            match meta_name.as_deref() {
                Some("version") => {
                    let this: Spanned<IntMeta> = stream.parse()?;
                    meta.push(ItemMetaItem::Version(this));
                },
                Some("err") => {
                    let this: Spanned<IdentMeta> = stream.parse()?;
                    meta.push(ItemMetaItem::Error(this));
                },
                Some("tag") => {
                    let raw: Spanned<RawTagMeta> = stream.parse()?;
                    let tag_attr = parse_tag_content(&raw.value.value)?;
                    let tag_meta =
                        Spanned::new(raw.span.span().start, raw.span.span().end, tag_attr);
                    meta.push(ItemMetaItem::Tag(tag_meta));
                },
                Some("rename") => {
                    let raw: Spanned<StrMeta> = stream.parse()?;
                    let rename_attr = RenameAttribute {
                        name: raw.value.value.borrow_string().to_string(),
                    };
                    let rename_meta =
                        Spanned::new(raw.span.span().start, raw.span.span().end, rename_attr);
                    meta.push(ItemMetaItem::Rename(rename_meta));
                },
                Some(unknown) => {
                    // Consume the meta using RawTagMeta which is most permissive
                    let raw: Spanned<RawTagMeta> = stream.parse()?;
                    return Err(crate::LexingError::unknown_meta(
                        vec!["version", "err", "tag", "rename"],
                        unknown.into(),
                        &raw.name.span,
                    ));
                },
                None => break,
            }
        }

        Ok(Self { meta })
    }
}

/// Peek at the name of a meta attribute without consuming
fn peek_meta_name(stream: &tokens::TokenStream) -> Option<String> {
    let mut stream = stream.fork();
    // Skip # and optional !
    let _: Spanned<HashToken> = stream.parse().ok()?;
    let _: Option<Spanned<BangToken>> = Option::parse(&mut stream).ok()?;
    // Parse [ ... ]
    let mut bracket;
    let _ = bracket!(bracket in stream; None);
    // Get the name
    let name: Spanned<IdentToken> = bracket.parse().ok()?;
    Some(name.borrow_string().to_string())
}

/// Parse RawTagContent into TagAttribute per RFC-0017 syntax
fn parse_tag_content(content: &RawTagContent) -> Result<TagAttribute, crate::LexingError> {
    let mut style = TagStyle::TypeHint;
    let mut type_hint = true;
    let mut name_field: Option<String> = None;
    let mut content_field: Option<String> = None;
    let mut is_index = false;

    for arg in &content.args {
        match arg {
            TagArg::Keyword(kw) => {
                let kw_str = kw.borrow_string();
                match kw_str.as_ref() {
                    "external" => style = TagStyle::External,
                    "untagged" => {
                        style = TagStyle::Untagged;
                        type_hint = false;
                    },
                    "index" => is_index = true,
                    "type_hint" => {}, // Default, just confirming
                    _ => {
                        // Unknown keyword in tag - use unknown_meta error
                        return Err(crate::LexingError::unknown_meta(
                            vec!["external", "untagged", "index", "type_hint"],
                            kw_str.to_string(),
                            &kw.span,
                        ));
                    },
                }
            },
            TagArg::StringValue { key, value, .. } => {
                let key_str = key.borrow_string();
                let val_str = value.borrow_string().to_string();
                match key_str.as_ref() {
                    "name" => name_field = Some(val_str),
                    "content" => content_field = Some(val_str),
                    _ => {
                        return Err(crate::LexingError::unknown_meta(
                            vec!["name", "content"],
                            key_str.to_string(),
                            &key.span,
                        ));
                    },
                }
            },
            TagArg::BoolValue { key, value, .. } => {
                let key_str = key.borrow_string();
                let val_str = value.borrow_string();
                match key_str.as_ref() {
                    "type_hint" => {
                        type_hint = val_str == "true";
                    },
                    _ => {
                        return Err(crate::LexingError::unknown_meta(
                            vec!["type_hint"],
                            key_str.to_string(),
                            &key.span,
                        ));
                    },
                }
            },
        }
    }

    // Determine final style based on parsed args
    if is_index {
        style = TagStyle::Index { name: name_field };
    } else if let Some(name) = name_field {
        if let Some(content) = content_field {
            style = TagStyle::Adjacent { name, content };
        } else {
            style = TagStyle::Internal { name };
        }
    }

    Ok(TagAttribute { style, type_hint })
}

impl ToTokens for ItemMetaItem {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            ItemMetaItem::Version(m) => tt.write(m),
            ItemMetaItem::Error(m) => tt.write(m),
            ItemMetaItem::Tag(m) => m.write(tt),
            ItemMetaItem::Rename(m) => m.write(tt),
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

// Variant Tagging Support per RFC-0017, SPEC-0016, TSY-0013
/// Tag style for variant types (oneof, error).
///
/// Controls how discriminated unions are serialized with type identifiers.
/// Per SPEC-0016 Section "Parsed AST representation".
///
/// **Spec references:** RFC-0017, SPEC-0016, TSY-0013
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "style", rename_all = "snake_case")]
pub enum TagStyle {
    /// Type hint tagging (default) - untagged + @kintsu discriminator field.
    /// This is the global default when no `#[tag(...)]` is specified.
    #[default]
    TypeHint,

    /// External tagging: `{ "variant_name": { ...fields... } }`
    External,

    /// Internal tagging: `{ "tag_field": "variant_name", ...fields... }`
    Internal {
        /// The field name for the tag (e.g., "kind", "type")
        name: String,
    },

    /// Adjacent tagging: `{ "tag_field": "variant_name", "content_field": {...} }`
    Adjacent {
        /// The field name for the tag (e.g., "kind")
        name: String,
        /// The field name for the content (e.g., "data")
        content: String,
    },

    /// Untagged: serialize variant directly without any discriminator.
    /// Requires variants to be structurally distinguishable.
    Untagged,

    /// Index-based tagging: uses numeric index instead of variant name.
    Index {
        /// Optional field name for the index (defaults to "kind" if None)
        name: Option<String>,
    },
}

/// Parsed `#[tag(...)]` attribute per SPEC-0016.
///
/// **Spec references:** RFC-0017, SPEC-0016, TSY-0013
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagAttribute {
    /// The resolved tagging style
    pub style: TagStyle,
    /// Whether type_hint is enabled (adds @kintsu field)
    pub type_hint: bool,
}

impl Default for TagAttribute {
    fn default() -> Self {
        Self {
            style: TagStyle::TypeHint,
            type_hint: true,
        }
    }
}

/// Parsed `#[rename(...)]` attribute per TSY-0013.
///
/// Applied to individual variants to override snake_case normalization.
///
/// **Spec references:** TSY-0013
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RenameAttribute {
    /// The custom serialized name for the variant
    pub name: String,
}

/// Type alias for tag meta - parsed `#[tag(style)]` or `#[tag(name = "x")]`
pub type TagMeta = Spanned<TagAttribute>;

/// Type alias for rename meta - parsed `#[rename("custom_name")]`
pub type RenameMeta = Spanned<RenameAttribute>;

impl ToTokens for TagAttribute {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.word("#[tag(");
        match &self.style {
            TagStyle::TypeHint => tt.word("type_hint"),
            TagStyle::External => tt.word("external"),
            TagStyle::Internal { name } => {
                tt.word("name = \"");
                tt.word(name);
                tt.word("\"");
            },
            TagStyle::Adjacent { name, content } => {
                tt.word("name = \"");
                tt.word(name);
                tt.word("\", content = \"");
                tt.word(content);
                tt.word("\"");
            },
            TagStyle::Untagged => tt.word("untagged"),
            TagStyle::Index { name } => {
                tt.word("index");
                if let Some(n) = name {
                    tt.word(", name = \"");
                    tt.word(n);
                    tt.word("\"");
                }
            },
        }
        if !self.type_hint && !matches!(self.style, TagStyle::TypeHint | TagStyle::Untagged) {
            tt.word(", type_hint = false");
        }
        tt.word(")]");
    }
}

impl ToTokens for RenameAttribute {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.word("#[rename(\"");
        tt.word(&self.name);
        tt.word("\")]");
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use crate::tokens::{AstResult, Parse, tokenize};

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

    // Tag attribute parsing tests per RFC-0017
    #[test_case::test_case("#[tag(external)]", TagStyle::External, true; "external tagging")]
    #[test_case::test_case("#[tag(untagged)]", TagStyle::Untagged, false; "untagged")]
    #[test_case::test_case("#[tag(index)]", TagStyle::Index { name: None }, true; "index tagging")]
    #[test_case::test_case("#[tag(type_hint)]", TagStyle::TypeHint, true; "explicit type_hint")]
    fn test_tag_keyword_parse(
        src: &str,
        expected_style: TagStyle,
        expected_type_hint: bool,
    ) {
        let mut tt = tokenize(src).expect("Should parse");
        let meta: Spanned<ItemMeta> = tt.parse().unwrap();
        let tag = match meta.meta.first().unwrap() {
            ItemMetaItem::Tag(t) => t,
            _ => panic!("expected Tag"),
        };
        assert_eq!(tag.value.style, expected_style);
        assert_eq!(tag.value.type_hint, expected_type_hint);
    }

    #[test]
    fn test_tag_internal_parse() {
        let mut tt = tokenize("#[tag(name = \"kind\")]").expect("Should parse");
        let meta: Spanned<ItemMeta> = tt.parse().unwrap();
        let tag = match meta.meta.first().unwrap() {
            ItemMetaItem::Tag(t) => t,
            _ => panic!("expected Tag"),
        };
        assert_eq!(
            tag.value.style,
            TagStyle::Internal {
                name: "kind".into()
            }
        );
        assert!(tag.value.type_hint);
    }

    #[test]
    fn test_tag_adjacent_parse() {
        let mut tt = tokenize("#[tag(name = \"type\", content = \"data\")]").expect("Should parse");
        let meta: Spanned<ItemMeta> = tt.parse().unwrap();
        let tag = match meta.meta.first().unwrap() {
            ItemMetaItem::Tag(t) => t,
            _ => panic!("expected Tag"),
        };
        assert_eq!(
            tag.value.style,
            TagStyle::Adjacent {
                name: "type".into(),
                content: "data".into()
            }
        );
    }

    #[test]
    fn test_tag_index_with_name() {
        let mut tt = tokenize("#[tag(index, name = \"t\")]").expect("Should parse");
        let meta: Spanned<ItemMeta> = tt.parse().unwrap();
        let tag = match meta.meta.first().unwrap() {
            ItemMetaItem::Tag(t) => t,
            _ => panic!("expected Tag"),
        };
        assert_eq!(
            tag.value.style,
            TagStyle::Index {
                name: Some("t".into())
            }
        );
    }

    #[test]
    fn test_tag_type_hint_false() {
        let mut tt = tokenize("#[tag(type_hint = false)]").expect("Should parse");
        let meta: Spanned<ItemMeta> = tt.parse().unwrap();
        let tag = match meta.meta.first().unwrap() {
            ItemMetaItem::Tag(t) => t,
            _ => panic!("expected Tag"),
        };
        assert!(!tag.value.type_hint);
    }

    #[test]
    fn test_rename_parse() {
        let mut tt = tokenize("#[rename(\"custom_name\")]").expect("Should parse");
        let meta: Spanned<ItemMeta> = tt.parse().unwrap();
        let rename = match meta.meta.first().unwrap() {
            ItemMetaItem::Rename(r) => r,
            _ => panic!("expected Rename"),
        };
        assert_eq!(rename.value.name, "custom_name");
    }
}
