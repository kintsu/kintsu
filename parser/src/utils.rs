use crate::{SpannedToken, ctx::NamedItemContext};
use std::collections::BTreeMap;

#[allow(unused)]
pub fn insert_unique_ident<V>(
    ns: SpannedToken![ident],
    tbl: &mut BTreeMap<NamedItemContext, V>,
    def: NamedItemContext,
    tag: &'static str,
    value: V,
) -> crate::Result<()> {
    match tbl.insert(def.clone(), value) {
        Some(..) => Err(crate::Error::conflict(ns, def, tag)),
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
