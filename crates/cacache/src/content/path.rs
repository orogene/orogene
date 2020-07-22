use ssri::Integrity;
use std::path::{Path, PathBuf};

const CONTENT_VERSION: &str = "2";

// Current format of content file path:
//
// sha512-BaSE64Hex= ->
// ~/.my-cache/content-v2/sha512/ba/da/55deadbeefc0ffee
//
pub fn content_path(cache: &Path, sri: &Integrity) -> PathBuf {
    let mut path = PathBuf::new();
    let (algo, hex) = sri.to_hex();
    path.push(cache);
    path.push(format!("content-v{}", CONTENT_VERSION));
    path.push(algo.to_string());
    path.push(&hex[0..2]);
    path.push(&hex[2..4]);
    path.push(&hex[4..]);
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use ssri::Integrity;
    use std::path::Path;

    #[test]
    fn basic_test() {
        let sri = Integrity::from(b"hello world");
        let cpath = content_path(Path::new("~/.my-cache"), &sri);
        let mut wanted = PathBuf::new();
        wanted.push("~/.my-cache");
        wanted.push(format!("content-v{}", CONTENT_VERSION));
        wanted.push("sha256");
        wanted.push("b9");
        wanted.push("4d");
        wanted.push("27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
        assert_eq!(cpath.to_str().unwrap(), wanted.to_str().unwrap());
    }
}
