use std::path::PathBuf;

use crate::generate::Source;

pub fn walk(source: &Source) -> super::Result<Vec<PathBuf>> {
    Ok(kintsu_fs::match_paths::match_paths(
        &source.include,
        &source.exclude,
    )?)
}
