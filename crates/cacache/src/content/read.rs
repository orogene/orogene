use std::fs::{self, File};
use std::path::{ PathBuf, Path };
use std::io::{ Read, BufReader };

use ssri::{Algorithm, Integrity, IntegrityChecker};
use memmap::{ MmapMut, Mmap };

use crate::content::path;
use crate::errors::{Internal, Result};

struct MaybeMmap {
    mmap: Option<(Mmap, usize)>,
    file: BufReader<File>,
}

impl std::io::Read for MaybeMmap {
    #[inline]
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some((mmap, pos)) = self.mmap.as_mut() {
            match (&mmap[*pos..]).read(&mut buf) {
                Ok(read) => {
                    *pos += read;
                    Ok(read)
                },
                Err(e) => Err(e)
            }
        } else {
            self.file.read(&mut buf)
        }
    }
}

pub struct Reader {
    fd: flate2::read::DeflateDecoder<MaybeMmap>,
    checker: IntegrityChecker,
    expected_size: usize,
}

impl std::io::Read for Reader {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let amt = self.fd.read(buf)?;
        self.checker.input(&buf[..amt]);
        Ok(amt)
    }
}

pub const MAX_MMAP_READ_SIZE: usize = 1024 * 1024 * 10;
pub const MIN_MMAP_READ_SIZE: usize = 1024 * 1024;

impl Reader {
    pub fn check(self) -> Result<Algorithm> {
        Ok(self.checker.result()?)
    }

    fn instantiate(cpath: PathBuf, sri: Integrity) -> Result<Self> {
        let mut reader = File::open(cpath).to_internal()?;
        let mut bytes = [0u8; 8];
        reader.read_exact(&mut bytes).to_internal()?;
        let expected_size = u64::from_be_bytes(bytes) as usize;

        let fd = MaybeMmap {
            mmap: if expected_size >= MIN_MMAP_READ_SIZE && expected_size <= MAX_MMAP_READ_SIZE {
                unsafe { Mmap::map(&reader) }.ok().map(|mmap| (mmap, 8))
            } else {
                None
            },
            file: BufReader::new(reader)
        };

        Ok(Reader {
            fd: flate2::read::DeflateDecoder::new(fd),
            checker: IntegrityChecker::new(sri),
            expected_size,
        })
    }

    pub fn new(cache: &Path, sri: &Integrity) -> Result<Self> {
        let cpath = path::content_path(&cache, &sri);
        let sri = sri.clone();
        Self::instantiate(cpath, sri)
    }

    pub fn consume(mut self) -> Result<Vec<u8>> {
        let mut v = Vec::with_capacity(self.expected_size);
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
    let mut target = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(to)
        .to_internal()?;

    let ret = if reader.expected_size > 0 {
        if let Ok(mut mmap) = unsafe { MmapMut::map_mut(&target) } {
            let mut cursor = std::io::Cursor::new(&mut mmap[..]);
            std::io::copy(&mut reader, &mut cursor).to_internal()
        } else {
            std::io::copy(&mut reader, &mut target).to_internal()
        }
    } else {
        std::io::copy(&mut reader, &mut target).to_internal()
    }?;

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
