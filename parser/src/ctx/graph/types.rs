use crate::ctx::paths::NamedItemContext;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// non-terminating cycles
    Required,

    /// terminates recursion
    Optional,

    /// terminates recursion (empty array)
    Array,
}

impl EdgeKind {
    pub fn is_terminating(self) -> bool {
        matches!(self, EdgeKind::Optional | EdgeKind::Array)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDependency {
    pub target_candidates: Vec<NamedItemContext>,
    pub kind: EdgeKind,
    pub field_path: Vec<String>,
}

impl TypeDependency {
    #[allow(dead_code)]
    pub fn with_target(
        target: NamedItemContext,
        kind: EdgeKind,
        field_path: Vec<String>,
    ) -> Self {
        Self {
            target_candidates: vec![target],
            kind,
            field_path,
        }
    }

    pub fn with_candidates(
        candidates: Vec<NamedItemContext>,
        kind: EdgeKind,
        field_path: Vec<String>,
    ) -> Self {
        Self {
            target_candidates: candidates,
            kind,
            field_path,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TypeDependencyGraph {
    nodes: BTreeMap<NamedItemContext, Vec<TypeDependency>>,
}

impl TypeDependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }

    pub fn add_type(
        &mut self,
        name: NamedItemContext,
        dependencies: Vec<TypeDependency>,
    ) {
        self.nodes.insert(name.clone(), dependencies);
    }

    pub fn type_names(&self) -> Vec<NamedItemContext> {
        self.nodes.keys().cloned().collect()
    }

    pub fn successors(
        &self,
        type_name: &NamedItemContext,
        only_required: bool,
    ) -> Vec<NamedItemContext> {
        self.nodes
            .get(type_name)
            .map(|node| {
                node.iter()
                    .filter(|dep| {
                        if only_required {
                            dep.kind == EdgeKind::Required
                        } else {
                            true
                        }
                    })
                    .flat_map(|dep| dep.target_candidates.iter())
                    // Map reference-site candidates to definition-site keys
                    // This ensures consistent spans for cycle detection
                    .filter_map(|candidate| {
                        self.nodes
                            .get_key_value(candidate)
                            .map(|(def_key, _)| def_key.clone())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn all_successors(
        &self,
        type_name: &NamedItemContext,
    ) -> Vec<NamedItemContext> {
        self.successors(type_name, false)
    }

    pub fn required_successors(
        &self,
        type_name: &NamedItemContext,
    ) -> Vec<NamedItemContext> {
        self.successors(type_name, true)
    }

    pub fn has_terminating_edge(
        &self,
        cycle: &[NamedItemContext],
    ) -> bool {
        let cycle_set: HashSet<_> = cycle.iter().cloned().collect();

        for type_name in cycle {
            if let Some(dependencies) = self.nodes.get(type_name) {
                for dep in dependencies {
                    if dep
                        .target_candidates
                        .iter()
                        .any(|candidate| cycle_set.contains(candidate))
                        && dep.kind.is_terminating()
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    #[allow(dead_code)]
    pub fn get_node(
        &self,
        type_name: &NamedItemContext,
    ) -> Option<&Vec<TypeDependency>> {
        self.nodes.get(type_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tst::test_ctx;

    #[test]
    fn test_simple_dependency() {
        let mut graph = TypeDependencyGraph::new();

        graph.add_type(
            test_ctx("A"),
            vec![TypeDependency::with_target(
                test_ctx("B"),
                EdgeKind::Required,
                vec!["field_b".to_string()],
            )],
        );

        graph.add_type(test_ctx("B"), vec![]);

        assert_eq!(graph.all_successors(&test_ctx("A")), vec![test_ctx("B")]);
        assert_eq!(graph.all_successors(&test_ctx("B")).len(), 0);
    }

    #[test]
    fn test_terminating_cycle() {
        let mut graph = TypeDependencyGraph::new();

        // A has optional field of type B
        graph.add_type(
            test_ctx("A"),
            vec![TypeDependency::with_target(
                test_ctx("B"),
                EdgeKind::Optional,
                vec!["b".to_string()],
            )],
        );

        // B has required field of type A
        graph.add_type(
            test_ctx("B"),
            vec![TypeDependency::with_target(
                test_ctx("A"),
                EdgeKind::Required,
                vec!["a".to_string()],
            )],
        );

        let cycle = vec![test_ctx("A"), test_ctx("B")];
        assert!(graph.has_terminating_edge(&cycle));
    }

    #[test]
    fn test_non_terminating_cycle() {
        let mut graph = TypeDependencyGraph::new();

        // A has required field of type B
        graph.add_type(
            test_ctx("A"),
            vec![TypeDependency::with_target(
                test_ctx("B"),
                EdgeKind::Required,
                vec!["b".to_string()],
            )],
        );

        // B has required field of type A
        graph.add_type(
            test_ctx("B"),
            vec![TypeDependency::with_target(
                test_ctx("A"),
                EdgeKind::Required,
                vec!["a".to_string()],
            )],
        );

        let cycle = vec![test_ctx("A"), test_ctx("B")];
        assert!(!graph.has_terminating_edge(&cycle));
    }

    #[test]
    fn test_required_vs_all_successors() {
        let mut graph = TypeDependencyGraph::new();

        graph.add_type(
            test_ctx("A"),
            vec![
                TypeDependency::with_target(
                    test_ctx("B"),
                    EdgeKind::Required,
                    vec!["b".to_string()],
                ),
                TypeDependency::with_target(
                    test_ctx("C"),
                    EdgeKind::Optional,
                    vec!["c".to_string()],
                ),
                TypeDependency::with_target(test_ctx("D"), EdgeKind::Array, vec!["d".to_string()]),
            ],
        );

        // Add the dependency types to the graph
        graph.add_type(test_ctx("B"), vec![]);
        graph.add_type(test_ctx("C"), vec![]);
        graph.add_type(test_ctx("D"), vec![]);

        let all = graph.all_successors(&test_ctx("A"));
        assert_eq!(all.len(), 3);

        let required = graph.required_successors(&test_ctx("A"));
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], test_ctx("B"));
    }
}
