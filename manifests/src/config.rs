use std::path::{Path, PathBuf};

use config::File;
use serde::{Serialize, de::DeserializeOwned};
use validator::Validate;

use crate::Error;

pub trait NewForNamed
where
    Self: Sized + Serialize + DeserializeOwned + Validate, {
    const NAME: &str;

    fn path<S: AsRef<Path>>(root_dir: S) -> PathBuf {
        root_dir.as_ref().join(Self::NAME)
    }

    fn new<S: AsRef<Path>>(
        f: &dyn kintsu_fs::FileSystem,
        root_dir: S,
    ) -> crate::Result<Self> {
        let data = f.read_to_string_sync(&Self::path(root_dir))?;
        let this: Self = toml::from_str(&data)?;
        this.validate()?;
        Ok(this)
    }

    fn new_for_opt<S: AsRef<Path>>(
        f: &dyn kintsu_fs::FileSystem,
        root_dir: S,
    ) -> crate::Result<Option<Self>> {
        Ok(if !f.exists_sync(&Self::path(&root_dir)) {
            None
        } else {
            Some(Self::new(f, root_dir)?)
        })
    }

    fn dump<S: AsRef<Path>>(&self) -> crate::Result<String> {
        Ok(toml::to_string(self)?)
    }
}

pub trait NewForConfig
where
    Self: Sized + DeserializeOwned + Validate, {
    const NAME: &'static str;
    fn new<S: AsRef<str>>(dir: Option<S>) -> crate::Result<Self> {
        let file_name = format!(
            "{}",
            PathBuf::from(
                dir.map(|s| String::from(s.as_ref()))
                    .unwrap_or("./".into())
            )
            .join(Self::NAME)
            .display()
        );

        let this: Self = config::ConfigBuilder::<config::builder::DefaultState>::default()
            .add_source(File::with_name(&file_name).required(false))
            .add_source(config::Environment::default().prefix("OP"))
            .build()
            .map_err(Error::from_with_source_init(file_name.clone()))?
            .try_deserialize()
            .map_err(Error::from_with_source_init(file_name.clone()))?;

        this.validate()
            .map_err(Error::from_with_source_init(file_name.clone()))?;

        Ok(this)
    }
}
