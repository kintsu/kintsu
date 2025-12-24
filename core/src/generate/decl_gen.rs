//! Code generation from parser declaration types.
//!
//! This module provides the `GenerateDecl` trait for generating code from
//! `DeclarationBundle` and related parser types.

use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::{
    declare::{
        DeclEnumDef, DeclError, DeclNamespace, DeclOneOf, DeclOperation, DeclStruct,
        DeclarationBundle, TypeDefinition, TypeRegistryDeclaration,
    },
    generate::{
        ConfigExt, GenOpts, LanguageTrait, Result, Target,
        files::{MemFlush, WithFlush},
    },
};

type OnCreateDecl<'ns, T, Config, L> = Box<
    dyn (Fn(
            &DeclNsContext<'ns, T, Config, L>,
            &PathBuf,
            &mut Box<dyn WithFlush>,
        ) -> std::io::Result<()>)
        + Send
        + Sync,
>;

pub struct DeclNsContext<'ns, T, Config: ConfigExt, L: LanguageTrait> {
    pub ns: &'ns DeclNamespace,
    pub state: Arc<T>,
    pub opts: &'ns GenOpts<Config>,
    files: Mutex<BTreeMap<PathBuf, Box<dyn WithFlush>>>,
    on_create: OnCreateDecl<'ns, T, Config, L>,
    mem_flush: Option<MemFlush>,
    _marker: std::marker::PhantomData<L>,
}

impl<'ns, T, Config: ConfigExt, L: LanguageTrait> DeclNsContext<'ns, T, Config, L> {
    pub fn new(
        ns: &'ns DeclNamespace,
        state: Arc<T>,
        opts: &'ns GenOpts<Config>,
        on_create: OnCreateDecl<'ns, T, Config, L>,
        mem_flush: Option<MemFlush>,
    ) -> Self {
        Self {
            ns,
            state,
            opts,
            on_create,
            files: Default::default(),
            mem_flush,
            _marker: Default::default(),
        }
    }

    fn new_file(
        &self,
        name: &PathBuf,
    ) -> Result<()> {
        tracing::info!("creating '{}'", name.display());

        if !self.opts.mem && !name.parent().unwrap().exists() {
            let parent = name.parent().unwrap();
            tracing::info!("mkdir '{}'", parent.display());
            std::fs::create_dir_all(parent)?;
        }

        let mut new_f = Box::new(super::files::FileOrMem::new(name.clone(), self.opts.mem)?)
            as Box<dyn WithFlush>;

        if self.opts.mem {
            if let Some(mem_flush) = self.mem_flush.clone() {
                new_f.with_flush(mem_flush);
            }
        }

        (self.on_create)(self, name, &mut new_f)?;

        let mut f = self.files.lock().unwrap();
        f.insert(name.clone(), new_f);

        Ok(())
    }

    pub fn with_file_handle<F: Fn(&mut Box<dyn WithFlush>) -> std::io::Result<()>>(
        &self,
        name: PathBuf,
        handle: F,
    ) -> Result<()> {
        let f = self.files.lock().unwrap();
        let contains = f.contains_key(&name);
        drop(f);

        if !contains {
            self.new_file(&name)?;
        }

        let mut f = self.files.lock().unwrap();
        let file = f.get_mut(&name).unwrap();
        (handle)(file)?;
        file.flush()?;

        Ok(())
    }

    pub fn path_for_file<S: AsRef<Path> + Clone>(
        &self,
        name: S,
    ) -> PathBuf {
        self.opts.output_dir.join(L::file_name(name))
    }

    pub fn ns_file(&self) -> PathBuf {
        let norm_path = self
            .ns
            .name
            .replace('.', "/")
            .replace('-', "_");
        self.path_for_file(PathBuf::from(norm_path))
    }
}

pub trait GenerateDecl<State: Sync + Send, Ext: ConfigExt + Sync + Send>
where
    Self: LanguageTrait + Sized + Sync + Send, {
    fn on_create_decl(
        state: &DeclNsContext<'_, State, Ext, Self>,
        fname: &Path,
        f: &mut Box<dyn WithFlush>,
    ) -> std::io::Result<()>;

    fn new_state_decl(
        &self,
        opts: &GenOpts<Ext>,
    ) -> State;

    fn gen_from_bundle(
        &self,
        bundle: &DeclarationBundle,
        opts: &GenOpts<Ext>,
        mem_flush: Option<MemFlush>,
        targets: &[Target],
    ) -> Result<()> {
        self.gen_from_registry(&bundle.root, opts, mem_flush.clone(), targets)?;

        for dep in bundle.dependencies.values() {
            self.gen_from_registry(dep, opts, mem_flush.clone(), targets)?;
        }

        Ok(())
    }

    fn gen_from_registry(
        &self,
        registry: &TypeRegistryDeclaration,
        opts: &GenOpts<Ext>,
        mem_flush: Option<MemFlush>,
        targets: &[Target],
    ) -> Result<()> {
        let state = Arc::new(self.new_state_decl(opts));

        for ns in registry.namespaces.values() {
            self.gen_namespace(ns, state.clone(), opts, mem_flush.clone(), targets)?;
        }

        Ok(())
    }

    fn gen_namespace(
        &self,
        ns: &DeclNamespace,
        state: Arc<State>,
        opts: &GenOpts<Ext>,
        mem_flush: Option<MemFlush>,
        targets: &[Target],
    ) -> Result<()> {
        let ns_ctx = DeclNsContext::new(
            ns,
            state.clone(),
            opts,
            Box::new(|state, fname, f| Self::on_create_decl(state, fname, f)),
            mem_flush.clone(),
        );

        if targets.contains(&Target::Types) {
            for type_def in &ns.types {
                match type_def {
                    TypeDefinition::Struct(s) => self.gen_decl_struct(&ns_ctx, s)?,
                    TypeDefinition::Enum(e) => self.gen_decl_enum(&ns_ctx, e)?,
                    TypeDefinition::OneOf(o) => self.gen_decl_one_of(&ns_ctx, o)?,
                    TypeDefinition::Error(e) => self.gen_decl_error(&ns_ctx, e)?,
                    TypeDefinition::Operation(op) => self.gen_decl_operation(&ns_ctx, op)?,
                    TypeDefinition::TypeAlias(_) => {},
                }
            }
        }

        for child_ns in ns.namespaces.values() {
            self.gen_namespace(child_ns, state.clone(), opts, mem_flush.clone(), targets)?;
        }

        Ok(())
    }

    fn gen_decl_struct(
        &self,
        state: &DeclNsContext<'_, State, Ext, Self>,
        def: &DeclStruct,
    ) -> Result<()>;

    fn gen_decl_operation(
        &self,
        state: &DeclNsContext<'_, State, Ext, Self>,
        def: &DeclOperation,
    ) -> Result<()>;

    fn gen_decl_enum(
        &self,
        state: &DeclNsContext<'_, State, Ext, Self>,
        def: &DeclEnumDef,
    ) -> Result<()>;

    fn gen_decl_one_of(
        &self,
        state: &DeclNsContext<'_, State, Ext, Self>,
        def: &DeclOneOf,
    ) -> Result<()>;

    fn gen_decl_error(
        &self,
        state: &DeclNsContext<'_, State, Ext, Self>,
        def: &DeclError,
    ) -> Result<()>;
}
