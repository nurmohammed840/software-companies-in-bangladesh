use crate::Result;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct TextFile {
    pub text: String,
    pub path: PathBuf,
}

impl TextFile {
    pub fn read(path: PathBuf) -> Result<Self> {
        let mut text = String::new();
        log_file(&path, false)?.read_to_string(&mut text)?;
        Ok(Self { text, path })
    }

    pub fn write(&self, c: impl AsRef<[u8]>) -> io::Result<()> {
        let old = self.text.as_bytes();
        let new = c.as_ref();

        if old == new {
            return Ok(());
        }

        fs::write(&self.path, new)
    }
}

pub type LogFile = Arc<File>;

pub fn open_log_file(path: impl AsRef<Path>) -> Result<LogFile> {
    log_file(path, true)
}

fn log_file(path: impl AsRef<Path>, truncate: bool) -> Result<LogFile> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(truncate)
        .open(path)?;

    Ok(Arc::new(file))
}
