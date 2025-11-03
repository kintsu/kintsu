use std::fmt;

use crate::{
    SpannedToken,
    ast::{path::Path, ty::PathOrIdent},
    defs::Spanned,
    tokens::{PathToken, ToTokens},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RefContext {
    pub package: String,
    pub namespace: Vec<String>,
}

impl fmt::Display for RefContext {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}::{}", self.package, self.namespace.join("::"))?;
        Ok(())
    }
}

impl ToTokens for RefContext {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        self.path().write(tt);
    }
}

impl RefContext {
    pub fn new(
        package: String,
        namespace: Vec<String>,
    ) -> Self {
        Self { package, namespace }
    }

    pub fn enter(
        &self,
        namespace: &str,
    ) -> Self {
        Self {
            package: self.package.clone(),
            namespace: {
                let mut ns = self.namespace.clone();
                ns.push(namespace.to_string());
                ns
            },
        }
    }

    pub fn item(
        &self,
        name: SpannedToken![ident],
    ) -> NamedItemContext {
        NamedItemContext::new(name, self.clone())
    }

    pub fn extend(
        &self,
        ns: &[String],
    ) -> Self {
        let mut new_ns = self.namespace.clone();
        new_ns.extend_from_slice(ns);
        Self {
            package: self.package.clone(),
            namespace: new_ns,
        }
    }

    pub fn merge_extend(
        &self,
        ns: &[String],
    ) -> Self {
        let mut new_ns = self.namespace.clone();

        if let Some(last) = new_ns.last()
            && let Some(first_other) = ns.first()
            && last == first_other
        {
            new_ns.pop();
        }

        new_ns.extend_from_slice(ns);

        Self {
            package: self.package.clone(),
            namespace: new_ns,
        }
    }

    fn path(&self) -> PathOrIdent {
        PathOrIdent::Path(Spanned::call_site(PathToken::new(Path::Ambiguous {
            bits: {
                let mut bits = vec![self.package.clone()];
                bits.extend(self.namespace.clone());
                bits
            },
        })))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NamedItemContext {
    pub context: RefContext,
    pub name: SpannedToken![ident],
}

impl NamedItemContext {
    pub fn new(
        name: SpannedToken![ident],
        context: RefContext,
    ) -> Self {
        Self { name, context }
    }

    pub fn path(&self) -> PathOrIdent {
        let span = self.name.span();
        PathOrIdent::Path(Spanned::new(
            span.start,
            span.end,
            PathToken::new(Path::Ambiguous {
                bits: {
                    let mut bits = vec![self.context.package.clone()];

                    bits.extend(self.context.namespace.clone());
                    bits.push(self.name.borrow_string().clone());

                    bits
                },
            }),
        ))
    }
}

impl ToTokens for NamedItemContext {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.path())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RefOrItemContext {
    Ref(RefContext),
    Item(NamedItemContext),
}

impl ToTokens for RefOrItemContext {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            Self::Ref(r) => tt.write(r),
            Self::Item(i) => tt.write(i),
        }
    }
}

impl From<RefContext> for RefOrItemContext {
    fn from(ctx: RefContext) -> Self {
        RefOrItemContext::Ref(ctx)
    }
}

impl From<NamedItemContext> for RefOrItemContext {
    fn from(value: NamedItemContext) -> Self {
        RefOrItemContext::Item(value)
    }
}

impl RefOrItemContext {
    pub fn new(ctx: impl Into<Self>) -> Self {
        ctx.into()
    }

    pub fn as_ref_context(&self) -> &RefContext {
        match self {
            RefOrItemContext::Ref(ctx) => ctx,
            RefOrItemContext::Item(item) => &item.context,
        }
    }
}
