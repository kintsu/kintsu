use crate::{SpannedToken, ctx::NamedItemContext};
use std::collections::BTreeMap;

#[allow(unused)]
pub fn insert_unique_ident<V>(
    ns: SpannedToken![ident],
    tbl: &mut BTreeMap<NamedItemContext, V>,
    def: NamedItemContext,
    tag: &'static str,
    value: V,
    span: Option<kintsu_errors::Span>,
) -> crate::Result<()> {
    match tbl.insert(def.clone(), value) {
        Some(..) => {
            let builder = crate::TypeDefError::ident_conflict(
                ns.borrow_string(),
                def.name.borrow_string(),
                tag,
            );
            let err = match span {
                Some(s) => builder.at(s).build(),
                None => builder.unlocated().build(),
            };
            Err(crate::Error::Compiler(err))
        },
        None => Ok(()),
    }
}

pub fn guard_schema(
    pkg: &str,
    leading_path: &str,
) -> String {
    if leading_path == "schema" {
        pkg.to_string()
    } else {
        leading_path.to_string()
    }
}
