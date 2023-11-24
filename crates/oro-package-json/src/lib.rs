pub mod error;
mod normalize;
use error::OroPackageJsonError;
use normalize::NormalizeOptions;
use oro_common::Manifest;
use std::path::PathBuf;

pub(crate) const NORMALIZE_STEPS: normalize::NormalizeSteps = normalize::NormalizeSteps {
    optional_dedupe: true,
    attributes: true,
    serverjs: true,
    git_head: true,
    gypfile: true,
    funding: true,
    authors: true,
    readme: true,
    mans: true,
};

pub async fn normalize(
    manifest: &mut Manifest,
    root: PathBuf,
    strict: bool,
) -> Result<Vec<String>, OroPackageJsonError> {
    normalize::normalize(
        manifest,
        NormalizeOptions {
            steps: NORMALIZE_STEPS,
            root,
            strict,
        },
    )
    .await
}
