use crate::{
    Parse, Peek, Token,
    defs::Spanned,
    tokens::{Brace, Repeated, ToTokens, brace},
};

pub type StructFields = Spanned<Repeated<super::strct::Arg, Token![,]>>;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct AnonymousStruct {
    pub(crate) brace: Brace,
    pub(crate) fields: StructFields,
}

impl Parse for AnonymousStruct {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        let mut braced;
        Ok(Self {
            brace: brace!(braced in stream),
            fields: braced.parse()?,
        })
    }
}

impl Peek for AnonymousStruct {
    fn peek(stream: &crate::tokens::TokenStream) -> bool {
        let mut fork = stream.fork();

        let mut braced;
        let _ = brace!(braced in fork; false);

        let _: StructFields = crate::bail_unchecked!(braced.parse(); false);

        true
    }
}

impl ToTokens for AnonymousStruct {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        use crate::tokens::toks::Token as TokType;

        tt.open_block();

        for (i, item) in self.fields.value.values.iter().enumerate() {
            tt.write(&item.value);
            if i < self.fields.value.values.len() - 1 {
                tt.token(&TokType::Comma);
                tt.add_newline();
            }
        }

        tt.close_block();
    }
}

#[cfg(test)]
mod test {
    #[test_case::test_case("{\n\ta: i32\n}"; "basic one field")]
    #[test_case::test_case("{\n\ta: i32,\n\tb: i64\n}"; "basic multi field")]
    #[test_case::test_case("{\n\ta: i32,\n\t/* some comment */\n\tb: i64\n}"; "fields with comment")]
    pub fn rt_anon(src: &str) {
        crate::tst::round_trip::<super::AnonymousStruct>(src).unwrap();
    }
}
