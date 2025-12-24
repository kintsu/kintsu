use crate::MemoryFileSystem;
use rand::Rng;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct PackageSpec {
    pub name: String,
    pub path: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum GraphPattern {
    /// Linear chain: A -> B -> C -> D
    Linear { depth: usize },
    /// Diamond: A -> B,C -> D (both B and C depend on D)
    Diamond { layers: usize },
    /// Tree: root with N children, each with M children
    Tree { branching: usize, depth: usize },
    /// Random DAG with specified nodes and edges
    RandomDag { nodes: usize, edge_probability: f64 },
    /// Star: one central node with N dependents
    Star { satellites: usize },
}

pub struct GraphGenerator {
    rng: rand::rngs::ThreadRng,
    counter: usize,
    root_package: String,
}

impl GraphGenerator {
    pub fn new() -> Self {
        Self::with_root("root-pkg")
    }

    pub fn with_root(root_name: &str) -> Self {
        Self {
            rng: rand::rng(),
            counter: 0,
            root_package: root_name.to_string(),
        }
    }

    pub fn root_package(&self) -> &str {
        &self.root_package
    }

    fn next_name(
        &mut self,
        prefix: &str,
    ) -> String {
        self.counter += 1;
        format!("{}-{}", prefix, self.counter)
    }

    pub fn generate(
        &mut self,
        pattern: GraphPattern,
    ) -> Vec<PackageSpec> {
        match pattern {
            GraphPattern::Linear { depth } => self.gen_linear(depth),
            GraphPattern::Diamond { layers } => self.gen_diamond(layers),
            GraphPattern::Tree { branching, depth } => self.gen_tree(branching, depth),
            GraphPattern::RandomDag {
                nodes,
                edge_probability,
            } => self.gen_random_dag(nodes, edge_probability),
            GraphPattern::Star { satellites } => self.gen_star(satellites),
        }
    }

    fn gen_linear(
        &mut self,
        depth: usize,
    ) -> Vec<PackageSpec> {
        let mut specs = Vec::new();
        let mut prev = None;

        for _ in 0..depth {
            let name = self.next_name("pkg");
            let deps = if let Some(p) = prev {
                vec![p]
            } else {
                vec![]
            };

            specs.push(PackageSpec {
                name: name.clone(),
                path: name.clone(),
                dependencies: deps,
            });

            prev = Some(name);
        }

        specs
    }

    fn gen_diamond(
        &mut self,
        layers: usize,
    ) -> Vec<PackageSpec> {
        let mut specs = Vec::new();
        let mut current_layer = vec![];

        // bottom layer (no deps)
        let base = self.next_name("base");
        specs.push(PackageSpec {
            name: base.clone(),
            path: base.clone(),
            dependencies: vec![],
        });
        current_layer.push(base);

        // middle layers (diamond shape)
        for _ in 0..layers {
            let mut next_layer = vec![];
            let width = self.rng.random_range(2..=4);

            for _ in 0..width {
                let name = self.next_name("mid");
                specs.push(PackageSpec {
                    name: name.clone(),
                    path: name.clone(),
                    dependencies: current_layer.clone(),
                });
                next_layer.push(name);
            }

            current_layer = next_layer;
        }

        // top layer (depends on all in current layer)
        let top = self.next_name("top");
        specs.push(PackageSpec {
            name: top.clone(),
            path: top,
            dependencies: current_layer,
        });

        specs
    }

    fn gen_tree(
        &mut self,
        branching: usize,
        depth: usize,
    ) -> Vec<PackageSpec> {
        let mut specs = Vec::new();
        let mut current_level = Vec::new();

        // root (no deps)
        let root = self.next_name("root");
        specs.push(PackageSpec {
            name: root.clone(),
            path: root.clone(),
            dependencies: vec![],
        });
        current_level.push(root);

        // build tree levels
        for _ in 0..depth {
            let mut next_level = Vec::new();

            for parent in &current_level {
                for _ in 0..branching {
                    let name = self.next_name("node");
                    specs.push(PackageSpec {
                        name: name.clone(),
                        path: name.clone(),
                        dependencies: vec![parent.clone()],
                    });
                    next_level.push(name);
                }
            }

            current_level = next_level;
        }

        specs
    }

    fn gen_random_dag(
        &mut self,
        nodes: usize,
        edge_probability: f64,
    ) -> Vec<PackageSpec> {
        let mut specs = Vec::new();
        let mut names = Vec::new();

        // create nodes
        for _ in 0..nodes {
            let name = self.next_name("pkg");
            names.push(name.clone());
            specs.push(PackageSpec {
                name: name.clone(),
                path: name,
                dependencies: vec![],
            });
        }

        // add edges (only forward to maintain DAG property)
        for i in 0..nodes {
            for j in 0..i {
                if self.rng.random_bool(edge_probability) {
                    specs[i].dependencies.push(names[j].clone());
                }
            }
        }

        specs
    }

    fn gen_star(
        &mut self,
        satellites: usize,
    ) -> Vec<PackageSpec> {
        let mut specs = Vec::new();

        // central node
        let center = self.next_name("core");
        specs.push(PackageSpec {
            name: center.clone(),
            path: center.clone(),
            dependencies: vec![],
        });

        // satellites
        for _ in 0..satellites {
            let name = self.next_name("sat");
            specs.push(PackageSpec {
                name: name.clone(),
                path: name,
                dependencies: vec![center.clone()],
            });
        }

        specs
    }

    /// Create a root package that depends on multiple dependency chains.
    /// This allows testing scenarios where a single root package depends on
    /// multiple independent dependency graphs.
    pub fn create_root_with_dependencies(
        &self,
        dependency_chains: Vec<Vec<PackageSpec>>,
    ) -> Vec<PackageSpec> {
        let mut all_specs = Vec::new();
        let mut root_deps = Vec::new();

        for chain in dependency_chains {
            if let Some(terminal) = find_root_package(&chain) {
                root_deps.push(terminal);
            }
            all_specs.extend(chain);
        }

        let root = PackageSpec {
            name: self.root_package().to_string(),
            path: self.root_package().to_string(),
            dependencies: root_deps,
        };

        all_specs.push(root);
        all_specs
    }
}

impl Default for GraphGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Populate a MemoryFileSystem with generated packages
pub fn populate_fs(
    fs: &mut MemoryFileSystem,
    specs: &[PackageSpec],
) {
    populate_fs_with_options(fs, specs, PopulateOptions::default());
}

#[derive(Debug, Clone, Default)]
pub struct PopulateOptions {
    pub include_operations: bool,
    pub include_errors: bool,
    pub include_nested_namespaces: bool,
}

/// Populate a MemoryFileSystem with generated packages with custom options
pub fn populate_fs_with_options(
    fs: &mut MemoryFileSystem,
    specs: &[PackageSpec],
    options: PopulateOptions,
) {
    // Generate compilation order and create order.json
    let order = compute_compilation_order(specs);
    let order_json = serde_json::to_string_pretty(&order).unwrap();
    fs.add_file("order.json", order_json.as_bytes());

    for spec in specs {
        // create manifest
        let manifest = generate_manifest(&spec.name, &spec.dependencies);
        let manifest_path = format!("{}/schema.toml", spec.path);
        fs.add_file(manifest_path, manifest.as_bytes());

        // create lib.ks with namespace and types
        let lib_content = generate_lib(&spec.name, &spec.dependencies);
        let lib_path = format!("{}/schema/lib.ks", spec.path);
        fs.add_file(lib_path, lib_content.as_bytes());

        // optionally generate additional files
        if options.include_operations {
            let ops_content = generate_operations(&spec.name, &spec.dependencies);
            let ops_path = format!("{}/schema/operations.ks", spec.path);
            fs.add_file(ops_path, ops_content.as_bytes());
        }

        if options.include_errors {
            let errors_content = generate_errors(&spec.name);
            let errors_path = format!("{}/schema/errors.ks", spec.path);
            fs.add_file(errors_path, errors_content.as_bytes());
        }

        if options.include_nested_namespaces {
            let nested_content = generate_nested_namespace(&spec.name);
            let nested_path = format!("{}/schema/nested/types.ks", spec.path);
            fs.add_file(nested_path, nested_content.as_bytes());
        }
    }
}

/// Compute a valid compilation order using topological sort
fn compute_compilation_order(specs: &[PackageSpec]) -> Vec<String> {
    use std::collections::{HashMap, HashSet};

    let mut order = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();

    // Build a map of package name to its spec
    let spec_map: HashMap<&str, &PackageSpec> = specs
        .iter()
        .map(|spec| (spec.name.as_str(), spec))
        .collect();

    fn visit<'a>(
        name: &'a str,
        spec_map: &HashMap<&'a str, &'a PackageSpec>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) {
        if visited.contains(name) {
            return;
        }

        if visiting.contains(name) {
            // Circular dependency - shouldn't happen in a DAG
            return;
        }

        visiting.insert(name.to_string());

        // Visit all dependencies first
        if let Some(spec) = spec_map.get(name) {
            for dep in &spec.dependencies {
                visit(dep, spec_map, visited, visiting, order);
            }
        }

        visiting.remove(name);
        visited.insert(name.to_string());
        order.push(name.to_string());
    }

    // Visit all packages
    for spec in specs {
        visit(
            &spec.name,
            &spec_map,
            &mut visited,
            &mut visiting,
            &mut order,
        );
    }

    order
}

fn generate_operations(
    name: &str,
    dependencies: &[String],
) -> String {
    let pkg_name = name.replace('-', "_");
    let mut ops = format!("namespace {};\n\n", pkg_name);

    ops.push_str("namespace ops {\n");

    // Add use statements for dependency types
    for dep in dependencies {
        let dep_name = dep.replace('-', "_");
        ops.push_str(&format!(
            "\tuse {}::types::{}Data;\n",
            dep_name,
            to_pascal_case(&dep_name)
        ));
    }

    if !dependencies.is_empty() {
        ops.push('\n');
    }

    // Simple operation
    ops.push_str(&format!(
        "\toperation get_{}(id: u64) -> types::{}Data;\n\n",
        pkg_name,
        to_pascal_case(&pkg_name)
    ));

    // Operation with dependency type (using short name)
    if let Some(first_dep) = dependencies.first() {
        let dep_name = first_dep.replace('-', "_");
        ops.push_str(&format!(
            "\toperation create_{}(data: types::{}Data, dep: {}Data) -> types::{}Data;\n",
            pkg_name,
            to_pascal_case(&pkg_name),
            to_pascal_case(&dep_name),
            to_pascal_case(&pkg_name)
        ));
    }

    ops.push_str("};\n");
    ops
}

fn generate_errors(name: &str) -> String {
    let pkg_name = name.replace('-', "_");
    let mut errors = format!("namespace {};\n\n", pkg_name);

    errors.push_str("namespace errors {\n");
    errors.push_str(&format!("\terror {}Error {{\n", to_pascal_case(&pkg_name)));
    errors.push_str("\t\tNotFound,\n");
    errors.push_str("\t\tInvalidInput,\n");
    errors.push_str("\t\tAlreadyExists\n");
    errors.push_str("\t};\n");
    errors.push_str("};\n");

    errors
}

fn generate_nested_namespace(
    name: &str,
    // dependencies: &[String],
) -> String {
    let pkg_name = name.replace('-', "_");
    let mut nested = format!("namespace {};\n\n", pkg_name);

    // NOTE: No use statements for external dependencies
    // External package types are referenced with their full path

    nested.push_str("namespace nested {\n");
    nested.push_str("\tnamespace models {\n");
    nested.push_str(&format!(
        "\t\tstruct Nested{}Model {{\n",
        to_pascal_case(&pkg_name)
    ));
    nested.push_str("\t\t\tvalue: str,\n");
    nested.push_str("\t\t\tcount: i32\n");
    nested.push_str("\t\t};\n");
    nested.push_str("\t};\n");
    nested.push_str("};\n");

    nested
}

fn generate_manifest(
    name: &str,
    dependencies: &[String],
) -> String {
    let mut manifest = format!(
        "[package]
name = \"{name}\"
version = \"1.0.0\"
description = \"Generated package\"
license = \"MIT\"
license_text = \"MIT License\"
authors = [{{ name = \"Kintsu\", email = \"foo@bar.com\" }}]
readme = \"# foo\"
repository = \"https://github.com/kintsu/kintsu\"
",
    );

    if !dependencies.is_empty() {
        manifest.push_str("\n[dependencies]\n");
        for dep in dependencies {
            manifest.push_str(&format!(
                "{} = {{ path = \"../{}\", version = \"1.0.0\" }}\n",
                dep, dep
            ));
        }
    }

    manifest
}

fn generate_lib(
    name: &str,
    dependencies: &[String],
) -> String {
    let pkg_name = name.replace('-', "_");
    let mut lib = format!("namespace {};\n\n", pkg_name);

    // Generate namespace with various types
    lib.push_str("namespace types {\n");

    // Add use statements for dependency types at the top of the namespace
    for dep in dependencies {
        let dep_name = dep.replace('-', "_");
        lib.push_str(&format!(
            "\tuse {}::types::{}Data;\n",
            dep_name,
            to_pascal_case(&dep_name)
        ));
    }

    if !dependencies.is_empty() {
        lib.push('\n');
    }

    // Generate enum
    lib.push_str(&format!("\tenum {}Status {{\n", to_pascal_case(&pkg_name)));
    lib.push_str("\t\tActive = 0,\n");
    lib.push_str("\t\tInactive = 1,\n");
    lib.push_str("\t\tPending = 2\n");
    lib.push_str("\t};\n\n");

    // Generate type alias that references dependency if available
    if let Some(first_dep) = dependencies.first() {
        let dep_name = first_dep.replace('-', "_");
        lib.push_str(&format!(
            "\ttype {}Ref = {}Data;\n\n",
            to_pascal_case(&pkg_name),
            to_pascal_case(&dep_name)
        ));
    } else {
        lib.push_str(&format!(
            "\ttype {}Id = u64;\n\n",
            to_pascal_case(&pkg_name)
        ));
    }

    // Generate main struct with dependency references
    lib.push_str(&format!("\tstruct {}Data {{\n", to_pascal_case(&pkg_name)));
    lib.push_str("\t\tid: u64,\n");
    lib.push_str(&format!("\t\tstatus: {}Status", to_pascal_case(&pkg_name)));

    // Add fields that reference types from each dependency (using short names)
    for dep in dependencies {
        let dep_name = dep.replace('-', "_");
        lib.push_str(&format!(
            ",\n\t\t{}_data: {}Data",
            dep_name,
            to_pascal_case(&dep_name)
        ));
    }

    lib.push_str("\n\t};\n");

    // Generate a wrapper struct that uses the type alias
    if !dependencies.is_empty() {
        lib.push_str(&format!(
            "\n\tstruct {}Wrapper {{\n",
            to_pascal_case(&pkg_name)
        ));
        lib.push_str(&format!("\t\tdata: {}Data,\n", to_pascal_case(&pkg_name)));
        lib.push_str("\t\ttimestamp: datetime\n");
        lib.push_str("\t};\n");
    }

    lib.push_str("};\n");

    lib
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                },
            }
        })
        .collect()
}

/// Helper to find the root package (the one nothing depends on)
pub fn find_root_package(specs: &[PackageSpec]) -> Option<String> {
    let mut all_deps: HashSet<String> = HashSet::new();
    let mut all_names: HashSet<String> = HashSet::new();

    for spec in specs {
        all_names.insert(spec.name.clone());
        for dep in &spec.dependencies {
            all_deps.insert(dep.clone());
        }
    }

    // root is in all_names but not in all_deps
    all_names
        .difference(&all_deps)
        .next()
        .cloned()
        .inspect(|ok| println!("root package: {ok}"))
}

/// Populate filesystem with only the root package.
/// Use this in combination with `populate_fs` for dependency chains.
pub fn populate_root_only(
    fs: &mut MemoryFileSystem,
    root: &PackageSpec,
) {
    populate_fs(fs, &[root.clone()]);
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use kintsu_fs::FileSystem;

    use super::*;

    #[test]
    fn test_linear_generation() {
        let mut g = GraphGenerator::new();
        let specs = g.generate(GraphPattern::Linear { depth: 3 });
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].dependencies.len(), 0);
        assert_eq!(specs[1].dependencies.len(), 1);
        assert_eq!(specs[2].dependencies.len(), 1);
    }

    #[test]
    fn test_diamond_generation() {
        let mut g = GraphGenerator::new();
        let specs = g.generate(GraphPattern::Diamond { layers: 2 });
        assert!(specs.len() >= 4); // at least base + 2 layers + top
    }

    #[test]
    fn test_find_root() {
        let specs = vec![
            PackageSpec {
                name: "base".into(),
                path: "base".into(),
                dependencies: vec![],
            },
            PackageSpec {
                name: "top".into(),
                path: "top".into(),
                dependencies: vec!["base".into()],
            },
        ];
        assert_eq!(find_root_package(&specs), Some("top".into()));
    }

    #[test]
    fn test_populate_fs() {
        let mut fs = MemoryFileSystem::new();
        let specs = vec![PackageSpec {
            name: "test-pkg".into(),
            path: "test-pkg".into(),
            dependencies: vec![],
        }];
        populate_fs(&mut fs, &specs);
        assert!(fs.exists_sync(Path::new("test-pkg/schema.toml")));
        let lib = Path::new("test-pkg/schema/lib.ks");
        assert!(fs.exists_sync(lib));

        assert_eq!(
            fs.read_to_string_sync(lib).unwrap(),
            "namespace test_pkg;\n\nnamespace types {\n\tenum TestPkgStatus {\n\t\tActive = 0,\n\t\tInactive = 1,\n\t\tPending = 2\n\t};\n\n\ttype TestPkgId = u64;\n\n\tstruct TestPkgData {\n\t\tid: u64,\n\t\tstatus: TestPkgStatus\n\t};\n};\n"
        );
    }

    #[test]
    fn test_create_root_with_multiple_chains() {
        let mut g = GraphGenerator::with_root("my-root");

        // Generate multiple dependency chains
        let chain1 = g.generate(GraphPattern::Linear { depth: 3 });
        let chain2 = g.generate(GraphPattern::Linear { depth: 2 });

        // Create root that depends on both chains
        let all_specs = g.create_root_with_dependencies(vec![chain1, chain2]);

        // Root should be the last package added
        let root = all_specs.last().unwrap();
        assert_eq!(root.name, "my-root");
        assert_eq!(root.dependencies.len(), 2); // depends on 2 terminals

        // Verify root is indeed the root
        assert_eq!(find_root_package(&all_specs), Some("my-root".to_string()));
    }

    #[test]
    fn test_compilation_order() {
        use std::path::Path;

        let mut g = GraphGenerator::new();
        // Create diamond: d <- b,c <- a
        let specs = vec![
            PackageSpec {
                name: "d".into(),
                path: "d".into(),
                dependencies: vec![],
            },
            PackageSpec {
                name: "b".into(),
                path: "b".into(),
                dependencies: vec!["d".into()],
            },
            PackageSpec {
                name: "c".into(),
                path: "c".into(),
                dependencies: vec!["d".into()],
            },
            PackageSpec {
                name: "a".into(),
                path: "a".into(),
                dependencies: vec!["b".into(), "c".into()],
            },
        ];

        let mut fs = MemoryFileSystem::new();
        populate_fs(&mut fs, &specs);

        // Check that order.json exists
        let order_path = Path::new("order.json");
        assert!(fs.exists_sync(order_path));

        // Parse and verify order
        let order_json = fs.read_to_string_sync(order_path).unwrap();
        let order: Vec<String> = serde_json::from_str(&order_json).unwrap();

        assert_eq!(order.len(), 4);

        // d should come before b and c
        let d_pos = order.iter().position(|s| s == "d").unwrap();
        let b_pos = order.iter().position(|s| s == "b").unwrap();
        let c_pos = order.iter().position(|s| s == "c").unwrap();
        let a_pos = order.iter().position(|s| s == "a").unwrap();

        assert!(d_pos < b_pos);
        assert!(d_pos < c_pos);
        assert!(b_pos < a_pos);
        assert!(c_pos < a_pos);
    }
}
