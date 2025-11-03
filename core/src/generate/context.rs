use std::{
    cell::Cell,
    collections::BTreeMap,
    io::Write,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::{
    generate::{
        ConfigExt, GenOpts, LanguageTrait, RustConfig,
        files::{MemFlush, WithFlush},
        rust::RustGenState,
    },
    namespace::Namespace,
};

type OnCreate<'ns, T, Config, L> = Box<
    dyn (Fn(
            &WithNsContext<'ns, T, Config, L>,
            &PathBuf,
            &mut Box<dyn WithFlush>,
        ) -> std::io::Result<()>)
        + Send
        + Sync,
>;

thread_local! {
    pub static RUST_CALLBACK: Cell<Option<OnCreate<'static,RustGenState, RustConfig, super::rust::RustGenerator >>> = Cell::new(None);
}

pub struct WithNsContext<'ns, T, Config: ConfigExt, L: LanguageTrait> {
    pub ns: &'ns Namespace,
    pub state: Arc<T>,
    pub opts: &'ns GenOpts<Config>,

    files: Mutex<BTreeMap<PathBuf, Box<dyn WithFlush>>>,

    on_create: OnCreate<'ns, T, Config, L>,
    ph: PhantomData<L>,
    mem_flush: Option<MemFlush>,
}

impl<'ns, T, Config: ConfigExt, L: LanguageTrait> WithNsContext<'ns, T, Config, L> {
    pub fn new(
        ns: &'ns Namespace,
        state: Arc<T>,
        opts: &'ns GenOpts<Config>,
        on_create: OnCreate<'ns, T, Config, L>,
        mem_flush: Option<MemFlush>,
    ) -> Self {
        Self {
            ns,
            opts,
            state,
            on_create,
            files: Default::default(),
            ph: Default::default(),
            mem_flush,
        }
    }
}

impl<'ns, T, Config: ConfigExt, L: LanguageTrait> WithNsContext<'ns, T, Config, L> {
    fn new_file(
        &self,
        name: &PathBuf,
    ) -> crate::generate::Result<()> {
        tracing::info!("creating '{}'", name.display());

        if !self.opts.mem && !name.parent().unwrap().exists() {
            let parent = name.parent().unwrap();

            tracing::info!("mkdir '{}'", parent.display());

            std::fs::create_dir_all(parent)?;
        }

        let mut new_f = Box::new(super::files::FileOrMem::new(name.clone(), self.opts.mem)?)
            as Box<dyn WithFlush>;

        if self.opts.mem
            && let Some(mem_flush) = self.mem_flush.clone()
        {
            new_f.with_flush(mem_flush);
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
    ) -> crate::generate::Result<()> {
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
        let norm_path = self.ns.normalized_path::<L>();
        self.path_for_file(norm_path)
    }
}
