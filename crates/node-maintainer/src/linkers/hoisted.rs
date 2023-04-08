use std::ffi::OsStr;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::{atomic, Arc};

use futures::{StreamExt, TryStreamExt};
use oro_common::BuildManifest;
use oro_script::OroScript;
use unicase::UniCase;
use walkdir::WalkDir;

use crate::error::NodeMaintainerError;
use crate::graph::Graph;
use crate::META_FILE_NAME;

use super::LinkerOptions;

pub(crate) struct HoistedLinker(pub(crate) LinkerOptions);

impl HoistedLinker {
    pub async fn prune(&self, graph: &Graph) -> Result<usize, NodeMaintainerError> {
        let prefix = self.0.root.join("node_modules");

        if !prefix.exists() {
            return Ok(0);
        }

        let start = std::time::Instant::now();

        if self.0.actual_tree.is_none() {
            // If there's no actual tree previously calculated, we can't trust
            // *anything* inside node_modules, so everything is immediately
            // extraneous and we wipe it all. Sorry.
            let mut entries = async_std::fs::read_dir(&prefix).await?;
            while let Some(entry) = entries.next().await {
                let entry = entry?;
                if entry.file_type().await?.is_dir() {
                    async_std::fs::remove_dir_all(entry.path()).await?;
                } else {
                    async_std::fs::remove_file(entry.path()).await?;
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
                        .0
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
                            .0
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
                if let Some(pb) = &self.0.on_prune_progress {
                    pb(entry_path);
                }
                tracing::trace!("Pruning extraneous directory: {}", entry.path().display());
                async_std::fs::remove_dir_all(entry.path()).await?;
            } else {
                if let Some(pb) = &self.0.on_prune_progress {
                    pb(entry_path);
                }
                tracing::trace!("Pruning extraneous file: {}", entry.path().display());
                async_std::fs::remove_file(entry.path()).await?;
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

        let root = &self.0.root;
        let stream = futures::stream::iter(graph.inner.node_indices());
        let concurrent_count = Arc::new(AtomicUsize::new(0));
        let actually_extracted = Arc::new(AtomicUsize::new(0));
        let total = graph.inner.node_count();
        let total_completed = Arc::new(AtomicUsize::new(0));
        let node_modules = root.join("node_modules");
        std::fs::create_dir_all(&node_modules)?;
        let prefer_copy = self.0.prefer_copy
            || match self.0.cache.as_deref() {
                Some(cache) => supports_reflink(cache, &node_modules),
                None => false,
            };
        let validate = self.0.validate;
        stream
            .map(|idx| Ok((idx, concurrent_count.clone(), total_completed.clone(), actually_extracted.clone())))
            .try_for_each_concurrent(
                self.0.concurrency,
                move |(child_idx, concurrent_count, total_completed, actually_extracted)| async move {
                    if child_idx == graph.root {
                        return Ok(());
                    }

                    concurrent_count.fetch_add(1, atomic::Ordering::SeqCst);
                    let subdir =
                        graph
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
                            .extract_to_dir(&target_dir, prefer_copy, validate)
                            .await?;
                        actually_extracted.fetch_add(1, atomic::Ordering::SeqCst);
                    }

                    if let Some(on_extract) = &self.0.on_extract_progress {
                        on_extract(&graph[child_idx].package);
                    }

                    tracing::trace!(
                        in_flight = concurrent_count.fetch_sub(1, atomic::Ordering::SeqCst) - 1,
                        "Extracted {} to {} in {:?}ms. {}/{total} done.",
                        graph[child_idx].package.name(),
                        target_dir.display(),
                        start.elapsed().as_millis(),
                        total_completed.fetch_add(1, atomic::Ordering::SeqCst) + 1,
                    );
                    Ok::<_, NodeMaintainerError>(())
                },
            )
            .await?;
        std::fs::write(
            node_modules.join(META_FILE_NAME),
            graph.to_kdl()?.to_string(),
        )?;
        let actually_extracted = actually_extracted.load(atomic::Ordering::SeqCst);
        tracing::debug!(
            "Extracted {actually_extracted} package{} in {}ms.",
            if actually_extracted == 1 { "" } else { "s" },
            start.elapsed().as_millis(),
        );
        Ok(actually_extracted)
    }

    pub async fn link_bins(&self, graph: &Graph) -> Result<usize, NodeMaintainerError> {
        tracing::debug!("Linking bins...");
        let start = std::time::Instant::now();
        let root = &self.0.root;
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
                async_std::fs::remove_dir_all(entry.path()).await?;
            }
        }
        futures::stream::iter(graph.inner.node_indices())
            .map(|idx| Ok((idx, linked.clone())))
            .try_for_each_concurrent(self.0.concurrency, move |(idx, linked)| async move {
                if idx == graph.root {
                    return Ok(());
                }

                let subdir =
                    graph
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
                    async_std::task::spawn_blocking(move || {
                        // We only create a symlink if the target bin exists.
                        if from.symlink_metadata().is_ok() {
                            std::fs::create_dir_all(target_dir)?;
                            // TODO: use a DashMap here to prevent race conditions, maybe?
                            if let Ok(meta) = to.symlink_metadata() {
                                if meta.is_dir() {
                                    std::fs::remove_dir_all(&to)?;
                                } else {
                                    std::fs::remove_file(&to)?;
                                }
                            }
                            link_bin(&from, &to)?;
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
        tracing::debug!(
            "Linked {linked} package bins in {}ms.",
            start.elapsed().as_millis()
        );
        Ok(linked)
    }

    pub async fn rebuild(&self, graph: &Graph, ignore_scripts: bool) -> Result<(), NodeMaintainerError> {
        tracing::debug!("Running lifecycle scripts...");
        let start = std::time::Instant::now();
        if !ignore_scripts {
            self.run_scripts(graph, "preinstall").await?;
        }
        self.link_bins(graph).await?;
        if !ignore_scripts {
            self.run_scripts(graph, "install").await?;
            self.run_scripts(graph, "postinstall").await?;
        }
        tracing::debug!(
            "Ran lifecycle scripts in {}ms.",
            start.elapsed().as_millis()
        );
        Ok(())
    }

    pub async fn run_scripts(&self, graph: &Graph, event: &str) -> Result<(), NodeMaintainerError> {
        tracing::debug!("Running {event} lifecycle scripts");
        let start = std::time::Instant::now();
        let root = &self.0.root;
        futures::stream::iter(graph.inner.node_indices())
            .map(Ok)
            .try_for_each_concurrent(self.0.script_concurrency, move |idx| async move {
                if idx == graph.root {
                    return Ok::<_, NodeMaintainerError>(());
                }

                let subdir =
                    graph
                    .node_path(idx)
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("/node_modules/");
                let package_dir = root.join("node_modules").join(subdir);

                let build_mani = BuildManifest::from_path(package_dir.join("package.json"))
                    .map_err(|e| {
                        NodeMaintainerError::BuildManifestReadError(
                            package_dir.join("package.json"),
                            e,
                        )
                    })?;

                let name = graph[idx].package.name().to_string();
                if build_mani.scripts.contains_key(event) {
                    let package_dir = package_dir.clone();
                    let root = root.clone();
                    let event = event.to_owned();
                    let span = tracing::info_span!("script::{name}::{event}");
                    let _span_enter = span.enter();
                    if let Some(on_script_start) = &self.0.on_script_start {
                        on_script_start(&graph[idx].package, &event);
                    }
                    std::mem::drop(_span_enter);
                    let mut script = async_std::task::spawn_blocking(move || {
                        OroScript::new(package_dir, event)?
                            .workspace_path(root)
                            .spawn()
                    })
                    .await?;
                    let stdout = script.stdout.take();
                    let stderr = script.stderr.take();
                    let stdout_name = name.clone();
                    let stderr_name = name.clone();
                    let stdout_on_line = self.0.on_script_line.clone();
                    let stderr_on_line = self.0.on_script_line.clone();
                    let stdout_span = span;
                    let stderr_span = stdout_span.clone();
                    futures::try_join!(
                        async_std::task::spawn_blocking(move || {
                            let _enter = stdout_span.enter();
                            if let Some(stdout) = stdout {
                                for line in BufReader::new(stdout).lines() {
                                    let line = line?;
                                    tracing::debug!("stdout::{stdout_name}: {}", line);
                                    if let Some(on_script_line) = &stdout_on_line {
                                        on_script_line(&line);
                                    }
                                }
                            }
                            Ok::<_, NodeMaintainerError>(())
                        }),
                        async_std::task::spawn_blocking(move || {
                            let _enter = stderr_span.enter();
                            if let Some(stderr) = stderr {
                                for line in BufReader::new(stderr).lines() {
                                    let line = line?;
                                    tracing::debug!("stderr::{stderr_name}: {}", line);
                                    if let Some(on_script_line) = &stderr_on_line {
                                        on_script_line(&line);
                                    }
                                }
                            }
                            Ok::<_, NodeMaintainerError>(())
                        }),
                        async_std::task::spawn_blocking(move || {
                            script.wait()?;
                            Ok::<_, NodeMaintainerError>(())
                        }),
                    )?;
                }

                Ok::<_, NodeMaintainerError>(())
            })
            .await?;
        tracing::debug!(
            "Ran lifecycle scripts for {event} in {}ms.",
            start.elapsed().as_millis()
        );
        Ok(())
    }
}

fn supports_reflink(src_dir: &Path, dest_dir: &Path) -> bool {
    let temp = match tempfile::NamedTempFile::new_in(src_dir) {
        Ok(t) => t,
        Err(e) => {
            tracing::debug!("error creating tempfile while checking for reflink support: {e}.");
            return false;
        }
    };
    match std::fs::write(&temp, "a") {
        Ok(_) => {}
        Err(e) => {
            tracing::debug!("error writing to tempfile while checking for reflink support: {e}.");
            return false;
        }
    };
    let tempdir = match tempfile::TempDir::new_in(dest_dir) {
        Ok(t) => t,
        Err(e) => {
            tracing::debug!(
                "error creating destination tempdir while checking for reflink support: {e}."
            );
            return false;
        }
    };
    let supports_reflink = reflink::reflink(temp.path(), tempdir.path().join("b"))
        .map(|_| true)
        .map_err(|e| {
            tracing::debug!(
                "reflink support check failed. Files will be hard linked or copied. ({e})"
            );
            e
        })
        .unwrap_or(false);

    if supports_reflink {
        tracing::debug!("Verified reflink support. Extracted data will use copy-on-write reflinks instead of hard links or full copies.")
    }

    supports_reflink
}

fn link_bin(from: &Path, to: &Path) -> Result<(), NodeMaintainerError> {
    #[cfg(windows)]
    oro_shim_bin::shim_bin(from, to)?;
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(from, to)?;
    }
    Ok(())
}
