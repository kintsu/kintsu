use std::sync::Arc;

use divan::black_box;
use kintsu_fs::memory;
use kintsu_parser::ctx::CompileCtx;
use kintsu_test_suite::many::{GraphGenerator, GraphPattern, populate_fs};

fn compile_bench(
    b: divan::Bencher,
    fs: Arc<kintsu_fs::memory::MemoryFileSystem>,
    root: &str,
) {
    b.with_inputs(|| {
        (
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
            fs.clone(),
        )
    })
    .bench_values(|(rt, fs)| {
        rt.block_on(async {
            CompileCtx::with_fs(black_box(fs.clone()), black_box(root))
                .await
                .unwrap();
        });
    });
}

fn compile_gen_bench<F: Fn() -> (String, Vec<kintsu_test_suite::many::PackageSpec>)>(
    b: divan::Bencher,
    f: F,
) {
    let (root, specs) = f();

    b.with_inputs(|| {
        let mut fs = kintsu_fs::memory::MemoryFileSystem::new();
        populate_fs(&mut fs, &specs);

        let fs = Arc::new(fs);

        (
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
            fs.clone(),
        )
    })
    .bench_values(|(rt, fs)| {
        rt.block_on(async {
            CompileCtx::with_fs(black_box(fs.clone()), black_box(root.clone()))
                .await
                .unwrap();
        });
    });
}

macro_rules! getter {
    ($name: ident: $desc: literal = $root: literal {
        $($body:tt)*
    }) => {
        paste::paste!{
            fn [<get_ $name>]() -> Arc<kintsu_fs::memory::MemoryFileSystem> {
                Arc::new(memory! {
                    $($body)*
                })
            }

            #[divan::bench(name = $desc)]
            fn [<bench_compile_ $name>](
                b: divan::Bencher,
            ) {
                compile_bench(
                    b,
                    [<get_ $name>](),
                    $root,
                );
            }
        }
    };
    ($($name: ident: $desc: literal = $root: literal { $($body:tt)* }), + $(,)?) => {
        $(
            getter! { $name: $desc = $root { $($body)* } }
        )*
    }
}

getter! {
    basic_single: "compile basic single schema" = "pkg" {
        "pkg/schema.toml" => include_str!("../fragments/minimal_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/minimal_lib.ks"),
    },
    with_one_dependency: "compile with one dependency" = "pkg" {
        "abc-corp/schema.toml" => include_str!("../fragments/abc_corp_manifest.toml"),
        "abc-corp/schema/lib.ks" => include_str!("../fragments/abc_corp_lib.ks"),
        "pkg/schema.toml" => include_str!("../fragments/pkg_with_dep_manifest.toml"),
        "pkg/schema/lib.ks" => include_str!("../fragments/use_foo.ks"),
        "pkg/schema/foo/test.ks" => include_str!("../fragments/bar_with_external_type.ks"),
    },
    with_diamond_dependency: "compile with diamond dependency" = "app" {
        "common/schema.toml" => include_str!("../fragments/common_manifest.toml"),
        "common/schema/lib.ks" => include_str!("../fragments/common_lib.ks"),
        "lib-a/schema.toml" => include_str!("../fragments/lib_a_manifest.toml"),
        "lib-a/schema/lib.ks" => include_str!("../fragments/lib_a_lib.ks"),
        "lib-b/schema.toml" => include_str!("../fragments/lib_b_manifest.toml"),
        "lib-b/schema/lib.ks" => include_str!("../fragments/lib_b_lib.ks"),
        "app/schema.toml" => include_str!("../fragments/app_manifest.toml"),
        "app/schema/lib.ks" => include_str!("../fragments/app_lib.ks"),
    }
}

macro_rules! generator {
    ($name: ident: $desc: literal = $f: expr) => {
        paste::paste!{
            #[divan::bench(name = $desc)]
            fn [<bench_compile_ $name>](
                b: divan::Bencher,
            ) {
                compile_gen_bench(
                    b,
                    $f,
                );
            }
        }
    };

    ($($name: ident: $desc: literal = $f: expr), + $(,)?) => {
        $(
            generator! { $name: $desc = $f }
        )*
    }
}

macro_rules! gen_from_expr {
    ($name: ident: $desc: literal = $e: expr) => {
        generator! {
            $name: $desc = || {
                let mut g = kintsu_test_suite::many::GraphGenerator::new();
                let specs = ($e)(&mut g);
                let deps = g.create_root_with_dependencies(vec![
                    specs
                ]);

                (
                    g.root_package().to_string(),
                    deps,
                )
            }
        }
    };

    ($($name: ident: $desc: literal = $e: expr), + $(,)?) => {
        $(
            gen_from_expr! { $name: $desc = $e }
        )*
    }
}

gen_from_expr! {
    diamond_deps_3_layers: "compile with diamond dependencies of 3 layers" = |g: &mut GraphGenerator| {
        g.generate(GraphPattern::Diamond{ layers: 3 })
    },
    diamond_deps_5_layers: "compile with diamond dependencies of 5 layers" = |g: &mut GraphGenerator| {
        g.generate(GraphPattern::Diamond{ layers: 5 })
    },
    linear_deps_10: "compile with 10 linear dependencies" = |g: &mut GraphGenerator| {
        g.generate(GraphPattern::Linear{ depth: 10 })
    },
    linear_deps_25: "compile with 25 linear dependencies" = |g: &mut GraphGenerator| {
        g.generate(GraphPattern::Linear{ depth: 25 })
    },
    linear_deps_100: "compile with 100 linear dependencies" = |g: &mut GraphGenerator| {
        g.generate(GraphPattern::Linear{ depth: 100 })
    },
}

fn main() {
    divan::main();
}
