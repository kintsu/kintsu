use crate::{
    ast::ty::TriesQualify,
    ctx::{NamedItemContext, RefOrItemContext},
    defs::Spanned,
    tokens::{AstResult, IdentToken, LexingError},
};
use std::{
    collections::BTreeSet,
    fmt::{Display, Formatter},
};

pub(crate) const VALID_FORMS: &str =
    "`schema::a::b`, `pkg::a::b`, `a::b::SomeStruct`, `SomeStruct`";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Path {
    Local { bits: Vec<String> },
    Ambiguous { bits: Vec<String> },
}
impl TriesQualify for Path {
    fn qualify_one(
        &self,
        context: &RefOrItemContext,
        potential: &mut BTreeSet<NamedItemContext>,
    ) {
        let context = match context {
            RefOrItemContext::Ref(ctx) => ctx,
            RefOrItemContext::Item(_) => return,
        };

        let bits = match self {
            Path::Local { bits } => bits,
            Path::Ambiguous { bits } => bits,
        };

        // we shouldn't have empty paths since this would be a compiler error, but JIC
        if bits.is_empty() {
            return;
        }

        let name = Spanned::call_site(<IdentToken>::new(bits.last().unwrap().to_string()));

        potential.insert(context.item(name));
    }
}

impl std::hash::Hash for Path {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        let fmt = self.to_string();
        fmt.hash(state);
    }
}

impl Path {
    pub fn parse(s: &str) -> AstResult<Self> {
        let parts: Vec<&str> = s.split("::").collect();

        if parts.is_empty() {
            return Err(LexingError::InvalidPath {
                input: s.into(),
                reason: format!("expected one of {VALID_FORMS}"),
            });
        }

        let bits = parts
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>();

        if bits[0] == "schema" {
            Ok(Path::Local { bits })
        } else {
            Ok(Path::Ambiguous { bits })
        }
    }

    pub fn segments(&self) -> &Vec<String> {
        match self {
            Path::Local { bits } => bits,
            Path::Ambiguous { bits } => bits,
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local { .. })
    }
}

impl Display for Path {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Path::Local { bits } => write!(f, "{}", bits.join("::")),
            Path::Ambiguous { bits } => {
                write!(f, "{}", bits.join("::"))
            },
        }
    }
}

mod serde {
    use super::Path;

    impl serde::Serialize for Path {
        fn serialize<S>(
            &self,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer, {
            serializer.serialize_str(&format!("{}", self))
        }
    }

    struct Visitor;

    impl<'v> serde::de::Visitor<'v> for Visitor {
        type Value = Path;

        fn expecting(
            &self,
            f: &mut std::fmt::Formatter,
        ) -> std::fmt::Result {
            write!(f, "string")
        }

        fn visit_str<E>(
            self,
            v: &str,
        ) -> Result<Self::Value, E>
        where
            E: serde::de::Error, {
            Path::parse(v).map_err(|err| serde::de::Error::custom(format!("{err}")))
        }
    }

    impl<'de> serde::Deserialize<'de> for Path {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>, {
            deserializer.deserialize_str(Visitor)
        }
    }
}
