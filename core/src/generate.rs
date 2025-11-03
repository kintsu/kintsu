pub mod context;
pub mod files;
pub mod matcher;
pub mod python;
pub mod remote;
pub mod rust;

use std::{
    collections::BTreeMap,
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};

use kintsu_manifests::config::NewForConfig;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;
use validator::Validate;

use crate::{
    Enum, ErrorTy, Ident, Named, OneOf, Operation, Result, Struct,
    context::Context,
    default,
    generate::{
        context::WithNsContext,
        files::{MemFlush, WithFlush},
        remote::RemoteConfig,
        rust::{RustGenState, RustGenerator},
    },
};

pub trait LanguageTrait {
    fn file_case() -> convert_case::Case<'static>;
    fn file_ext() -> &'static str;

    fn file_name<P: AsRef<Path>>(name: P) -> PathBuf {
        PathBuf::from(format!("{}.{}", name.as_ref().display(), Self::file_ext()))
    }
}

#[derive(Deserialize, PartialEq, Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Rust,
}

#[derive(Deserialize, PartialEq, Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Client,
    Server,
    Types,
}

pub trait ConfigExt: Debug + PartialEq + Clone + Validate {}

default!(
    output_dir: PathBuf = "./".into()
);

#[derive(Deserialize, PartialEq, Debug, Clone, Validate)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(rename_all = "kebab-case")]
pub struct GenOpts<Ext: ConfigExt> {
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,

    #[serde(flatten)]
    #[validate(nested)]
    pub opts: Ext,

    #[serde(default)]
    pub mem: bool,
}

#[derive(Deserialize, PartialEq, Debug, Clone, Default)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(rename_all = "snake_case")]
pub enum Vis {
    #[serde(alias = "local")]
    Private,
    Crate,
    #[default]
    #[serde(alias = "pub")]
    Public,
}

/// this is a named mapping of namespace > ident > vis
///
/// e.g.
/// ```yaml
/// vis:
///     default: crate
///     abc.corp.test:
///         PublicEnum: pub # or 'public'
///         CrateStruct: crate
///         ModOnlyStruct: private # or 'local'
/// ```
#[derive(Deserialize, PartialEq, Debug, Clone, Default, Validate)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct VisConfig {
    #[serde(default)]
    default: Vis,

    #[serde(default)]
    overrides: Named<Named<Vis>>,
}

impl Vis {
    pub(crate) fn as_rust(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Public => quote::quote!(pub),
            Self::Crate => quote::quote!(pub(crate)),
            Self::Private => quote::quote!(),
        }
    }
}

impl VisConfig {
    pub(crate) fn as_rust(
        &self,
        ns: &WithNsContext<'_, RustGenState, RustConfig, RustGenerator>,
        ident: &Ident,
    ) -> proc_macro2::TokenStream {
        match self
            .overrides
            .get(&ns.ns.name)
            .map(|rule| rule.get(ident))
        {
            Some(Some(vis)) => vis.as_rust(),
            _ => self.default.as_rust(),
        }
    }
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum DateTimeLibrary {
    Chrono,
    Time,
}

impl Default for DateTimeLibrary {
    #[allow(unreachable_code, clippy::needless_return)]
    fn default() -> Self {
        #[cfg(all(not(feature = "chrono"), feature = "time"))]
        return Self::Time;
        return Self::Chrono;
    }
}

#[derive(Deserialize, PartialEq, Debug, Clone, Validate)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(rename_all = "kebab-case")]
pub struct RustConfig {
    #[serde(default)]
    #[validate(nested)]
    pub vis: VisConfig,

    #[serde(default)]
    pub time: DateTimeLibrary,
}

impl ConfigExt for RustConfig {}

#[derive(Deserialize, PartialEq, Debug, Clone, Validate)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(rename_all = "kebab-case")]
pub struct JsonSchemaConfig {}

impl ConfigExt for JsonSchemaConfig {}

crate::default!(
    Vec<Target>: { targets = vec![Target::Types] },
);

#[derive(Deserialize, PartialEq, Debug, Validate)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct Source {
    #[serde(default)]
    #[validate(nested)]
    remote: Vec<RemoteConfig>,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Deserialize, PartialEq, Debug, Validate)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(rename_all = "kebab-case")]
pub struct GenerationConfig {
    #[serde(default = "default_targets")]
    pub targets: Vec<Target>,

    #[serde(default)]
    pub languages: Vec<Language>,

    #[validate(nested)]
    pub sources: Source,

    #[serde(default)]
    #[validate(nested)]
    pub rust: Option<GenOpts<RustConfig>>,
}

impl NewForConfig for GenerationConfig {
    const NAME: &'static str = "op-gen";
}

pub trait Generate<State: Sync + Send, Ext: ConfigExt + Sync + Send>
where
    Self: LanguageTrait + Sized + Sync + Send, {
    fn on_create(
        state: &WithNsContext<'_, State, Ext, Self>,
        fname: &Path,
        f: &mut Box<dyn WithFlush>,
    ) -> std::io::Result<()>;

    fn new_state(
        &self,
        opts: &GenOpts<Ext>,
    ) -> State;

    #[allow(unused)]
    fn with_all_namespaces(
        &self,
        ctx: &Context,
        opts: &GenOpts<Ext>,
        ctx_ns: BTreeMap<crate::Ident, WithNsContext<'_, State, Ext, Self>>,
    ) -> Result<()> {
        Ok(())
    }

    fn gen_ty_ctx(
        &self,
        ctx: &Context,
        opts: &GenOpts<Ext>,
        mem_flush: Option<MemFlush>,
        root: &GenerationConfig,
    ) -> Result<()> {
        let state = Arc::new(self.new_state(opts));
        let mut ctx_ns = BTreeMap::new();

        for ns in ctx.namespaces.values() {
            let ns_ctx = WithNsContext::new(
                ns,
                state.clone(),
                opts,
                Box::new(|state, fname, f| Self::on_create(state, fname, f)),
                mem_flush.clone(),
            );

            if root.targets.contains(&Target::Types) {
                for enm in ns.enums.values() {
                    self.gen_enum(&ns_ctx, enm)?;
                }

                for one in ns.one_ofs.values() {
                    self.gen_one_of(&ns_ctx, one)?;
                }

                for def in ns.defs.values() {
                    self.gen_struct(&ns_ctx, def)?;
                }

                for err in ns.errors.values() {
                    self.gen_error(&ns_ctx, err)?;
                }

                for op in ns.ops.values() {
                    self.gen_operation(&ns_ctx, op)?;
                }
            }

            ctx_ns.insert(ns.name.clone(), ns_ctx);
        }

        self.with_all_namespaces(ctx, opts, ctx_ns)?;

        Ok(())
    }

    fn gen_struct(
        &self,
        state: &WithNsContext<'_, State, Ext, Self>,
        def: &Struct,
    ) -> Result<()>;

    fn gen_operation(
        &self,
        state: &WithNsContext<'_, State, Ext, Self>,
        def: &Operation,
    ) -> Result<()>;

    fn gen_enum(
        &self,
        state: &WithNsContext<'_, State, Ext, Self>,
        def: &Enum,
    ) -> Result<()>;

    fn gen_one_of(
        &self,
        state: &WithNsContext<'_, State, Ext, Self>,
        def: &OneOf,
    ) -> Result<()>;

    fn gen_error(
        &self,
        state: &WithNsContext<'_, State, Ext, Self>,
        def: &ErrorTy,
    ) -> Result<()>;
}

impl GenerationConfig {
    pub fn sources(&self) -> Result<Vec<PathBuf>> {
        matcher::walk(&self.sources)
    }

    pub fn get_ctx(&self) -> crate::Result<Context> {
        let mut ctx = Context::new();

        for source in self.sources()? {
            ctx.load_from_source(source)?;
        }

        ctx.finish()?;

        Ok(ctx)
    }

    pub fn set_mem(
        &mut self,
        mem: bool,
    ) {
        if let Some(rs) = &mut self.rust {
            rs.mem = mem;
        }
    }
}

pub struct Generation {
    pub config: GenerationConfig,
    pub ctx: Context,
}

impl Generation {
    pub fn new(config: GenerationConfig) -> Result<Self> {
        let ctx = config.get_ctx()?;
        Ok(Self { config, ctx })
    }

    fn wants_rust(&self) -> Option<&GenOpts<RustConfig>> {
        if self
            .config
            .languages
            .contains(&Language::Rust)
            && let Some(rust) = &self.config.rust
        {
            Some(rust)
        } else {
            None
        }
    }

    pub fn generate_all_sync(
        &self,
        mem_flush: Option<MemFlush>,
    ) -> crate::Result<()> {
        let mut generators = vec![];

        if let Some(rust) = self.wants_rust() {
            generators.push(Box::new(|| {
                RustGenerator.gen_ty_ctx(&self.ctx, rust, mem_flush.clone(), &self.config)
            }));
        }

        for it in generators
            .into_par_iter()
            .map(|handle| (*handle)())
            .collect::<Vec<_>>()
        {
            it?;
        }

        Ok(())
    }

    pub async fn generate_all(
        &self,
        mem_flush: Option<MemFlush>,
    ) -> crate::Result<()> {
        let mut futs = vec![];
        if let Some(rust) = self.wants_rust() {
            futs.push(Box::pin(async move {
                RustGenerator.gen_ty_ctx(&self.ctx, rust, mem_flush.clone(), &self.config)
            }));
        }

        for fut in futs {
            fut.await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{Definitions, generate::files::MemCollector};

    use super::*;

    #[test]
    fn test_gen_mem() -> crate::Result<()> {
        let mut conf = GenerationConfig::new(Some("../samples/config-a")).unwrap();
        conf.set_mem(true);

        let collector = MemCollector::new();

        let generate = Generation::new(conf)?;
        generate.generate_all_sync(Some(collector.mem_flush()))?;

        let state = collector.files();
        assert_eq!(state.keys().len(), 3);

        Ok(())
    }

    #[tokio::test]
    async fn test_config_loader() {
        let mut conf = GenerationConfig::new(Some("../samples/config-a")).unwrap();

        let overrides = [(
            Ident::new("abc.corp.test"),
            Named::<Vis>::new([(Ident::new("BasicStruct"), Vis::Crate)]),
        )];

        let expect = GenerationConfig {
            targets: vec![Target::Client, Target::Server, Target::Types],
            languages: vec![Language::Rust],
            sources: Source {
                remote: vec![RemoteConfig {
                    url: "http://localhost:9009/dynamic.toml".into(),
                    headers: Default::default(),
                }],
                include: vec![
                    "../samples/basic-op.toml",
                    "../samples/basic-struct.toml",
                    "../samples/test-enum.toml",
                    "../samples/test-str-enum.toml",
                    "../samples/test-enum-in-struct.toml",
                    "../samples/test-one-of.toml",
                    "../samples/test-operation-error.toml",
                    "../samples/test-error-code.toml",
                    "../samples/test-struct*.toml",
                ]
                .into_iter()
                .map(Into::into)
                .collect(),
                exclude: vec!["*basic-op*".into()],
            },
            rust: Some(GenOpts {
                output_dir: "examples/gen-a/src/operations".into(),
                opts: RustConfig {
                    vis: VisConfig {
                        default: Default::default(),
                        overrides: Named::new(overrides),
                    },
                    time: DateTimeLibrary::Chrono,
                },
                mem: false,
            }),
        };

        assert_eq! {
            conf, expect
        }

        let expect: Vec<_> = vec![
            "../samples/basic-struct.toml",
            "../samples/test-enum-in-struct.toml",
            "../samples/test-enum.toml",
            "../samples/test-error-code.toml",
            "../samples/test-one-of.toml",
            "../samples/test-operation-error.toml",
            "../samples/test-str-enum.toml",
            "../samples/test-struct-known-error.toml",
            "../samples/test-struct-operation-error-unknown.toml",
            "../samples/test-struct-readme.toml",
            "../samples/test-struct-text.toml",
            "../samples/test-struct-with-chrono.toml",
            "../samples/test-struct-with-enum.toml",
            "../samples/test-struct-with-one-of.toml",
            "../samples/test-struct-with-time.toml",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect();

        assert_eq! {
            conf.sources().unwrap(),
            expect
        }

        let ctx = conf.get_ctx().unwrap();
        let ns = ctx
            .namespaces
            .get(&"abc.corp.namespace".into())
            .unwrap();

        Definitions::NamespaceV1(ns.clone())
            .export("../samples/abc_corp_namespace.toml".into())
            .unwrap();

        kintsu_testing::insta_test!(|| {
            kintsu_testing::assert_yaml_snapshot!(ctx.namespaces);
        });

        conf.set_mem(true);

        let collector = MemCollector::new();
        let generator = Generation::new(conf).unwrap();

        generator
            .generate_all(Some(collector.mem_flush()))
            .await
            .unwrap();
    }
}
