use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use async_std::fs as afs;
use async_std::io::BufReader;
use digest::Digest;
use either::{Left, Right};
use futures::io::{AsyncBufReadExt, AsyncWriteExt};
use futures::stream::StreamExt;
use hex;
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::Sha1;
use sha2::Sha256;
use ssri::Integrity;
use walkdir::WalkDir;

use crate::errors::{Internal, InternalResult, Result};
use crate::put::WriteOpts;

const INDEX_VERSION: &str = "5";

/// Represents a cache index entry, which points to content.
#[derive(PartialEq, Debug)]
pub struct Metadata {
    /// Key this entry is stored under.
    pub key: String,
    /// Integrity hash for the stored data. Acts as a key into {cache}/content.
    pub integrity: Integrity,
    /// Timestamp in unix milliseconds when this entry was written.
    pub time: u128,
    /// Size of data associated with this entry.
    pub size: usize,
    /// Arbitrary JSON  associated with this entry.
    pub metadata: Value,
}

#[derive(Deserialize, Serialize, Debug)]
struct SerializableMetadata {
    key: String,
    integrity: Option<String>,
    time: u128,
    size: usize,
    metadata: Value,
}

impl PartialEq for SerializableMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for SerializableMetadata {}

impl Hash for SerializableMetadata {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

pub fn insert(cache: &Path, key: &str, opts: WriteOpts) -> Result<Integrity> {
    let bucket = bucket_path(&cache, &key);
    fs::create_dir_all(bucket.parent().unwrap()).with_context(|| {
        format!(
            "Failed to create index bucket directory: {:?}",
            bucket.parent().unwrap()
        )
    })?;
    let stringified = serde_json::to_string(&SerializableMetadata {
        key: key.to_owned(),
        integrity: opts.sri.clone().map(|x| x.to_string()),
        time: opts.time.unwrap_or_else(now),
        size: opts.size.unwrap_or(0),
        metadata: opts.metadata.unwrap_or_else(|| json!(null)),
    })
    .with_context(|| format!("Failed to serialize entry with key `{}`", key))?;

    let mut buck = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&bucket)
        .with_context(|| format!("Failed to create or open index bucket at {:?}", bucket))?;

    let out = format!("\n{}\t{}", hash_entry(&stringified), stringified);
    buck.write_all(out.as_bytes())
        .with_context(|| format!("Failed to write to index bucket at {:?}", bucket))?;
    buck.flush()
        .with_context(|| format!("Failed to flush bucket at {:?}", bucket))?;
    Ok(opts
        .sri
        .or_else(|| "sha1-deadbeef".parse::<Integrity>().ok())
        .unwrap())
}

pub async fn insert_async<'a>(cache: &'a Path, key: &'a str, opts: WriteOpts) -> Result<Integrity> {
    let bucket = bucket_path(&cache, &key);
    afs::create_dir_all(bucket.parent().unwrap())
        .await
        .with_context(|| {
            format!(
                "Failed to create index bucket directory: {:?}",
                bucket.parent().unwrap()
            )
        })?;
    let stringified = serde_json::to_string(&SerializableMetadata {
        key: key.to_owned(),
        integrity: opts.sri.clone().map(|x| x.to_string()),
        time: opts.time.unwrap_or_else(now),
        size: opts.size.unwrap_or(0),
        metadata: opts.metadata.unwrap_or_else(|| json!(null)),
    })
    .with_context(|| format!("Failed to serialize entry with key `{}`", key))?;

    let mut buck = async_std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&bucket)
        .await
        .with_context(|| format!("Failed to create or open index bucket at {:?}", bucket))?;

    let out = format!("\n{}\t{}", hash_entry(&stringified), stringified);
    buck.write_all(out.as_bytes())
        .await
        .with_context(|| format!("Failed to write to index bucket at {:?}", bucket))?;
    buck.flush()
        .await
        .with_context(|| format!("Failed to flush bucket at {:?}", bucket))?;
    Ok(opts
        .sri
        .or_else(|| "sha1-deadbeef".parse::<Integrity>().ok())
        .unwrap())
}

pub fn find(cache: &Path, key: &str) -> Result<Option<Metadata>> {
    let bucket = bucket_path(cache, &key);
    Ok(bucket_entries(&bucket)
        .with_context(|| format!("Failed to read index bucket entries from {:?}", bucket))?
        .into_iter()
        .fold(None, |acc, entry| {
            if entry.key == key {
                if let Some(integrity) = entry.integrity {
                    let integrity: Integrity = match integrity.parse() {
                        Ok(sri) => sri,
                        _ => return acc,
                    };
                    Some(Metadata {
                        key: entry.key,
                        integrity,
                        size: entry.size,
                        time: entry.time,
                        metadata: entry.metadata,
                    })
                } else {
                    None
                }
            } else {
                acc
            }
        }))
}

pub async fn find_async(cache: &Path, key: &str) -> Result<Option<Metadata>> {
    let bucket = bucket_path(cache, &key);
    Ok(bucket_entries_async(&bucket)
        .await
        .with_context(|| format!("Failed to read index bucket entries from {:?}", bucket))?
        .into_iter()
        .fold(None, |acc, entry| {
            if entry.key == key {
                if let Some(integrity) = entry.integrity {
                    let integrity: Integrity = match integrity.parse() {
                        Ok(sri) => sri,
                        _ => return acc,
                    };
                    Some(Metadata {
                        key: entry.key,
                        integrity,
                        size: entry.size,
                        time: entry.time,
                        metadata: entry.metadata,
                    })
                } else {
                    None
                }
            } else {
                acc
            }
        }))
}

pub fn delete(cache: &Path, key: &str) -> Result<()> {
    insert(
        cache,
        key,
        WriteOpts {
            algorithm: None,
            size: None,
            sri: None,
            time: None,
            metadata: None,
        },
    )
    .map(|_| ())
}

pub async fn delete_async(cache: &Path, key: &str) -> Result<()> {
    insert(
        cache,
        key,
        WriteOpts {
            algorithm: None,
            size: None,
            sri: None,
            time: None,
            metadata: None,
        },
    )
    .map(|_| ())
}

pub fn ls(cache: &Path) -> impl Iterator<Item = Result<Metadata>> {
    WalkDir::new(cache.join(format!("index-v{}", INDEX_VERSION)))
        .into_iter()
        .map(|bucket| {
            let bucket = bucket.to_internal()?;

            if bucket.file_type().is_dir() {
                return Ok(Vec::new());
            }

            Ok(bucket_entries(bucket.path())?
                .into_iter()
                .collect::<HashSet<SerializableMetadata>>()
                .into_iter()
                .filter_map(|se| {
                    if let Some(i) = se.integrity {
                        Some(Metadata {
                            key: se.key,
                            integrity: i.parse().unwrap(),
                            time: se.time,
                            size: se.size,
                            metadata: se.metadata,
                        })
                    } else {
                        None
                    }
                })
                .collect())
        })
        .flat_map(|res| match res {
            Ok(it) => Left(it.into_iter().map(Ok)),
            Err(err) => Right(std::iter::once(Err(err))),
        })
}

fn bucket_path(cache: &Path, key: &str) -> PathBuf {
    let hashed = hash_key(&key);
    cache
        .join(format!("index-v{}", INDEX_VERSION))
        .join(&hashed[0..2])
        .join(&hashed[2..4])
        .join(&hashed[4..])
}

fn hash_key(key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.input(&key);
    hex::encode(hasher.result())
}

fn hash_entry(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.input(&key);
    hex::encode(hasher.result())
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn bucket_entries(bucket: &Path) -> InternalResult<Vec<SerializableMetadata>> {
    use std::io::{BufRead, BufReader};
    fs::File::open(bucket)
        .map(|file| {
            BufReader::new(file)
                .lines()
                .filter_map(std::result::Result::ok)
                .filter_map(|entry| {
                    let entry_str = match entry.split('\t').collect::<Vec<&str>>()[..] {
                        [hash, entry_str] if hash_entry(entry_str) == hash => entry_str,
                        // Something's wrong with the entry. Abort.
                        _ => return None,
                    };
                    serde_json::from_str::<SerializableMetadata>(entry_str).ok()
                })
                .collect()
        })
        .or_else(|err| {
            if err.kind() == ErrorKind::NotFound {
                Ok(Vec::new())
            } else {
                Err(err).to_internal()?
            }
        })
}

async fn bucket_entries_async(bucket: &Path) -> InternalResult<Vec<SerializableMetadata>> {
    let file_result = afs::File::open(bucket).await;
    let file;
    if let Err(err) = file_result {
        if err.kind() == ErrorKind::NotFound {
            return Ok(Vec::new());
        }
        return Err(err).to_internal()?;
    } else {
        file = file_result.unwrap();
    }
    let mut vec = Vec::new();
    let mut lines = BufReader::new(file).lines();
    while let Some(line) = lines.next().await {
        if let Ok(entry) = line {
            let entry_str = match entry.split('\t').collect::<Vec<&str>>()[..] {
                [hash, entry_str] if hash_entry(entry_str) == hash => entry_str,
                // Something's wrong with the entry. Abort.
                _ => continue,
            };
            if let Ok(serialized) = serde_json::from_str::<SerializableMetadata>(entry_str) {
                vec.push(serialized);
            }
        }
    }
    Ok(vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::task;
    use tempfile;

    const MOCK_ENTRY: &str = "\n251d18a2b33264ea8655695fd23c88bd874cdea2c3dc9d8f9b7596717ad30fec\t{\"key\":\"hello\",\"integrity\":\"sha1-deadbeef\",\"time\":1234567,\"size\":0,\"metadata\":null}";

    #[test]
    fn insert_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let sri: Integrity = "sha1-deadbeef".parse().unwrap();
        let time = 1_234_567;
        let opts = WriteOpts::new().integrity(sri).time(time);
        insert(&dir, "hello", opts).unwrap();
        let entry = std::fs::read_to_string(bucket_path(&dir, "hello")).unwrap();
        assert_eq!(entry, MOCK_ENTRY);
    }

    #[test]
    fn insert_async_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let sri: Integrity = "sha1-deadbeef".parse().unwrap();
        let time = 1_234_567;
        let opts = WriteOpts::new().integrity(sri).time(time);
        task::block_on(async {
            insert_async(&dir, "hello", opts).await.unwrap();
        });
        let entry = std::fs::read_to_string(bucket_path(&dir, "hello")).unwrap();
        assert_eq!(entry, MOCK_ENTRY);
    }

    #[test]
    fn find_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let sri: Integrity = "sha1-deadbeef".parse().unwrap();
        let time = 1_234_567;
        let bucket = bucket_path(&dir, "hello");
        fs::create_dir_all(bucket.parent().unwrap()).unwrap();
        fs::write(bucket, MOCK_ENTRY).unwrap();
        let entry = find(&dir, "hello").unwrap().unwrap();
        assert_eq!(
            entry,
            Metadata {
                key: String::from("hello"),
                integrity: sri,
                time,
                size: 0,
                metadata: json!(null)
            }
        );
    }

    #[test]
    fn find_none() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        assert_eq!(find(&dir, "hello").unwrap(), None);
    }

    #[test]
    fn delete_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let sri: Integrity = "sha1-deadbeef".parse().unwrap();
        let time = 1_234_567;
        let opts = WriteOpts::new().integrity(sri).time(time);
        insert(&dir, "hello", opts).unwrap();
        delete(&dir, "hello").unwrap();
        assert_eq!(find(&dir, "hello").unwrap(), None);
    }

    #[test]
    fn delete_async_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let sri: Integrity = "sha1-deadbeef".parse().unwrap();
        let time = 1_234_567;
        let opts = WriteOpts::new().integrity(sri).time(time);
        insert(&dir, "hello", opts).unwrap();
        task::block_on(async {
            delete_async(&dir, "hello").await.unwrap();
        });
        assert_eq!(find(&dir, "hello").unwrap(), None);
    }

    #[test]
    fn round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let sri: Integrity = "sha1-deadbeef".parse().unwrap();
        let time = 1_234_567;
        let opts = WriteOpts::new().integrity(sri.clone()).time(time);
        insert(&dir, "hello", opts).unwrap();
        let entry = find(&dir, "hello").unwrap().unwrap();
        assert_eq!(
            entry,
            Metadata {
                key: String::from("hello"),
                integrity: sri,
                time,
                size: 0,
                metadata: json!(null)
            }
        );
    }

    #[test]
    fn round_trip_async() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let sri: Integrity = "sha1-deadbeef".parse().unwrap();
        let time = 1_234_567;
        let opts = WriteOpts::new().integrity(sri.clone()).time(time);
        task::block_on(async {
            insert_async(&dir, "hello", opts).await.unwrap();
        });
        let entry = task::block_on(async { find_async(&dir, "hello").await.unwrap().unwrap() });
        assert_eq!(
            entry,
            Metadata {
                key: String::from("hello"),
                integrity: sri,
                time,
                size: 0,
                metadata: json!(null)
            }
        );
    }

    #[test]
    fn ls_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();
        let sri: Integrity = "sha1-deadbeef".parse().unwrap();
        let time = 1_234_567;
        let opts = WriteOpts::new().integrity(sri.clone()).time(time);
        insert(&dir, "hello", opts).unwrap();
        let opts = WriteOpts::new().integrity(sri).time(time);
        insert(&dir, "world", opts).unwrap();

        let mut entries = ls(&dir)
            .map(|x| Ok(x?.key))
            .collect::<Result<Vec<_>>>()
            .unwrap();
        entries.sort();
        assert_eq!(entries, vec![String::from("hello"), String::from("world")])
    }
}
