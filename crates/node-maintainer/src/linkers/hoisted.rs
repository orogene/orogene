use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::{atomic, Arc};

use dashmap::DashSet;
use futures::lock::Mutex;
use futures::{StreamExt, TryStreamExt};
use nassun::ExtractMode;
use oro_common::BuildManifest;
use petgraph::stable_graph::NodeIndex;
use unicase::UniCase;
use walkdir::WalkDir;

use crate::error::{IoContext, NodeMaintainerError};
use crate::graph::Graph;
use crate::{META_FILE_NAME, STORE_DIR_NAME};

use super::LinkerOptions;

pub(crate) struct HoistedLinker {
    pub(crate) pending_rebuild: Arc<Mutex<HashSet<NodeIndex>>>,
    pub(crate) mkdir_cache: Arc<DashSet<PathBuf>>,
    pub(crate) opts: LinkerOptions,
}

impl HoistedLinker {
    pub fn new(opts: LinkerOptions) -> Self {
        Self {
            pending_rebuild: Arc::new(Mutex::new(HashSet::new())),
            mkdir_cache: Arc::new(DashSet::new()),
            opts,
        }
    }

    pub async fn prune(&self, graph: &Graph) -> Result<usize, NodeMaintainerError> {
        let start = std::time::Instant::now();

        let prefix = self.opts.root.join("node_modules");

        if !prefix.exists() {
            tracing::debug!(
                "Nothing to prune. Completed check in {}ms.",
                start.elapsed().as_micros() / 1000
            );
            return Ok(0);
        }

        if self.opts.actual_tree.is_none()
            || async_std::path::Path::new(&prefix.join(STORE_DIR_NAME))
                .exists()
                .await
        {
            // If there's no actual tree previously calculated, we can't trust
            // *anything* inside node_modules, so everything is immediately
            // extraneous and we wipe it all. Sorry.
            let mut entries = async_std::fs::read_dir(&prefix).await.io_context(|| {
                format!(
                    "Failed to read contents of node_modules at {}",
                    prefix.display()
                )
            })?;
            while let Some(entry) = entries.next().await {
                let entry = entry.io_context(|| {
                    format!(
                        "Failed to read directory entry from prefix at {}",
                        prefix.display()
                    )
                })?;
                let ty = entry.file_type().await.io_context(|| {
                    format!(
                        "Failed to get file type from entry at {}.",
                        entry.path().display()
                    )
                })?;
                if ty.is_dir() {
                    async_std::fs::remove_dir_all(entry.path()).await.io_context(|| format!("Failed to rimraf contents of directory at {} while pruning node_modules.", entry.path().display()))?;
                } else if ty.is_file() {
                    async_std::fs::remove_file(entry.path())
                        .await
                        .io_context(|| {
                            format!(
                                "Failed to delete file at {} while pruning node_modules.",
                                entry.path().display()
                            )
                        })?;
                } else if ty.is_symlink() && async_std::fs::remove_file(entry.path()).await.is_err()
                {
                    async_std::fs::remove_dir_all(entry.path())
                        .await
                        .io_context(|| {
                            format!(
                                "Failed to delete {} while pruning node_modules.",
                                entry.path().display()
                            )
                        })?;
                }
            }

            tracing::debug!("No metadata file found in node_modules/. Pruned entire node_modules/ directory in {}ms.", start.elapsed().as_micros() / 1000);

            // TODO: get an accurate count here?
            return Ok(0);
        }

        let nm_osstr = Some(std::ffi::OsStr::new("node_modules"));
        let bin_osstr = Some(std::ffi::OsStr::new(".bin"));
        let meta = prefix.join(META_FILE_NAME);
        let mut extraneous_packages = 0;
        let extraneous = &mut extraneous_packages;

        for entry in WalkDir::new(&prefix)
            .into_iter()
            .filter_entry(move |entry| {
                let entry_path = entry.path();

                if entry_path == meta {
                    // Skip the meta file
                    return false;
                }

                let file_name = entry_path.file_name();

                if file_name == nm_osstr {
                    // We don't want to skip node_modules themselves
                    return true;
                }

                if file_name == bin_osstr {
                    return false;
                }

                if file_name
                    .expect("this should have a file name")
                    .to_string_lossy()
                    .starts_with('@')
                {
                    // Let scoped packages through.
                    return true;
                }

                // See if we're looking at a package dir, presumably (or a straggler file).
                if entry_path
                    .parent()
                    .expect("this must have a parent")
                    .file_name()
                    == nm_osstr
                {
                    let entry_subpath_path = entry_path
                        .strip_prefix(&prefix)
                        .expect("this should definitely be under the prefix");
                    let entry_subpath =
                        UniCase::from(entry_subpath_path.to_string_lossy().replace('\\', "/"));

                    let actual = self
                        .opts
                        .actual_tree
                        .as_ref()
                        .and_then(|tree| tree.packages.get(&entry_subpath));
                    let ideal = graph
                        .node_at_path(entry_subpath_path)
                        .and_then(|node| graph.node_lockfile_node(node.idx, false).ok());
                    // If the package is not in the actual tree, or it doesn't
                    // match up with what the ideal tree wants, it's
                    // extraneous. We want to return true for those so we
                    // delete them later.
                    if ideal.is_some()
                        && self
                            .opts
                            .actual_tree
                            .as_ref()
                            .map(|tree| tree.packages.contains_key(&entry_subpath))
                            .unwrap_or(false)
                        && actual == ideal.as_ref()
                    {
                        return false;
                    } else {
                        *extraneous += 1;
                        return true;
                    }
                }

                // We're not interested in any other files than the package dirs themselves.
                false
            })
        {
            let entry = entry?;
            let entry_path = entry.path();
            let file_name = entry_path.file_name();
            if file_name == nm_osstr
                || file_name == bin_osstr
                || file_name
                    .map(|s| s.to_string_lossy().starts_with('@'))
                    .unwrap_or(false)
            {
                continue;
            } else if entry.file_type().is_dir() {
                if let Some(pb) = &self.opts.on_prune_progress {
                    pb(entry_path);
                }
                tracing::trace!("Pruning extraneous directory: {}", entry.path().display());
                async_std::fs::remove_dir_all(entry.path())
                    .await
                    .io_context(|| {
                        format!(
                            "Failed to prune extraneous directory at {}",
                            entry.path().display()
                        )
                    })?;
            } else {
                if let Some(pb) = &self.opts.on_prune_progress {
                    pb(entry_path);
                }
                tracing::trace!("Pruning extraneous file: {}", entry.path().display());
                async_std::fs::remove_file(entry.path())
                    .await
                    .io_context(|| {
                        format!(
                            "Failed to prune extraneous file at {}",
                            entry.path().display()
                        )
                    })?;
            }
        }

        if extraneous_packages == 0 {
            tracing::debug!(
                "Nothing to prune. Completed check in {}ms.",
                start.elapsed().as_micros() / 1000
            );
        } else {
            tracing::debug!(
                "Pruned {extraneous_packages} extraneous package{} in {}ms.",
                start.elapsed().as_micros() / 1000,
                if extraneous_packages == 1 { "" } else { "s" },
            );
        }
        Ok(extraneous_packages)
    }

    pub async fn extract(&self, graph: &Graph) -> Result<usize, NodeMaintainerError> {
        tracing::debug!("Extracting node_modules/...");
        let start = std::time::Instant::now();

        let root = &self.opts.root;
        let stream = futures::stream::iter(graph.inner.node_indices());
        let concurrent_count = Arc::new(AtomicUsize::new(0));
        let actually_extracted = Arc::new(AtomicUsize::new(0));
        let pending_rebuild = self.pending_rebuild.clone();
        let total = graph.inner.node_count();
        let total_completed = Arc::new(AtomicUsize::new(0));
        let node_modules = root.join("node_modules");
        super::mkdirp(&node_modules, &self.mkdir_cache)?;
        let extract_mode = if let Some(cache) = self.opts.cache.as_deref() {
            if super::supports_reflink(cache, &node_modules) {
                ExtractMode::Reflink
            } else if self.opts.prefer_copy {
                ExtractMode::Copy
            } else if super::supports_hardlink(cache, &node_modules) {
                ExtractMode::Hardlink
            } else {
                ExtractMode::Copy
            }
        } else {
            ExtractMode::AutoHardlink
        };
        stream
            .map(|idx| {
                Ok((
                    idx,
                    concurrent_count.clone(),
                    total_completed.clone(),
                    actually_extracted.clone(),
                    pending_rebuild.clone(),
                ))
            })
            .try_for_each_concurrent(
                self.opts.concurrency,
                move |(
                    child_idx,
                    concurrent_count,
                    total_completed,
                    actually_extracted,
                    pending_rebuild,
                )| async move {
                    if child_idx == graph.root {
                        return Ok(());
                    }

                    concurrent_count.fetch_add(1, atomic::Ordering::SeqCst);
                    let subdir = graph
                        .node_path(child_idx)
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join("/node_modules/");
                    let target_dir = root.join("node_modules").join(&subdir);

                    let start = std::time::Instant::now();

                    if !target_dir.exists() {
                        graph[child_idx]
                            .package
                            .extract_to_dir(&target_dir, extract_mode)
                            .await?;
                        actually_extracted.fetch_add(1, atomic::Ordering::SeqCst);
                        let target_dir = target_dir.clone();
                        let build_mani = async_std::task::spawn_blocking(move || {
                            BuildManifest::from_path(target_dir.join("package.json")).map_err(|e| {
                                NodeMaintainerError::BuildManifestReadError(
                                    target_dir.join("package.json"),
                                    e,
                                )
                            })
                        })
                        .await?;
                        if build_mani.scripts.contains_key("preinstall")
                            || build_mani.scripts.contains_key("install")
                            || build_mani.scripts.contains_key("postinstall")
                            || build_mani.scripts.contains_key("prepare")
                            || !build_mani.bin.is_empty()
                        {
                            pending_rebuild.lock().await.insert(child_idx);
                        }
                    }

                    let elapsed = start.elapsed();

                    if let Some(on_extract) = &self.opts.on_extract_progress {
                        on_extract(&graph[child_idx].package, elapsed);
                    }

                    tracing::trace!(
                        in_flight = concurrent_count.fetch_sub(1, atomic::Ordering::SeqCst) - 1,
                        "Extracted {} to {} in {:?}ms. {}/{total} done.",
                        graph[child_idx].package.name(),
                        target_dir.display(),
                        elapsed.as_micros() / 1000,
                        total_completed.fetch_add(1, atomic::Ordering::SeqCst) + 1,
                    );
                    Ok::<_, NodeMaintainerError>(())
                },
            )
            .await?;
        let meta = node_modules.join(META_FILE_NAME);
        std::fs::write(&meta, graph.to_kdl()?.to_string())
            .io_context(|| format!("Failed to write Orogene meta file to {}.", meta.display()))?;
        let extracted_count = actually_extracted.load(atomic::Ordering::SeqCst);

        tracing::debug!(
            "Extracted {extracted_count} package{} in {}ms.",
            if extracted_count == 1 { "" } else { "s" },
            start.elapsed().as_millis(),
        );
        Ok(extracted_count)
    }

    pub async fn link_bins(&self, graph: &Graph) -> Result<usize, NodeMaintainerError> {
        let root = &self.opts.root;
        let linked = Arc::new(AtomicUsize::new(0));
        let bin_file_name = Some(OsStr::new(".bin"));
        let nm_file_name = Some(OsStr::new("node_modules"));
        for entry in WalkDir::new(root.join("node_modules"))
            .into_iter()
            .filter_entry(|e| {
                let path = e.path().file_name();
                path == bin_file_name || path == nm_file_name
            })
        {
            let entry = entry?;
            if entry.path().file_name() == bin_file_name {
                async_std::fs::remove_dir_all(entry.path()).await.io_context(|| format!("Failed to remove directory at {} while clearing out existing node_modules/.bin directories.", entry.path().display()))?;
            }
        }
        futures::stream::iter(self.pending_rebuild.lock().await.iter().copied())
            .map(|idx| Ok((idx, linked.clone())))
            .try_for_each_concurrent(self.opts.concurrency, move |(idx, linked)| async move {
                if idx == graph.root {
                    return Ok(());
                }

                let subdir = graph
                    .node_path(idx)
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("/node_modules/");
                let package_dir = root.join("node_modules").join(subdir);
                let parent = package_dir.parent().expect("must have parent");
                let target_dir = if parent.file_name() == Some(OsStr::new("node_modules")) {
                    parent.join(".bin")
                } else {
                    // Scoped
                    parent.parent().expect("must have parent").join(".bin")
                };

                let build_mani = BuildManifest::from_path(package_dir.join("package.json"))
                    .map_err(|e| {
                        NodeMaintainerError::BuildManifestReadError(
                            package_dir.join("package.json"),
                            e,
                        )
                    })?;

                for (name, path) in &build_mani.bin {
                    let target_dir = target_dir.clone();
                    let to = target_dir.join(name);
                    let from = package_dir.join(path);
                    let name = name.clone();
                    let mkdir_cache = self.mkdir_cache.clone();
                    async_std::task::spawn_blocking(move || {
                        // We only create a symlink if the target bin exists.
                        let target_dir = &target_dir;
                        if from.symlink_metadata().is_ok() {
                            super::mkdirp(target_dir, &mkdir_cache)?;
                            // TODO: use a DashMap here to prevent race conditions, maybe?
                            if let Ok(meta) = to.symlink_metadata() {
                                if meta.is_dir() {
                                    std::fs::remove_dir_all(&to).io_context(|| {
                                        format!(
                                            "Failed to remove existing bin dir at {} while linking {} bin.",
                                            to.display(),
                                            name,
                                        )
                                    })?;
                                } else {
                                    std::fs::remove_file(&to).io_context(|| {
                                        format!(
                                            "Failed to remove existing bin file at {} while linking {} bin.",
                                            to.display(),
                                            name,
                                        )
                                    })?;
                                }
                            }
                            super::link_bin(&from, &to)?;
                            tracing::trace!(
                                "Linked bin for {} from {} to {}",
                                name,
                                from.display(),
                                to.display()
                            );
                        }
                        Ok::<_, NodeMaintainerError>(())
                    })
                    .await?;
                    linked.fetch_add(1, atomic::Ordering::SeqCst);
                }
                Ok::<_, NodeMaintainerError>(())
            })
            .await?;
        let linked = linked.load(atomic::Ordering::SeqCst);
        Ok(linked)
    }

    pub fn package_dir(&self, graph: &Graph, idx: NodeIndex) -> (PathBuf, PathBuf) {
        let subdir = graph
            .node_path(idx)
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join("/node_modules/");
        (
            self.opts.root.join("node_modules").join(subdir),
            self.opts.root.clone(),
        )
    }
}
