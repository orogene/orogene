use std::fs;
use std::path::Path;

use async_std::fs as afs;
use ssri::Integrity;

use crate::content::path;
use crate::errors::{Internal, Result};

pub fn rm(cache: &Path, sri: &Integrity) -> Result<()> {
    fs::remove_file(path::content_path(cache, sri)).to_internal()?;
    Ok(())
}

pub async fn rm_async(cache: &Path, sri: &Integrity) -> Result<()> {
    afs::remove_file(path::content_path(cache, sri))
        .await
        .to_internal()?;
    Ok(())
}
