use kintsu_test_suite::{
    many::{GraphGenerator, GraphPattern, PopulateOptions, populate_fs, populate_fs_with_options},
    *,
};

#[tokio::test]
async fn compile_linear_dependency_chain() {
    let mut g = GraphGenerator::new();

    let specs = g.generate(GraphPattern::Linear { depth: 5 });
    let specs = g.create_root_with_dependencies(vec![specs]);

    let mut fs = MemFs::new();
    populate_fs(&mut fs, &specs);

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_linear_dependency_chain",
        "Linear Dependency Chain",
        "Test lockfile generation for a linear chain of dependencies",
        true,
        vec![Tag::Dependencies],
    )
    .with_root(g.root_package());

    let ctx = harness.compile_pass().await;
    harness.assert_lockfile_written();
}

#[tokio::test]
async fn compile_diamond_dependencies() {
    let mut g = GraphGenerator::new();
    let specs = g.generate(GraphPattern::Diamond { layers: 2 });
    let specs = g.create_root_with_dependencies(vec![specs]);

    let mut fs = MemFs::new();
    populate_fs(&mut fs, &specs);

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_diamond_dependencies",
        "Diamond Dependencies",
        "Test lockfile generation for a diamond dependency structure",
        true,
        vec![Tag::Dependencies],
    )
    .with_root(g.root_package());

    let _ = harness.compile_pass().await;
    harness.assert_lockfile_written();
}

#[tokio::test]
async fn compile_tree_dependencies() {
    let mut g = GraphGenerator::new();
    let specs = g.generate(GraphPattern::Tree {
        branching: 2,
        depth: 3,
    });

    let mut fs = MemFs::new();
    populate_fs(&mut fs, &specs);

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_tree_dependencies",
        "Tree Dependencies",
        "Test lockfile generation for a tree dependency structure",
        true,
        vec![Tag::Dependencies],
    )
    .with_root("root-1");

    let _ = harness.compile_pass().await;
    harness.assert_lockfile_written();
}

#[tokio::test]
async fn compile_star_dependencies() {
    let mut g = GraphGenerator::new();
    let specs = g.generate(GraphPattern::Star { satellites: 5 });
    let specs = g.create_root_with_dependencies(vec![specs]);

    let mut fs = MemFs::new();
    populate_fs(&mut fs, &specs);

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_star_dependencies",
        "Star Dependencies",
        "Test lockfile generation for a star dependency structure",
        true,
        vec![Tag::Dependencies],
    )
    .with_root(g.root_package());

    let _ = harness.compile_pass().await;
    harness.assert_lockfile_written();
}

#[tokio::test]
async fn compile_random_dag_dependencies() {
    let mut g = GraphGenerator::new();
    let specs = g.generate(GraphPattern::RandomDag {
        nodes: 8,
        edge_probability: 0.3,
    });
    let specs = g.create_root_with_dependencies(vec![specs]);

    let mut fs = MemFs::new();
    populate_fs(&mut fs, &specs);

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_random_dag_dependencies",
        "Random DAG Dependencies",
        "Test lockfile generation for a random DAG dependency structure",
        true,
        vec![Tag::Dependencies],
    )
    .with_root(g.root_package());

    let ctx = harness.compile_pass().await;

    assert!(!ctx.type_registry().all_types().is_empty());

    harness.assert_lockfile_written();
}

#[tokio::test]
async fn compile_linear_with_operations_and_errors() {
    let mut g = GraphGenerator::new();
    let specs = g.generate(GraphPattern::Linear { depth: 10 });
    let specs = g.create_root_with_dependencies(vec![specs]);

    let mut fs = MemFs::new();
    let options = PopulateOptions {
        include_operations: true,
        include_errors: true,
        include_nested_namespaces: true,
    };
    populate_fs_with_options(&mut fs, &specs, options);

    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_linear_with_operations_and_errors",
        "Linear with Operations and Errors",
        "Test lockfile generation with operations and errors included",
        true,
        vec![Tag::Lockfile, Tag::Error, Tag::Operation],
    )
    .with_root(g.root_package());

    let ctx = harness.compile_pass().await;

    std::fs::write(
        "linear.json",
        serde_json::to_string_pretty(&ctx.type_registry().all_types()).unwrap(),
    )
    .unwrap();

    std::fs::write(
        "declarations.json",
        serde_json::to_string_pretty(&ctx.emit_declarations().await.unwrap()).unwrap(),
    )
    .unwrap();
    harness.assert_lockfile_written();
}

#[tokio::test]
async fn compile_multi_chain_root() {
    let mut g = GraphGenerator::with_root("multi-root");

    // Create multiple independent dependency chains
    let chain1 = g.generate(GraphPattern::Linear { depth: 3 });
    let chain2 = g.generate(GraphPattern::Diamond { layers: 2 });
    let chain3 = g.generate(GraphPattern::Linear { depth: 2 });

    // Combine all chains with a single root
    let all_specs = g.create_root_with_dependencies(vec![chain1, chain2, chain3]);

    let mut fs = MemFs::new();
    populate_fs(&mut fs, &all_specs);

    let root = g.root_package();
    let mut harness = TestHarness::with_metadata(
        fs,
        "compile_multi_chain_root",
        "Multi-Chain Root Dependencies",
        "Test lockfile generation where root depends on multiple independent chains",
        true,
        vec![Tag::Dependencies, Tag::Lockfile],
    )
    .with_root(root);

    let ctx = harness.compile_pass().await;

    // Verify we registered types from all chains
    let type_count = ctx.type_registry().all_types().len();
    assert!(
        type_count > 0,
        "Should have types from all dependency chains"
    );

    harness.assert_lockfile_written();
}
