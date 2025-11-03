use std::{collections::BTreeMap, sync::Arc};

use kintsu_manifests::version::Version;
use tokio::sync::MutexGuard;

use crate::ctx::SchemaCtx;

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CacheKey {
    pub package_name: String,
    pub version: Version,
    pub content_hash: Option<String>,
}

impl std::fmt::Debug for CacheKey {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "CacheKey({})", self)
    }
}

impl std::fmt::Display for CacheKey {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{}@{}{}",
            self.package_name,
            self.version,
            self.content_hash
                .as_deref()
                .map(|h| format!(":{}", h))
                .unwrap_or("".into())
        )
    }
}

impl CacheKey {
    pub fn new(
        package_name: String,
        version: Version,
        content_hash: Option<String>,
    ) -> Self {
        Self {
            package_name,
            version,
            content_hash,
        }
    }
}

#[derive(Clone)]
pub struct CachedSchema {
    pub schema: Arc<SchemaCtx>,
    pub version: Version,
}

impl CachedSchema {
    pub fn new(
        schema: Arc<SchemaCtx>,
        version: Version,
    ) -> Self {
        Self { schema, version }
    }
}

#[derive(Clone)]
pub struct SchemaCache {
    inner: Arc<tokio::sync::Mutex<BTreeMap<CacheKey, CachedSchema>>>,
}

impl Default for SchemaCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaCache {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub async fn with_read<F: Fn(&MutexGuard<BTreeMap<CacheKey, CachedSchema>>) -> R, R>(
        &self,
        f: F,
    ) -> R {
        f(&self.inner.lock().await)
    }

    pub async fn entry_count(&self) -> usize {
        self.with_read(|inner| inner.len()).await
    }

    pub async fn size_shallow(&self) -> usize {
        const SIZE: usize = std::mem::size_of::<CachedSchema>() + std::mem::size_of::<CacheKey>();
        self.entry_count().await * SIZE
    }

    pub async fn size_deep(&self) -> usize {
        const SIZE: usize = std::mem::size_of::<SchemaCtx>()
            + std::mem::size_of::<Version>()
            + std::mem::size_of::<CacheKey>();
        self.entry_count().await * SIZE
    }

    pub async fn insert(
        &self,
        key: CacheKey,
        schema: CachedSchema,
    ) {
        let mut inner = self.inner.lock().await;
        inner.insert(key, schema);
    }

    pub async fn get(
        &self,
        key: &CacheKey,
    ) -> Option<CachedSchema> {
        let inner = self.inner.lock().await;
        inner
            .get(key)
            .cloned()
            .inspect(|_| {
                tracing::trace!("cache hit for key: {}", key);
            })
            .or_else(|| {
                tracing::trace!("cache miss for key: {}", key);
                None
            })
    }
}
