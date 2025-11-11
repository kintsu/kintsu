use crate::PackageStorage;
use std::sync::Arc;

#[derive(Clone)]
pub struct StorageManager<D> {
    storage: Arc<dyn PackageStorage<D>>,
}

impl<D> StorageManager<D> {
    pub fn new(storage: Arc<dyn PackageStorage<D>>) -> Self {
        Self { storage }
    }

    pub fn storage(&self) -> Arc<dyn PackageStorage<D>> {
        Arc::clone(&self.storage)
    }
}

impl<D> std::ops::Deref for StorageManager<D> {
    type Target = Arc<dyn PackageStorage<D>>;

    fn deref(&self) -> &Self::Target {
        &self.storage
    }
}

impl<D> AsRef<Arc<dyn PackageStorage<D>>> for StorageManager<D> {
    fn as_ref(&self) -> &Arc<dyn PackageStorage<D>> {
        &self.storage
    }
}

impl<D: Send + Sync + serde::Serialize + serde::de::DeserializeOwned> crate::PackageStorage<D>
    for StorageManager<D>
{
    fn put_source<'d>(
        &'d self,
        path: &'d str,
        data: &'d kintsu_fs::memory::MemoryFileSystem,
    ) -> crate::LocalFuture<'d, crate::Checksum> {
        self.storage.put_source(path, data)
    }

    fn put_declarations<'d>(
        &'d self,
        path: &'d str,
        data: &'d D,
    ) -> crate::LocalFuture<'d, crate::Checksum> {
        self.storage.put_declarations(path, data)
    }

    fn get_declarations<'d>(
        &'d self,
        path: &'d str,
        checksum: crate::Checksum,
    ) -> crate::LocalFuture<'d, D> {
        self.storage.get_declarations(path, checksum)
    }

    fn get_source<'d>(
        &'d self,
        path: &'d str,
        checksum: crate::Checksum,
    ) -> crate::LocalFuture<'d, kintsu_fs::memory::MemoryFileSystem> {
        self.storage.get_source(path, checksum)
    }
}
