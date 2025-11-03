use std::{
    collections::BTreeMap,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

pub trait WithFlush: Write {
    fn with_flush(
        &mut self,
        flush: MemFlush,
    );
}

pub struct File {
    f: std::fs::File,
}

pub struct Mem {
    #[allow(unused)]
    pub fname: PathBuf,
    pub fdata: Vec<u8>,
}

pub type MemFlush = Arc<dyn Fn(&mut Mem) + Sync + Send>;

pub enum FileOrMem {
    File(File),
    Mem { state: Mem, flush: MemFlush },
}

impl FileOrMem {
    pub fn new<P: Into<PathBuf>>(
        fname: P,
        mem: bool,
    ) -> std::io::Result<Self> {
        let fname = fname.into();
        Ok(if !mem {
            let f = std::fs::File::create(fname)?;
            Self::File(File { f })
        } else {
            Self::Mem {
                flush: Arc::new(|_| {}),
                state: Mem {
                    fname,
                    fdata: vec![],
                },
            }
        })
    }
}

impl Write for FileOrMem {
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::File(f) => f.f.flush(),
            Self::Mem { flush, state } => {
                let _: () = (flush)(state);
                Ok(())
            },
        }
    }

    fn write(
        &mut self,
        buf: &[u8],
    ) -> std::io::Result<usize> {
        Ok(match self {
            Self::File(f) => f.f.write(buf)?,
            Self::Mem { state, .. } => {
                let sz = buf.len();
                state.fdata.append(&mut buf.to_vec());
                sz
            },
        })
    }
}

impl WithFlush for FileOrMem {
    fn with_flush(
        &mut self,
        new_fn: MemFlush,
    ) {
        match self {
            Self::File(..) => (),
            Self::Mem { flush, .. } => {
                *flush = new_fn;
            },
        }
    }
}

#[derive(Default)]
pub struct MemCollector {
    files: Arc<Mutex<BTreeMap<PathBuf, Vec<u8>>>>,
}

impl MemCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mem_flush(&self) -> MemFlush {
        let fref = self.files.clone();
        Arc::new(move |flush| {
            tracing::trace!(
                "flush {} bytes to {}",
                flush.fdata.len(),
                flush.fname.display()
            );

            let mut flushed = fref.lock().unwrap();
            match flushed.get_mut(&flush.fname) {
                Some(buf) => {
                    *buf = flush.fdata.clone();
                },
                None => {
                    flushed.insert(flush.fname.clone(), flush.fdata.clone());
                },
            }
        })
    }

    pub fn files(&self) -> MutexGuard<'_, BTreeMap<PathBuf, Vec<u8>>> {
        self.files.lock().expect("file lock")
    }
}

#[cfg(test)]
mod test {
    use std::{io::Write, path::PathBuf};

    use crate::generate::files::WithFlush;

    #[test]
    fn test_file() -> crate::Result<()> {
        let t = tempfile::tempdir().unwrap();

        let txt = t.path().join("temp.txt");
        let mut file = super::FileOrMem::new(&txt, false)?;
        file.write("test".as_bytes())?;
        file.flush()?;

        let data = std::fs::read_to_string(txt)?;
        assert_eq!(data, "test");
        Ok(())
    }

    #[test]
    fn test_mem() -> crate::Result<()> {
        let p: PathBuf = "temp.txt".into();

        let collector = super::MemCollector::new();
        let mut file = super::FileOrMem::new(&p, true)?;

        file.with_flush(collector.mem_flush());
        let _ = file.write("test".as_bytes())?;
        file.flush()?;

        let state = collector.files();
        let out = String::from_utf8_lossy(state.get(&p).unwrap());

        assert_eq!(out, "test");

        Ok(())
    }
}
