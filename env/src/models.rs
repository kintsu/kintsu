use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::registry)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Registry {
    pub name: String,
    pub url: String,
}

// #[derive(Queryable, Selectable)]
// #[diesel(table_name = crate::schema::package)]
// #[diesel(check_for_backend(diesel::sqlite::Sqlite))]
// pub struct Package {
//     registry: String,
//     name: String,
//     version: kintsu_manifests::version::Version,
//     content_hash: String,
//     content: kintsu_fs::memory::MemoryFileSystem,
// }
