use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    sync::Arc,
};

use kintsu_manifests::{
    lock::{LockedSource, Lockfile},
    version::Version,
};

use crate::ctx::SchemaCtx;

#[derive(Clone)]
pub struct ResolvedMetadata {
    pub version: Version,
    pub source: LockedSource,
    pub checksum: String,
    /// exported namespaces/types
    pub provides: BTreeSet<String>,
    /// direct dependencies of this package
    pub dependencies: Vec<String>,
}

/// Shared state for parallel compilation
pub struct SharedCompilationState {
    /// Loaded dependency schemas: package name -> schema
    pub dependencies: BTreeMap<String, Arc<SchemaCtx>>,

    /// Track which dependencies are currently being processed
    pub processing_set: HashSet<String>,

    /// Track loaded versions for compatibility checking
    pub loaded_versions: BTreeMap<String, Version>,

    /// Lockfile being built during compilation
    pub lockfile: Option<Lockfile>,

    /// Track if lockfile validation failed and we need to rebuild
    pub lockfile_invalidated: bool,

    /// Track resolved dependency metadata for lockfile generation
    /// Map: package_name -> (version, source, checksum, provides)
    pub resolved_metadata: BTreeMap<String, ResolvedMetadata>,
}

impl SharedCompilationState {
    pub fn new() -> Self {
        Self {
            dependencies: BTreeMap::new(),
            processing_set: HashSet::new(),
            loaded_versions: BTreeMap::new(),
            lockfile: None,
            lockfile_invalidated: false,
            resolved_metadata: BTreeMap::new(),
        }
    }
}

impl Default for SharedCompilationState {
    fn default() -> Self {
        Self::new()
    }
}
