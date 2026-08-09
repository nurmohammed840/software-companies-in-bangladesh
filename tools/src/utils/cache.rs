use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

use crate::{Result, utils::zlib};
use std::{env, fs, path::PathBuf};

pub struct Cache {
    pub path: PathBuf,
}

impl Cache {
    pub fn open(path: &str, key: &str) -> Result<Self> {
        Ok(Self {
            path: tmp_cache_dir(path)?.join(to_filename(key)),
        })
    }

    pub fn get(&self) -> Result<Option<String>> {
        if !self.path.is_file() {
            return Ok(None);
        }

        let data = fs::read(&self.path)
            .map(zlib::decompress)?
            .map(String::from_utf8)??;

        Ok(Some(data))
    }

    pub fn set(&self, data: impl AsRef<[u8]>) -> Result {
        fs::write(&self.path, zlib::compress(data)?)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove(&self) -> Result<bool> {
        if self.path.is_file() {
            fs::remove_file(&self.path)?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn clear(path: &str) -> Result {
        let dir = tmp_cache_dir(path)?;

        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_file() {
                fs::remove_file(path)?;
            }
        }

        Ok(())
    }
}

pub fn to_filename(url: &str) -> String {
    utf8_percent_encode(url, NON_ALPHANUMERIC).to_string()
}

pub fn tmp_cache_dir(path: &str) -> Result<PathBuf> {
    let path = env::temp_dir().join(path);
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    Ok(path)
}
