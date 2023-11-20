use crate::error::OroPackageJsonError;
use async_std::path::PathBuf;
use async_std::stream::StreamExt;
use oro_common::{Funding, Man, Manifest, PersonField};
use walkdir::WalkDir;

#[derive(Debug, Clone, Default)]
pub struct NormalizeSteps {
    pub optional_dedupe: bool,
    pub attributes: bool,
    pub serverjs: bool,
    pub git_head: bool,
    pub gypfile: bool,
    pub funding: bool,
    pub authors: bool,
    pub readme: bool,
    pub mans: bool,
}

#[derive(Debug, Clone)]
pub struct NormalizeOptions {
    pub steps: NormalizeSteps,
    pub root: std::path::PathBuf,
    pub strict: bool,
}

pub async fn normalize(
    manifest: &mut Manifest,
    options: NormalizeOptions,
) -> Result<Vec<String>, OroPackageJsonError> {
    let mut changes = Vec::new();
    let top_level_files = async_std::fs::read_dir(&options.root)
        .await?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<PathBuf>, std::io::Error>>()
        .await?
        .into_iter();

    if options.steps.attributes {
        manifest._rest.retain(|key, _| {
            if key.starts_with('_') {
                changes.push(format!(r#""{key}" was removed"#));
                return false;
            }
            true
        });
    }
    if options.steps.serverjs
        && manifest.scripts.get("start").is_none()
        && options.root.join("server.js").exists()
    {
        manifest
            .scripts
            .insert("start".to_owned(), "node server.js".to_owned());
        changes.push(r#""scripts.start" was set to "node server.js""#.to_owned());
    }
    if options.steps.optional_dedupe {
        for key in manifest.optional_dependencies.keys() {
            let _ = manifest.dependencies.remove(key);
        }
    }
    if options.steps.funding {
        if let Some(Funding::Str(ref funding_url)) = manifest.funding {
            manifest.funding = Some(Funding::Obj {
                url: Some(funding_url.to_owned()),
            });
        }
    }
    if options.steps.authors && manifest.contributors.is_empty() {
        let authors = async_std::fs::read_to_string(options.root.join("AUTHORS"))
            .await
            .unwrap_or("".to_owned())
            .lines()
            .filter(|v| !v.starts_with('#'))
            .map(|v| PersonField::Str(v.to_owned()))
            .collect::<Vec<_>>();
        manifest.contributors = authors;
        changes.push(
            r#""contributors" was auto-populated with the contents of the "AUTHORS" file"#
                .to_owned(),
        );
    }
    if options.steps.mans && manifest.man.is_none() {
        if let Some(man_directory) = manifest
            .directories
            .clone()
            .and_then(|directories| directories.man)
        {
            let cloned_man_directory = man_directory.clone();
            let cloned_root = options.root.clone();
            let entries = async_std::task::spawn_blocking(move || {
                WalkDir::new(cloned_root.join(man_directory))
                    .into_iter()
                    .filter_entry(|entry| {
                        if let Some(extension) = entry.path().extension() {
                            return extension
                                .to_string_lossy()
                                .into_owned()
                                .chars()
                                .all(|cha| cha.is_numeric());
                        }
                        false
                    })
            })
            .await;
            manifest.man = Some(Man::Vec(
                entries
                    .map(|entry| {
                        entry.map(|entry| {
                            pathdiff::diff_paths(entry.into_path(), &cloned_man_directory)
                                .expect("TODO")
                        })
                    })
                    .map(|entry| entry.map(|entry| entry.display().to_string()))
                    .collect::<Result<Vec<String>, walkdir::Error>>()?,
            ));
        }
    }
    if options.steps.readme && manifest.readme.is_none() {
        let readmefiles = top_level_files.clone().filter(|v| {
            v.file_stem() == Some("readme".as_ref())
                || v.file_stem() == Some("README".as_ref())
                || v.extension() == Some("markdown".as_ref())
        });
        if let Some(readme) = readmefiles.last() {
            let readme_data = async_std::fs::read_to_string(options.root.join(&readme)).await?;
            manifest.readme = Some(readme_data);
            manifest.readme_filename = pathdiff::diff_paths(&readme, &options.root);
        } else {
            manifest.readme = Some("ERROR: No README data found!".to_owned())
        }
    }
    if options.steps.gypfile
        && manifest.scripts.get("install").is_none()
        && manifest.scripts.get("preinstall").is_none()
        && manifest.gypfile.is_none()
    {
        let gypfiles = top_level_files
            .clone()
            .filter(|v| v.extension() == Some("gyp".as_ref()));
        if gypfiles.count() != 0 {
            manifest
                .scripts
                .insert("install".to_owned(), "node-gyp rebuild".to_owned());
            manifest.gypfile = Some(true);
            changes.push(r#""scripts.install" was set to "node-gyp rebuild""#.to_owned());
            changes.push(r#""gypfile" was set to "true""#.to_owned());
        }
    }
    if options.steps.git_head && manifest.git_head.is_none() {
        manifest.git_head = async_std::task::spawn_blocking(move || {
            let repository = git2::Repository::open(options.root).ok()?;
            let object = repository.revparse_single("HEAD").ok()?;
            Some(object.as_commit()?.id().to_string())
        })
        .await;
    }
    Ok(changes)
}
