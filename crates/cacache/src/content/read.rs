use std::fs::{self, File};
use std::path::{ PathBuf, Path };
use std::io::{ Read, BufReader };

use ssri::{Algorithm, Integrity, IntegrityChecker};

use crate::content::path;
use crate::errors::{Internal, Result};

pub struct Reader {
    fd: flate2::read::DeflateDecoder<BufReader<File>>,
    checker: IntegrityChecker,
}

impl std::io::Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let amt = self.fd.read(buf)?;
        self.checker.input(&buf[..amt]);
        Ok(amt)
    }
}

impl Reader {
    pub fn check(self) -> Result<Algorithm> {
        Ok(self.checker.result()?)
    }

    fn instantiate(cpath: PathBuf, sri: Integrity) -> Result<Self> {
        Ok(Reader {
            fd: flate2::read::DeflateDecoder::new(BufReader::new(File::open(cpath).to_internal()?)),
            checker: IntegrityChecker::new(sri),
        })
    }

    pub fn new(cache: &Path, sri: &Integrity) -> Result<Self> {
        let cpath = path::content_path(&cache, &sri);
        let sri = sri.clone();
        Self::instantiate(cpath, sri)
    }

    pub fn consume(mut self) -> Result<Vec<u8>> {
        let mut v = Vec::new();
        self.read_to_end(&mut v).to_internal()?;
        self.check()?;
        Ok(v)
    }

    pub async fn new_async(cache: &Path, sri: &Integrity) -> Result<Self> {
        let cpath = path::content_path(&cache, &sri);
        let sri = sri.clone();
        smol::unblock!(Self::instantiate(cpath, sri))
    }

    pub async fn consume_async(self) -> Result<Vec<u8>> {
        smol::unblock!(self.consume())
    }
}

pub fn open(cache: &Path, sri: Integrity) -> Result<Reader> {
    Reader::new(cache, &sri)
}

pub async fn open_async(cache: &Path, sri: Integrity) -> Result<Reader> {
    Reader::new_async(cache, &sri).await
}

pub fn read(cache: &Path, sri: &Integrity) -> Result<Vec<u8>> {
    Reader::new(cache, sri)?.consume()
}

pub async fn read_async<'a>(cache: &Path, sri: &Integrity) -> Result<Vec<u8>> {
    Reader::new_async(cache, sri).await?.consume_async().await
}

pub fn copy(cache: &Path, sri: &Integrity, to: &Path) -> Result<u64> {
    let mut reader = Reader::new(cache, sri)?;
    // TODO: if we know the size of the file coming out, we could copy via mmap
    let mut target = fs::OpenOptions::new().write(true).create(true).truncate(true).open(to).to_internal()?;
    let ret = std::io::copy(&mut reader, &mut target).to_internal()?; 
    reader.check()?;
    Ok(ret)
}

pub async fn copy_async<'a>(cache: &'a Path, sri: &'a Integrity, to: &'a Path) -> Result<u64> {
    let cache = cache.to_owned();
    let sri = sri.to_owned();
    let to = to.to_owned();
    smol::unblock!(copy(&cache, &sri, &to))
}

pub fn has_content(cache: &Path, sri: &Integrity) -> Option<Integrity> {
    if path::content_path(&cache, &sri).exists() {
        Some(sri.clone())
    } else {
        None
    }
}

pub async fn has_content_async(cache: &Path, sri: &Integrity) -> Option<Integrity> {
    if async_std::fs::metadata(path::content_path(&cache, &sri))
        .await
        .is_ok()
    {
        Some(sri.clone())
    } else {
        None
    }
}
