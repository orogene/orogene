//! Functions for iterating over the cache.
use std::path::Path;

use crate::errors::Result;
use crate::index;

/// Returns a synchronous iterator that lists all cache index entries.
pub fn list_sync<P: AsRef<Path>>(cache: P) -> impl Iterator<Item = Result<index::Metadata>> {
    index::ls(cache.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;

    #[test]
    fn test_list_sync() {
        // check that the public interface to list elements can actually use the
        // Iterator::Item
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_owned();

        assert!(list_sync(dir)
            .map(|x| Ok(x?.key))
            .collect::<Result<Vec<_>>>()
            .is_err())
    }
}
