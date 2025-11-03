use std::path::PathBuf;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ManagerConfig {
    pub(crate) home: PathBuf,
}
