use std::{collections::BTreeSet, path::PathBuf};

pub fn match_paths(
    include: &[String],
    exclude: &[String],
) -> crate::Result<Vec<PathBuf>> {
    let mut found = BTreeSet::new();
    for include in include {
        for p in glob::glob(include)? {
            found.insert(p?);
        }
    }

    let mut match_exp = vec![];
    for exclude in exclude {
        match_exp.push(glob::Pattern::new(exclude)?);
    }

    let mut out: Vec<PathBuf> = found
        .into_iter()
        .filter(|it| {
            let remove = match_exp
                .iter()
                .any(|re| re.matches_path(it));

            if remove {
                tracing::trace!(
                    "removing '{}' from targets due to exclusion rule",
                    it.display()
                )
            }

            !remove
        })
        .inspect(|it| {
            tracing::trace!("including '{}' in targets", it.display());
        })
        .collect();

    out.sort();

    Ok(out)
}
