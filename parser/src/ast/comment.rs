use crate::{
    defs::Spanned,
    tokens::{self, ImplDiagnostic},
};

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum CommentAst {
    SingleLine(Spanned<tokens::CommentSingleLineToken>),
    MultiLine(Spanned<tokens::CommentMultiLineToken>),
}

impl tokens::ToTokens for CommentAst {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            Self::SingleLine(cmt) => {
                tt.write(cmt);
                tt.add_newline();
            },
            Self::MultiLine(cmt) => {
                let content = cmt.borrow_string();
                if content.contains('\n') {
                    tt.word("/*");
                    tt.buf.push('\n');
                    tt.indent_level += 1;

                    for line in content.lines() {
                        tt.add_indent();
                        tt.word(line.trim_start());
                        tt.buf.push('\n');
                    }

                    tt.indent_level -= 1;
                    tt.add_indent();
                    tt.word("*/");
                    tt.add_newline();
                } else {
                    tt.word("/* ");
                    tt.word(content);
                    tt.word(" */");
                    tt.add_newline();
                }
            },
        }
    }
}

impl ImplDiagnostic for CommentAst {
    fn fmt() -> &'static str {
        "// <comment> | /* comment */"
    }
}

impl tokens::Peek for CommentAst {
    fn is(token: &tokens::Token) -> bool {
        tokens::CommentMultiLineToken::is(token) || tokens::CommentSingleLineToken::is(token)
    }
}

impl tokens::Parse for CommentAst {
    fn parse(stream: &mut tokens::TokenStream) -> Result<Self, tokens::LexingError> {
        Ok(if stream.peek::<tokens::CommentMultiLineToken>() {
            Self::MultiLine(stream.parse()?)
        } else {
            Self::SingleLine(stream.parse()?)
        })
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct CommentStream {
    pub comments: Vec<Spanned<CommentAst>>,
}

impl tokens::Parse for CommentStream {
    fn parse(stream: &mut tokens::TokenStream) -> Result<Self, tokens::LexingError> {
        Ok(Self {
            comments: tokens::Parse::parse(stream)?,
        })
    }
}

impl Default for CommentStream {
    fn default() -> Self {
        Self::new()
    }
}

impl CommentStream {
    pub fn new() -> Self {
        Self { comments: vec![] }
    }

    pub fn new_from(comments: Vec<Spanned<CommentAst>>) -> Self {
        Self { comments }
    }

    pub fn comments(&self) -> impl Iterator<Item = &String> {
        self.comments.iter().map(|it| {
            match &it.value {
                CommentAst::MultiLine(multi) => multi.borrow_string(),
                CommentAst::SingleLine(single) => single.borrow_string(),
            }
        })
    }
}

impl tokens::ToTokens for CommentStream {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        for c in &self.comments {
            c.value.write(tt);
        }
    }
}

#[cfg(test)]
mod test {
    use super::CommentStream;
    use crate::{defs::Spanned, tokens::*};

    #[test_case::test_case(
        "
        // first single line
        /*
            subsequent multi-line.
            the next line within.
        */
        ", vec!["first single line", "subsequent multi-line.\nthe next line within."];
        "parses multiple comment stream types"
    )]
    #[test_case::test_case(
        "", vec![];
        "parses empty"
    )]
    fn test_comments_parse(
        src: &str,
        expect: Vec<&str>,
    ) {
        let mut t = tokenize(src).unwrap();
        let parsed: Spanned<CommentStream> = t.parse().unwrap();
        let found: Vec<_> = parsed.comments().collect();
        assert_eq!(found, expect)
    }

    #[test_case::test_case(
        "// first single line\n/*\n\tsubsequent multi-line.\n\tthe next line within.\n*/\n";
        "comment stream round trip"
    )]
    pub fn round_trip(src: &str) {
        crate::tst::round_trip::<CommentStream>(src).unwrap();
    }
}
