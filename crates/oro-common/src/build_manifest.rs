use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{Bin, Directories};

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBuildManifest {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub bin: Option<Bin>,

    #[serde(default)]
    pub directories: Option<Directories>,

    #[serde(default)]
    pub scripts: HashMap<String, String>,
}

/// Manifest intended for use with the `build` step in orogene's installer. It
/// reads and normalizes a package.json's bins (including the
/// `directories.bin` field), and its scripts object.
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildManifest {
    /// Mapping of bin name to the relative path to the script/binary.
    #[serde(default)]
    pub bin: HashMap<String, PathBuf>,

    /// package.json scripts object.
    #[serde(default)]
    pub scripts: HashMap<String, String>,
}

impl BuildManifest {
    /// Create a new [`BuildManifest`] from a given path, normalizing its bin
    /// field (or its `directories.bin`) into a plain HashMap.
    pub fn from_path(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        let pkg_str = std::fs::read_to_string(path)?;
        let raw: RawBuildManifest = serde_json::from_str(&pkg_str)?;
        Self::normalize(raw)
    }

    fn normalize(raw: RawBuildManifest) -> std::io::Result<Self> {
        let mut bin_map = HashMap::new();
        if let Some(Bin::Hash(bins)) = raw.bin {
            for (name, bin) in &bins {
                let base = Path::new(name).file_name();
                if base.is_none() || base == Some(OsStr::new("")) {
                    continue;
                }
                let base = Path::new("/")
                    .join(Path::new(
                        &base
                            .unwrap()
                            .to_string_lossy()
                            .to_string()
                            .replace(['\\', ':'], "/"),
                    ))
                    .strip_prefix(
                        #[cfg(windows)]
                        "\\",
                        #[cfg(not(windows))]
                        "/",
                    )
                    .expect("We added this ourselves")
                    .to_path_buf();
                if base == Path::new("") {
                    continue;
                }

                let bin_target = Path::new("/")
                    .join(bin.to_string_lossy().to_string())
                    .strip_prefix(
                        #[cfg(windows)]
                        "\\",
                        #[cfg(not(windows))]
                        "/",
                    )
                    .expect("We added this ourselves")
                    .to_path_buf();
                if bin_target == Path::new("") {
                    continue;
                }

                bin_map.insert(base.to_string_lossy().to_string(), bin_target);
            }
        } else if let Some(Bin::Str(bin)) = raw.bin {
            let mut bin_map = HashMap::new();
            if let Some(name) = raw.name {
                bin_map.insert(name, PathBuf::from(bin));
            }
        } else if let Some(Bin::Array(bins)) = raw.bin {
            let mut bin_map = HashMap::new();
            for bin in bins {
                let name = bin
                    .as_path()
                    .file_name()
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("invalid bin name: {}", bin.to_string_lossy()),
                        )
                    })?
                    .to_string_lossy()
                    .to_string();
                bin_map.insert(name, bin);
            }
        } else if let Some(Directories {
            bin: Some(bin_dir), ..
        }) = raw.directories
        {
            for entry in WalkDir::new(bin_dir) {
                let entry = entry?;
                let path = entry.path();
                if path.starts_with(".") {
                    continue;
                }
                if let Some(file_name) = path.file_name() {
                    bin_map.insert(file_name.to_string_lossy().to_string(), path.into());
                }
            }
        };
        Ok(Self {
            bin: bin_map,
            scripts: raw.scripts,
        })
    }
}
