use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use crate::ctx::cache::CacheKey;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Import {
    pub name: String,
    pub resolved_id: Option<CacheKey>,
}

#[derive(Debug, Clone)]
pub struct SchemaNode {
    // todo: remove dead code warning when used
    #[allow(dead_code)]
    pub id: CacheKey,
    pub imports: Vec<Import>,
    pub depends_on: Vec<CacheKey>,
}

#[derive(Debug, Clone, Default)]
pub struct SchemaDependencyGraph {
    pub(crate) nodes: BTreeMap<CacheKey, SchemaNode>,
}

impl SchemaDependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }

    pub fn add_schema(
        &mut self,
        id: CacheKey,
        imports: Vec<Import>,
    ) {
        self.nodes.insert(
            id.clone(),
            SchemaNode {
                id,
                imports,
                depends_on: Vec::new(),
            },
        );
    }

    pub fn build_dependencies(&mut self) {
        let mut name_to_id: BTreeMap<String, CacheKey> = BTreeMap::new();

        for schema_id in self.nodes.keys() {
            name_to_id.insert(schema_id.package_name.clone(), schema_id.clone());
        }

        let schema_ids: Vec<_> = self.nodes.keys().cloned().collect();

        for schema_id in schema_ids {
            let imports = self.nodes[&schema_id].imports.clone();
            let mut depends_on = Vec::new();

            for import in imports {
                if let Some(resolved_id) = import.resolved_id {
                    if resolved_id != schema_id {
                        depends_on.push(resolved_id);
                    }
                    continue;
                }

                // try to find by package name
                if let Some(dep_id) = name_to_id.get(&import.name)
                    && dep_id != &schema_id
                {
                    depends_on.push(dep_id.clone());
                }
            }

            if let Some(node) = self.nodes.get_mut(&schema_id) {
                node.depends_on = depends_on;
            }
        }
    }

    /// Note on order: if B depends on A, then A's successor is B.
    pub fn successors(
        &self,
        schema_id: &CacheKey,
    ) -> Vec<CacheKey> {
        self.nodes
            .iter()
            .filter_map(|(id, node)| {
                if node.depends_on.contains(schema_id) {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    #[allow(dead_code)]
    pub fn get_node(
        &self,
        schema_id: &CacheKey,
    ) -> Option<&SchemaNode> {
        self.nodes.get(schema_id)
    }

    #[allow(dead_code)]
    pub fn has_circular_dependency(
        &self,
        cycle: &[CacheKey],
    ) -> bool {
        let cycle_set: HashSet<_> = cycle.iter().collect();

        for schema_id in cycle {
            if let Some(node) = self.nodes.get(schema_id) {
                for dep in &node.depends_on {
                    if cycle_set.contains(&dep) && dep != schema_id {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn topological_groups(&self) -> crate::Result<Vec<Vec<CacheKey>>> {
        if self.nodes.is_empty() {
            return Ok(vec![]);
        }

        let mut level_map: HashMap<CacheKey, usize> = HashMap::new();
        let mut queue: VecDeque<(CacheKey, usize)> = VecDeque::new();

        for (schema_id, node) in &self.nodes {
            if node.depends_on.is_empty() {
                tracing::trace!(leaf_node = %schema_id.package_name, "Found leaf node");
                queue.push_back((schema_id.clone(), 0));
                level_map.insert(schema_id.clone(), 0);
            } else {
                tracing::trace!(
                    schema = %schema_id.package_name,
                    depends_on = ?node.depends_on.iter().map(|k| &k.package_name).collect::<Vec<_>>(),
                    "Schema has dependencies"
                );
            }
        }

        tracing::trace!(leaf_count = queue.len(), "Starting BFS from leaf nodes");

        while let Some((current, level)) = queue.pop_front() {
            if let Some(&assigned_level) = level_map.get(&current)
                && level < assigned_level
            {
                continue;
            }

            let successors = self.successors(&current);
            tracing::trace!(
                current = %current.package_name,
                current_level = level,
                successor_count = successors.len(),
                successors = ?successors.iter().map(|k| &k.package_name).collect::<Vec<_>>(),
                "Processing schema"
            );

            for successor in successors {
                let successor_level = level + 1;

                let current_successor_level = level_map
                    .get(&successor)
                    .copied()
                    .unwrap_or(0);

                if successor_level > current_successor_level {
                    tracing::trace!(
                        successor = %successor.package_name,
                        old_level = current_successor_level,
                        new_level = successor_level,
                        "Updating successor level"
                    );
                    level_map.insert(successor.clone(), successor_level);
                    queue.push_back((successor, successor_level));
                }
            }
        }

        if level_map.len() != self.nodes.len() {
            let missing: Vec<String> = self
                .nodes
                .keys()
                .filter(|id| !level_map.contains_key(id))
                .map(|id| id.package_name.clone())
                .collect();

            return Err(crate::Error::SchemaCircularDependency { schemas: missing });
        }

        let max_level = level_map
            .values()
            .copied()
            .max()
            .unwrap_or(0);

        let mut levels: Vec<Vec<CacheKey>> = vec![Vec::new(); max_level + 1];

        for (schema_id, level) in level_map {
            levels[level].push(schema_id);
        }

        for level in &mut levels {
            level.sort_by(|a, b| a.package_name.cmp(&b.package_name));
        }

        Ok(levels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kintsu_manifests::version::Version;

    fn schema_id(name: &str) -> CacheKey {
        CacheKey::new(name.to_string(), Version::parse("1.0.0").unwrap(), None)
    }

    fn import_resolved(
        name: &str,
        resolved: CacheKey,
    ) -> Import {
        Import {
            name: name.to_string(),
            resolved_id: Some(resolved),
        }
    }

    #[test]
    fn test_no_dependencies() {
        let mut graph = SchemaDependencyGraph::new();

        graph.add_schema(schema_id("schema_a"), vec![]);
        graph.add_schema(schema_id("schema_b"), vec![]);

        graph.build_dependencies();

        assert_eq!(
            graph
                .successors(&schema_id("schema_a"))
                .len(),
            0
        );
        assert_eq!(
            graph
                .successors(&schema_id("schema_b"))
                .len(),
            0
        );

        let groups = graph.topological_groups().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
        let ord = groups[0]
            .iter()
            .map(|id| format!("{}", id))
            .collect::<Vec<_>>();

        assert_eq!(ord, vec!["schema_a@1.0.0", "schema_b@1.0.0"])
    }

    #[test]

    fn test_linear_dependencies() {
        let mut graph = SchemaDependencyGraph::new();

        let schema_a_id = schema_id("schema_a");
        let schema_b_id = schema_id("schema_b");
        let schema_c_id = schema_id("schema_c");

        // A depends on B
        graph.add_schema(
            schema_a_id.clone(),
            vec![import_resolved("schema_b", schema_b_id.clone())],
        );
        // B depends on C
        graph.add_schema(
            schema_b_id.clone(),
            vec![import_resolved("schema_c", schema_c_id.clone())],
        );
        // C has no dependencies
        graph.add_schema(schema_c_id.clone(), vec![]);

        graph.build_dependencies();

        // successors() returns "who depends on me"
        // C is depended on by B
        let c_successors = graph.successors(&schema_c_id);
        assert_eq!(c_successors.len(), 1);
        assert!(c_successors.contains(&schema_b_id));

        // B is depended on by A
        let b_successors = graph.successors(&schema_b_id);
        assert_eq!(b_successors.len(), 1);
        assert!(b_successors.contains(&schema_a_id));

        // A has no dependents
        assert_eq!(graph.successors(&schema_a_id).len(), 0);
    }

    #[test]
    fn test_diamond_dependencies() {
        let mut graph = SchemaDependencyGraph::new();

        let schema_a_id = schema_id("schema_a");
        let schema_b_id = schema_id("schema_b");
        let schema_c_id = schema_id("schema_c");
        let schema_d_id = schema_id("schema_d");

        // D depends on B and C
        graph.add_schema(
            schema_d_id.clone(),
            vec![
                import_resolved("schema_b", schema_b_id.clone()),
                import_resolved("schema_c", schema_c_id.clone()),
            ],
        );
        // B depends on A
        graph.add_schema(
            schema_b_id.clone(),
            vec![import_resolved("schema_a", schema_a_id.clone())],
        );
        // C depends on A
        graph.add_schema(
            schema_c_id.clone(),
            vec![import_resolved("schema_a", schema_a_id.clone())],
        );
        // A has no dependencies
        graph.add_schema(schema_a_id.clone(), vec![]);

        graph.build_dependencies();

        // successors() returns "who depends on me"
        // A is depended on by both B and C
        let a_successors = graph.successors(&schema_a_id);
        assert_eq!(a_successors.len(), 2);
        assert!(a_successors.contains(&schema_b_id));
        assert!(a_successors.contains(&schema_c_id));

        // B is depended on by D
        let b_successors = graph.successors(&schema_b_id);
        assert_eq!(b_successors.len(), 1);
        assert!(b_successors.contains(&schema_d_id));

        // C is depended on by D
        let c_successors = graph.successors(&schema_c_id);
        assert_eq!(c_successors.len(), 1);
        assert!(c_successors.contains(&schema_d_id));

        // D has no dependents
        assert_eq!(graph.successors(&schema_d_id).len(), 0);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = SchemaDependencyGraph::new();

        let schema_a_id = schema_id("schema_a");
        let schema_b_id = schema_id("schema_b");

        // A depends on B
        graph.add_schema(
            schema_a_id.clone(),
            vec![import_resolved("schema_b", schema_b_id.clone())],
        );
        // B depends on A (circular!)
        graph.add_schema(
            schema_b_id.clone(),
            vec![import_resolved("schema_a", schema_a_id.clone())],
        );

        graph.build_dependencies();

        let cycle = vec![schema_a_id, schema_b_id];
        assert!(graph.has_circular_dependency(&cycle));
    }

    #[test]
    fn test_topological_groups_empty() {
        let graph = SchemaDependencyGraph::new();
        let groups = graph.topological_groups().unwrap();
        assert_eq!(groups.len(), 0, "Empty graph should have no groups");
    }

    #[test]
    fn test_topological_groups_single_schema() {
        use kintsu_manifests::version::Version;

        let mut graph = SchemaDependencyGraph::new();
        let key = CacheKey::new("test".to_string(), Version::parse("1.0.0").unwrap(), None);
        graph.add_schema(key.clone(), vec![]);
        graph.build_dependencies();

        let groups = graph.topological_groups().unwrap();
        assert_eq!(groups.len(), 1, "Single schema should be one group");
        assert_eq!(groups[0].len(), 1, "Group should contain one schema");
        assert_eq!(groups[0][0], key);
    }

    #[test]
    fn test_topological_groups_linear_dependency() {
        use kintsu_manifests::version::Version;

        let mut graph = SchemaDependencyGraph::new();

        let key_a = CacheKey::new("a".to_string(), Version::parse("1.0.0").unwrap(), None);
        let key_b = CacheKey::new("b".to_string(), Version::parse("1.0.0").unwrap(), None);

        // b depends on a
        graph.add_schema(key_a.clone(), vec![]);
        graph.add_schema(
            key_b.clone(),
            vec![Import {
                name: "a".to_string(),
                resolved_id: None,
            }],
        );
        graph.build_dependencies();

        let groups = graph.topological_groups().unwrap();
        assert_eq!(groups.len(), 2, "Linear dependency should have 2 levels");
        assert_eq!(groups[0].len(), 1, "Level 0 should have 1 schema");
        assert_eq!(groups[1].len(), 1, "Level 1 should have 1 schema");

        // pathfinding's topological_sort_into_groups returns dependencies FIRST
        // So level 0 = things with no dependencies (b depends on a, so a is first)
        assert_eq!(
            groups[0][0], key_a,
            "Level 0 should be schema 'a' (no deps)"
        );
        assert_eq!(
            groups[1][0], key_b,
            "Level 1 should be schema 'b' (depends on a)"
        );
    }

    #[test]
    fn test_topological_groups_parallel_schemas() {
        use kintsu_manifests::version::Version;

        let mut graph = SchemaDependencyGraph::new();

        let key_a = CacheKey::new("a".to_string(), Version::parse("1.0.0").unwrap(), None);
        let key_b = CacheKey::new("b".to_string(), Version::parse("1.0.0").unwrap(), None);
        let key_c = CacheKey::new("c".to_string(), Version::parse("1.0.0").unwrap(), None);

        // a, b, c are independent
        graph.add_schema(key_a.clone(), vec![]);
        graph.add_schema(key_b.clone(), vec![]);
        graph.add_schema(key_c.clone(), vec![]);
        graph.build_dependencies();

        let groups = graph.topological_groups().unwrap();
        assert_eq!(
            groups.len(),
            1,
            "Independent schemas should be in one group"
        );
        assert_eq!(groups[0].len(), 3, "All 3 schemas can compile in parallel");

        // All schemas should be in the group (order doesn't matter)
        let group_set: std::collections::HashSet<_> = groups[0].iter().collect();
        assert!(group_set.contains(&key_a));
        assert!(group_set.contains(&key_b));
        assert!(group_set.contains(&key_c));
    }
}
