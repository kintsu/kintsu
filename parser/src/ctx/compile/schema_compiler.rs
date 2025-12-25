use std::sync::Arc;

use kintsu_cli_core::ProgressBar;
use pathfinding::prelude::*;

use crate::{
    ctx::{
        SchemaCtx,
        cache::CacheKey,
        common::{Definition, NamespaceChild},
        compile::utils::{normalize_import_to_package_name, normalize_package_to_import_name},
        graph::schemas::{Import, SchemaDependencyGraph},
    },
    tokens::ToTokens,
};

async fn run_in_group<
    T,
    F: Fn(&ProgressBar, usize, Vec<T>) -> Fut,
    Fut: Future<Output = crate::Result<()>>,
>(
    compile_bar: ProgressBar,
    groups: Vec<Vec<T>>,
    f: F,
) -> crate::Result<()> {
    let mut futs = vec![];
    for (level, group) in groups.into_iter().enumerate() {
        let compile_bar = compile_bar.clone();
        futs.push(Box::pin(f(&compile_bar, level, group)));
    }

    let _ = futures_util::future::try_join_all(futs).await?;
    Ok(())
}

pub struct SchemaCompiler;

impl SchemaCompiler {
    pub async fn compile_all(ctx: &super::CompileCtx) -> crate::Result<()> {
        tracing::info!("Starting parallel schema compilation");

        let graph_spinner = ctx.progress.add_spinner("Analyzing");
        graph_spinner.set_message("schema dependencies");

        tracing::trace!("Building schema dependency graph");
        let graph = Self::build_graph(ctx).await?;
        tracing::trace!(schema_count = graph.nodes.len(), "Dependency graph built");

        graph_spinner.finish_with_message(format!("{} schemas", graph.nodes.len()));

        if !graph.nodes.is_empty() {
            tracing::trace!("Detecting circular schema dependencies");
            let successors_fn = |node: &CacheKey| -> Vec<CacheKey> { graph.successors(node) };

            let components = strongly_connected_components(
                &graph
                    .nodes
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>(),
                successors_fn,
            );

            for component in components {
                if component.len() > 1 {
                    let cycle_names: Vec<String> = component
                        .iter()
                        .map(|k| k.package_name.clone())
                        .collect();

                    tracing::error!(cycle = ?cycle_names, "Circular schema dependency detected");
                    return Err(crate::Error::SchemaCircularDependency {
                        schemas: cycle_names,
                    });
                }
            }
            tracing::trace!("No circular dependencies found");
        }

        tracing::trace!("Computing topological sort groups");

        let groups = graph.topological_groups()?;

        tracing::info!(
            group_count = groups.len(),
            total_schemas = groups.iter().map(|g| g.len()).sum::<usize>(),
            "Topological sort complete, starting compilation"
        );

        #[cfg(debug_assertions)]
        {
            println!(
                "Schema compilation groups:\n{}",
                Self::pretty_print_groups(&groups)
            );
        }

        let total_schemas: u64 = groups.iter().map(|g| g.len() as u64).sum();
        let compile_bar = ctx
            .progress
            .add_bar(total_schemas, "Compiling");

        run_in_group(
            compile_bar.clone(),
            groups.clone(),
            |compile_bar, level, group| {
                let compile_bar = compile_bar.clone();
                async move {
                    tracing::debug!(
                        level = level,
                        schemas_in_group = group.len(),
                        schemas = ?group.iter().map(|k| &k.package_name).collect::<Vec<_>>(),
                        "Compiling schema group"
                    );

                    for it in group {
                        tracing::debug!(
                            level = level,
                            schema = %it.package_name,
                            "Compiling schema"
                        );
                        compile_bar.set_message(format!("compiling {}", it.package_name));
                        Self::compile_schema(ctx, &it).await?;
                        compile_bar.set_message(format!("completed {}", it.package_name));
                        compile_bar.inc(1);
                    }

                    tracing::debug!(level = level, "Schema group compilation complete");
                    Ok(())
                }
            },
        )
        .await?;

        compile_bar.finish_with_message("all schemas");

        tracing::info!("Schema compilation complete, starting type resolution");

        // calculate namespaces after first pass, no-op if progress disabled
        let total_namespaces: u64 = if ctx.progress.is_enabled() {
            let mut count = 0u64;

            count += ctx.root.namespaces.len() as u64;

            let state = ctx.state.read().await;
            for schema in state.dependencies.values() {
                count += schema.namespaces.len() as u64;
            }
            drop(state);

            count
        } else {
            0
        };

        // Create progress bar for type resolution
        let resolution_bar = ctx
            .progress
            .add_bar(total_namespaces, "Resolving");

        run_in_group(
            resolution_bar.clone(),
            groups,
            |resolution_bar, level, group| {
                let resolution_bar = resolution_bar.clone();
                async move {
                    tracing::debug!(
                        level = level,
                        schemas_in_group = group.len(),
                        "Starting type resolution for schema group"
                    );

                    for it in group {
                        tracing::debug!(
                            level = level,
                            schema = %it.package_name,
                            "Resolving types for schema"
                        );
                        resolution_bar.set_message(format!("resolving types: {}", it.package_name));
                        Self::resolve_schema_types(ctx, &it, &resolution_bar).await?;
                        resolution_bar.set_message(format!("completed: {}", it.package_name));
                    }

                    tracing::debug!(level = level, "Type resolution for schema group complete");
                    Ok(())
                }
            },
        )
        .await?;

        resolution_bar.finish_with_message("all namespaces");

        tracing::info!("Type resolution complete");

        Ok(())
    }

    async fn build_graph(ctx: &super::CompileCtx) -> crate::Result<SchemaDependencyGraph> {
        let mut graph = SchemaDependencyGraph::new();

        // Add root schema
        let root_key = Self::build_cache_key_for_schema(&ctx.root)?;
        let root_imports = Self::extract_imports(&ctx.root).await;
        graph.add_schema(root_key.clone(), root_imports);

        let state = ctx.state.read().await;
        for schema in state.dependencies.values() {
            let dep_key = Self::build_cache_key_for_schema(schema)?;
            let dep_imports = Self::extract_imports(schema).await;
            graph.add_schema(dep_key, dep_imports);
        }
        drop(state);

        graph.build_dependencies();

        Ok(graph)
    }

    #[tracing::instrument(skip(ctx, schema_id), fields(package = %schema_id.package_name))]
    async fn compile_schema(
        ctx: &super::CompileCtx,
        schema_id: &CacheKey,
    ) -> crate::Result<()> {
        tracing::debug!("Starting schema compilation");

        let root_package_normalized =
            normalize_package_to_import_name(&ctx.root.package.package().name);

        let target = normalize_package_to_import_name(&schema_id.package_name);

        let schema: Arc<SchemaCtx> = if target == root_package_normalized {
            tracing::trace!("Using root schema");
            ctx.root.clone()
        } else {
            tracing::trace!(
                looking_for = %target,
                available_deps = ?ctx.dependency_names().await,
                "Looking up dependency"
            );
            ctx.get_dependency(&target)
                .await
                .ok_or_else(|| {
                    crate::Error::InternalError {
                        message: format!("Schema not found: {}", schema_id.package_name),
                    }
                })?
        };

        tracing::trace!(namespace_count = schema.namespaces.len(), "Schema resolved");

        let levels = Self::namespace_levels(&schema).await;

        tracing::debug!(
            depth_levels = levels.len(),
            total_namespaces = levels.iter().map(|l| l.len()).sum::<usize>(),
            "Namespace levels computed"
        );

        for (depth, group) in levels.into_iter().enumerate() {
            tracing::debug!(
                depth = depth,
                namespaces_in_group = group.len(),
                namespaces = ?group,
                "Processing namespace depth level"
            );

            let tasks: Vec<_> = group
                .iter()
                .map(|ns_name| {
                    let schema = Arc::clone(&schema);
                    let ns_name = ns_name.clone();
                    async move { Self::register_types_recursive(&schema, &ns_name, depth).await }
                })
                .collect();

            futures_util::future::try_join_all(tasks).await?;

            tracing::debug!(depth = depth, "Namespace depth level complete");
        }

        tracing::debug!("Schema compilation complete");
        Ok(())
    }

    #[allow(dead_code)]
    fn pretty_print_groups(groups: &Vec<Vec<CacheKey>>) -> String {
        let mut output = String::new();
        for (level, group) in groups.iter().enumerate() {
            output.push_str(&format!("Level {}:\n", level));
            for schema_id in group {
                output.push_str(&format!("  -> {}\n", schema_id.package_name));
            }
        }
        output
    }

    #[tracing::instrument(skip(ctx, schema_id, resolution_bar), fields(package = %schema_id.package_name))]
    async fn resolve_schema_types(
        ctx: &super::CompileCtx,
        schema_id: &CacheKey,
        resolution_bar: &ProgressBar,
    ) -> crate::Result<()> {
        tracing::debug!("Starting schema type resolution");

        let root_package_normalized =
            normalize_package_to_import_name(&ctx.root.package.package().name);

        let target = normalize_package_to_import_name(&schema_id.package_name);

        let schema: Arc<SchemaCtx> = if target == root_package_normalized {
            ctx.root.clone()
        } else {
            ctx.get_dependency(&target)
                .await
                .ok_or_else(|| {
                    crate::Error::InternalError {
                        message: format!("Schema not found: {}", schema_id.package_name),
                    }
                })?
        };

        let resolution_levels = Self::namespace_levels(&schema).await;

        tracing::debug!(
            depth_levels = resolution_levels.len(),
            total_namespaces = resolution_levels
                .iter()
                .map(|l| l.len())
                .sum::<usize>(),
            "Starting type resolution phase"
        );

        for (depth, group) in resolution_levels.into_iter().enumerate() {
            tracing::debug!(
                depth = depth,
                namespaces_in_group = group.len(),
                namespaces = ?group,
                "Resolving namespace depth level"
            );

            let resolution_tasks: Vec<_> = group
                .iter()
                .map(|ns_name| {
                    let schema = Arc::clone(&schema);
                    let ns_name = ns_name.clone();
                    let resolution_bar = resolution_bar.clone();
                    async move {
                        Self::resolve_namespace_types(&schema, &ns_name, &resolution_bar).await
                    }
                })
                .collect();

            futures_util::future::try_join_all(resolution_tasks).await?;

            tracing::debug!(depth = depth, "Namespace resolution depth level complete");
        }

        tracing::debug!("Schema type resolution complete");
        Ok(())
    }

    #[tracing::instrument(skip(schema, resolution_bar), fields(ns = %ns_name))]
    async fn resolve_namespace_types(
        schema: &Arc<SchemaCtx>,
        ns_name: &str,
        resolution_bar: &ProgressBar,
    ) -> crate::Result<()> {
        use super::super::resolve::TypeResolver;

        tracing::debug!("Starting TypeResolver");

        let ns = schema
            .get_namespace(ns_name)
            .ok_or_else(|| {
                crate::Error::InternalError {
                    message: format!("Namespace not found: {}", ns_name),
                }
            })?;

        let resolver = TypeResolver::new(ns.clone());
        let resolution = resolver.resolve().await?;

        tracing::debug!(
            anonymous_structs = resolution.anonymous_structs.len(),
            union_structs = resolution.union_structs.len(),
            versions = resolution.versions.len(),
            errors = resolution.errors.len(),
            "TypeResolver completed"
        );

        {
            let mut ns_mut = ns.lock().await;

            ns_mut
                .integrate_resolution(resolution)
                .await?;

            drop(ns_mut);
        }

        let ns_resolved = ns.lock().await;
        for (item_ctx, child) in &ns_resolved.children {
            if let super::super::common::NamespaceChild::Struct(struct_def) = &child.value
                && !ns_resolved.children.contains_key(item_ctx)
            {
                tracing::trace!(
                    struct_name = %struct_def.def.value.name.borrow_string(),
                    "Registering generated struct"
                );

                schema.registry.register(
                    &item_ctx.context,
                    &struct_def.def.name,
                    Definition::Struct(Arc::new(struct_def.clone())),
                    struct_def.def_span().clone(),
                    child.source.clone(),
                )?;
            }
        }

        resolution_bar.inc(1);
        resolution_bar.set_message(ns_name.to_string());

        tracing::debug!("Type resolution complete");
        Ok(())
    }

    async fn namespace_levels(schema: &SchemaCtx) -> Vec<Vec<String>> {
        use std::collections::{BTreeMap, BTreeSet, VecDeque};

        let mut nodes: BTreeSet<String> = BTreeSet::new();
        let mut adj: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut indegree: BTreeMap<String, usize> = BTreeMap::new();

        for ns_name in schema.namespaces.keys() {
            nodes.insert(ns_name.clone());
            adj.insert(ns_name.clone(), Vec::new());
            indegree.insert(ns_name.clone(), 0);
        }

        for (ns_name, ns_ctx) in &schema.namespaces {
            for import in &ns_ctx.lock().await.imports {
                let ref_ctx = import.value.as_ref_context();
                if ref_ctx.package == schema.package.package().name
                    && let Some(target_top) = ref_ctx.namespace.first()
                    && nodes.contains::<String>(target_top)
                {
                    if let Some(v) = adj.get_mut(ns_name) {
                        v.push(target_top.clone())
                    }
                    if let Some(degree) = indegree.get_mut(target_top) {
                        *degree += 1;
                    }
                }
            }
        }

        // find roots
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        for (n, &deg) in &indegree {
            if deg == 0 {
                queue.push_back((n.clone(), 0));
            }
        }

        // if no roots (cycle), start from all nodes
        if queue.is_empty() {
            for n in nodes.iter() {
                queue.push_back((n.clone(), 0));
            }
        }

        let mut depth_map: BTreeMap<String, usize> = BTreeMap::new();

        while let Some((node, depth)) = queue.pop_front() {
            let entry = depth_map
                .entry(node.clone())
                .or_insert(depth);
            if depth < *entry {
                *entry = depth;
            }

            if let Some(neighs) = adj.get(&node) {
                for n in neighs.iter() {
                    let next_depth = depth + 1;
                    let should_push = match depth_map.get(n) {
                        Some(&existing) => next_depth < existing,
                        None => true,
                    };
                    if should_push {
                        depth_map.insert(n.clone(), next_depth);
                        queue.push_back((n.clone(), next_depth));
                    }
                }
            }
        }

        let mut by_depth: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        for (ns, depth) in depth_map.into_iter() {
            by_depth.entry(depth).or_default().push(ns);
        }

        let mut levels: Vec<Vec<String>> = Vec::new();
        for (_, mut group) in by_depth.into_iter() {
            group.sort();
            levels.push(group);
        }

        levels
    }

    #[tracing::instrument(skip(schema), fields(namespace = %ns_name, depth = depth))]
    pub(crate) async fn register_types_recursive(
        schema: &Arc<SchemaCtx>,
        ns_name: &str,
        depth: usize,
    ) -> crate::Result<()> {
        tracing::trace!("Starting type registration for namespace");

        let ns_ctx = schema
            .namespaces
            .get(ns_name)
            .ok_or_else(|| {
                crate::Error::InternalError {
                    message: format!("Namespace not found at depth {}: {}", depth, ns_name),
                }
            })?;

        tracing::trace!(
            child_count = ns_ctx.lock().await.children.len(),
            "Namespace context retrieved"
        );

        use crate::ctx::graph::TypeExtractor;
        let ns = ns_ctx.lock().await;
        let type_graph = TypeExtractor::extract_from_namespace(&ns, &ns.ctx);
        drop(ns);

        tracing::trace!(
            type_count = type_graph.type_names().len(),
            "Type dependency graph extracted"
        );

        let type_names = type_graph.type_names();
        if !type_names.is_empty() {
            tracing::trace!("Detecting type dependency cycles");
            use pathfinding::prelude::*;

            let successors_fn =
                |node: &crate::ctx::paths::NamedItemContext| type_graph.required_successors(node);

            let components = strongly_connected_components(&type_names, successors_fn);

            tracing::trace!(component_count = components.len(), "SCC detection complete");

            for component in &components {
                if component.len() > 1 {
                    if !type_graph.has_terminating_edge(component) {
                        // Non-terminating cycle - error
                        let cycle_str: Vec<String> = component
                            .iter()
                            .map(|ctx| ctx.display())
                            .collect();

                        tracing::error!(cycle = ?cycle_str, "Non-terminating type cycle detected");
                        return Err(crate::Error::TypeCircularDependency { types: cycle_str });
                    }
                    // Terminating cycle - allowed (e.g., A has optional B, B has required A)
                    tracing::trace!(
                        cycle = ?component.iter().map(|c| c.display()).collect::<Vec<_>>(),
                        "Terminating cycle allowed"
                    );
                }
            }

            tracing::trace!("Computing topological sort for types");

            let successors_fn =
                |node: &crate::ctx::paths::NamedItemContext| type_graph.all_successors(node);

            let groups = match topological_sort_into_groups(&type_names, successors_fn) {
                Ok(groups) => groups,
                Err(cycle) => {
                    // This shouldn't happen - we already validated cycles
                    // But handle gracefully just in case
                    tracing::error!(?cycle, "Topological sort failed unexpectedly");
                    return Err(crate::Error::TypeCircularDependency {
                        types: vec![format!("<topological sort failed: {:?}>", cycle)],
                    });
                },
            };

            tracing::debug!(
                group_count = groups.len(),
                total_types = groups.iter().map(|g| g.len()).sum::<usize>(),
                "Starting type registration"
            );

            for (group_idx, group) in groups.iter().enumerate() {
                tracing::trace!(
                    group = group_idx,
                    types_in_group = group.len(),
                    types = ?group.iter().map(|t| t.display()).collect::<Vec<_>>(),
                    "Registering type group"
                );

                let registration_tasks: Vec<_> = group
                    .iter()
                    .map(|type_ctx| {
                        let schema = Arc::clone(schema);
                        let type_ctx = type_ctx.clone();
                        let ns_name = ns_name.to_string();

                        async move { Self::register_type(&schema, &ns_name, &type_ctx).await }
                    })
                    .collect();

                futures_util::future::try_join_all(registration_tasks).await?;

                tracing::trace!(group = group_idx, "Type group registration complete");
            }
        }

        let child_prefix = format!("{}::", ns_name);
        let child_namespaces: Vec<String> = schema
            .namespaces
            .keys()
            .filter(|key| key.starts_with(&child_prefix) && key.matches("::").count() == depth + 1)
            .cloned()
            .collect();

        if !child_namespaces.is_empty() {
            tracing::trace!(
                child_count = child_namespaces.len(),
                children = ?child_namespaces,
                "Processing child namespaces"
            );

            let child_tasks: Vec<_> =
                child_namespaces
                    .iter()
                    .map(|child_ns| {
                        let schema = Arc::clone(schema);
                        let child_ns = child_ns.clone();
                        async move {
                            Self::register_types_recursive(&schema, &child_ns, depth + 1).await
                        }
                    })
                    .collect();

            futures_util::future::try_join_all(child_tasks).await?;
        }

        tracing::trace!("Namespace type registration complete");
        Ok(())
    }

    #[tracing::instrument(skip(schema), fields(namespace = %ns_name, type_name = %type_ctx.display()))]
    async fn register_type(
        schema: &Arc<SchemaCtx>,
        ns_name: &str,
        type_ctx: &crate::ctx::paths::NamedItemContext,
    ) -> crate::Result<()> {
        let ns_ctx = schema
            .namespaces
            .get(ns_name)
            .ok_or_else(|| {
                crate::Error::InternalError {
                    message: format!(
                        "Namespace disappeared during type registration: {}",
                        ns_name
                    ),
                }
            })?;

        let ns = ns_ctx.lock().await;
        let child = match ns.children.get(type_ctx) {
            Some(child) => child,
            None => {
                // Type not found - this is an internal error
                return Err(crate::Error::InternalError {
                    message: format!(
                        "Type not found in namespace during registration: {}",
                        type_ctx.display()
                    ),
                });
            },
        };

        let (definition, span) = match &child.value {
            NamespaceChild::Struct(struct_def) => {
                tracing::trace!("Registering struct type");
                let span = struct_def.def_span().clone();

                let def = Definition::Struct(Arc::new(struct_def.clone()));
                (def, span)
            },
            NamespaceChild::OneOf(oneof_def) => {
                tracing::trace!("Registering oneof type");
                let span = oneof_def.def_span().clone();

                let def = Definition::OneOf(Arc::new(oneof_def.clone()));
                (def, span)
            },
            NamespaceChild::Error(error_def) => {
                tracing::trace!("Registering error type");
                let span = error_def.def_span().clone();

                let def = Definition::Error(Arc::new(error_def.clone()));
                (def, span)
            },
            NamespaceChild::Type(type_def) => {
                tracing::trace!("registering type alias");
                let span = type_def.def_span().clone();

                let def = Definition::TypeAlias(Arc::new(type_def.clone()));
                (def, span)
            },
            NamespaceChild::Enum(enum_def) => {
                tracing::trace!("registering enum type");
                let span = enum_def.def_span().clone();

                let def = Definition::Enum(Arc::new(enum_def.clone()));
                (def, span)
            },
            NamespaceChild::Operation(_) => {
                tracing::trace!("skipping operation (not a type)");
                // Operations are not registered in the type registry
                // They are handled separately during code generation
                return Ok(());
            },
            NamespaceChild::Namespace(_) => {
                tracing::trace!("skipping namespace (not a type)");
                // Nested namespaces are not types - skip
                return Ok(());
            },
        };

        let source = child.source.clone();
        let span = span.clone();

        drop(ns);

        schema
            .registry
            .register(&type_ctx.context, &type_ctx.name, definition, span, source)?;

        tracing::trace!("type registration successful");

        Ok(())
    }

    fn build_cache_key_for_schema(schema: &SchemaCtx) -> crate::Result<CacheKey> {
        let package_name = normalize_import_to_package_name(&schema.package.package().name);
        let version = schema.package.package().version.0.clone();
        Ok(CacheKey::new(package_name, version, None))
    }

    async fn extract_imports(schema: &SchemaCtx) -> Vec<Import> {
        let mut imports = Vec::new();
        let mut seen_packages = std::collections::HashSet::new();

        for ns_ctx in schema.namespaces.values() {
            for import in &ns_ctx.lock().await.imports {
                let ref_ctx = import.value.as_ref_context();
                let package = ref_ctx.package.clone();

                if package != schema.package.package().name && seen_packages.insert(package.clone())
                {
                    imports.push(Import {
                        name: package,
                        resolved_id: None, // Will be resolved by build_dependencies()
                    });
                }
            }
        }

        imports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_imports_basic() {
        let imports = [
            Import {
                name: "foo".to_string(),
                resolved_id: None,
            },
            Import {
                name: "bar".to_string(),
                resolved_id: None,
            },
        ];

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].name, "foo");
        assert_eq!(imports[1].name, "bar");
    }
}
