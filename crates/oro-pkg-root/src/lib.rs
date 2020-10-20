use std::fs;
use std::path::{Path, PathBuf};

pub fn pkg_root(start_dir: impl AsRef<Path>) -> Option<PathBuf> {
    for path in start_dir.as_ref().ancestors() {
        let node_modules = path.join("node_modules");
        let pkg_json = path.join("package.json");
        if let Ok(meta) = fs::metadata(node_modules) {
            if meta.is_dir() {
                return Some(PathBuf::from(path));
            }
        }
        if let Ok(meta) = fs::metadata(pkg_json) {
            if meta.is_file() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}
