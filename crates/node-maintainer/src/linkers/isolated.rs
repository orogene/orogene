use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, BufReader},
    path::Path,
    sync::{
        atomic::{self, AtomicUsize},
        Arc,
    },
};

use futures::{StreamExt, TryStreamExt};
use oro_common::BuildManifest;
use oro_script::OroScript;
use petgraph::{stable_graph::NodeIndex, visit::EdgeRef, Direction};
use ssri::Integrity;

use crate::{graph::Graph, NodeMaintainerError, META_FILE_NAME, STORE_DIR_NAME};

use super::LinkerOptions;

pub(crate) struct IsolatedLinker(pub(crate) LinkerOptions);

impl IsolatedLinker {
    pub async fn prune(&self, graph: &Graph) -> Result<usize, NodeMaintainerError> {
        let start = std::time::Instant::now();

        let prefix = self.0.root.join("node_modules");

        if !prefix.exists() {
            tracing::debug!(
                "Nothing to prune. Completed check in {}ms.",
                start.elapsed().as_micros() / 1000
            );
            return Ok(0);
        }

        let store = prefix.join(STORE_DIR_NAME);

        if self.0.actual_tree.is_none() || !async_std::path::Path::new(&store).exists().await {
            // If there's no actual tree previously calculated, we can't trust
            // *anything* inside node_modules, so everything is immediately
            // extraneous and we wipe it all. Sorry.
            let mut entries = async_std::fs::read_dir(&prefix).await?;
            while let Some(entry) = entries.next().await {
                let entry = entry?;
                let path = entry.path();
                let ty = entry.file_type().await?;
                if ty.is_dir() {
                    async_std::fs::remove_dir_all(&path).await?;
                } else if ty.is_file() {
                    async_std::fs::remove_file(&path).await?;
                } else if ty.is_symlink() && async_std::fs::remove_file(entry.path()).await.is_err()
                {
                    async_std::fs::remove_dir_all(&path).await?;
                }
            }

            tracing::debug!("No metadata file found in node_modules/. Pruned entire node_modules/ directory in {}ms.", start.elapsed().as_micros() / 1000);

            // TODO: get an accurate count here?
            return Ok(0);
        }

        let mut expected = HashSet::new();

        let expected_mut = &mut expected;
        let store_ref = &store;
        // Clean out individual node_modules within
        let indices = graph.inner.node_indices().map(move |idx| {
            if idx != graph.root {
                let pkg_store_dir = store_ref.join(package_dir_name(graph, idx));

                expected_mut.insert(pkg_store_dir);
            }
            idx
        });

        let prefix_ref = &prefix;
        futures::stream::iter(indices)
            .map(Ok)
            .try_for_each_concurrent(self.0.concurrency, move |idx| async move {
                let pkg = &graph[idx].package;

                let pkg_nm = if idx == graph.root {
                    prefix_ref.to_owned()
                } else {
                    store_ref
                        .join(package_dir_name(graph, idx))
                        .join("node_modules")
                        .join(pkg.name())
                        .join("node_modules")
                };

                let mut expected_deps = HashMap::new();

                for edge in graph.inner.edges_directed(idx, Direction::Outgoing) {
                    let dep_pkg = &graph[edge.target()].package;
                    let dep_store_dir = async_std::path::PathBuf::from(
                        store_ref
                            .join(package_dir_name(graph, edge.target()))
                            .join("node_modules")
                            .join(dep_pkg.name()),
                    );
                    let dep_nm_entry = async_std::path::PathBuf::from(pkg_nm.join(dep_pkg.name()));
                    expected_deps.insert(dep_nm_entry, dep_store_dir);
                }

                if async_std::path::Path::new(&pkg_nm).exists().await {
                    let expected_ref = Arc::new(expected_deps);

                    async_std::fs::read_dir(&pkg_nm)
                        .await?
                        .map(|e| Ok((e, expected_ref.clone())))
                        .try_for_each(move |(entry, expected)| async move {
                            let entry = entry?;
                            let path = entry.path();
                            if let Some(target) = expected.get(&path) {
                                let target = target.clone();
                                let ty = entry.file_type().await?;
                                if ty.is_file() {
                                    async_std::fs::remove_file(&path).await?;
                                } else if ty.is_dir() {
                                    async_std::fs::remove_dir_all(&path).await?;
                                } else if ty.is_symlink() && target != path.read_link().await? {
                                    if async_std::fs::remove_file(&path).await.is_err() {
                                        async_std::fs::remove_dir_all(&path).await?;
                                    }
                                } else if ty.is_dir() {
                                    async_std::fs::remove_dir_all(&path).await?;
                                } else {
                                    #[cfg(windows)]
                                    let path_clone = path.clone();
                                    #[cfg(windows)]
                                    if async_std::task::spawn_blocking(move || {
                                        Ok::<_, std::io::Error>(
                                            !junction::exists(&path_clone)?
                                                || async_std::path::PathBuf::from(
                                                    &junction::get_target(&path_clone)?,
                                                ) != target,
                                        )
                                    })
                                    .await?
                                        && async_std::fs::remove_file(&path).await.is_err()
                                    {
                                        async_std::fs::remove_dir_all(&path).await?;
                                    }
                                }
                            }
                            Ok::<_, NodeMaintainerError>(())
                        })
                        .await?;
                }

                Ok::<_, NodeMaintainerError>(())
            })
            .await?;

        let expected_ref = &expected;

        let pruned = Arc::new(AtomicUsize::new(0));

        // Clean out any extraneous things in the store dir itself. We've
        // already verified the store dir at least exists.
        async_std::fs::read_dir(&store)
            .await?
            .map(|entry| Ok((entry, pruned.clone())))
            .try_for_each_concurrent(self.0.concurrency, move |(entry, pruned)| async move {
                let entry = entry?;
                let _path = entry.path();
                let path: &Path = _path.as_ref();
                if !expected_ref.contains(path) {
                    let ty = entry.file_type().await?;
                    if ty.is_dir() {
                        if path
                            .file_name()
                            .expect("must have filename")
                            .to_string_lossy()
                            .starts_with('@')
                        {
                            let mut iter = async_std::fs::read_dir(path).await?;
                            while let Some(next) = iter.next().await {
                                let next = next?;
                                if !expected_ref.contains::<std::path::PathBuf>(&next.path().into())
                                {
                                    let ty = next.file_type().await?;
                                    if ty.is_file() {
                                        async_std::fs::remove_file(next.path()).await?;
                                    } else if ty.is_dir() {
                                        async_std::fs::remove_dir_all(next.path()).await?;
                                    } else if ty.is_symlink()
                                        && async_std::fs::remove_file(next.path()).await.is_err()
                                    {
                                        async_std::fs::remove_dir_all(next.path()).await?;
                                    }
                                    pruned.fetch_add(1, atomic::Ordering::SeqCst);
                                }
                            }
                        } else {
                            async_std::fs::remove_dir_all(entry.path()).await?;
                            pruned.fetch_add(1, atomic::Ordering::SeqCst);
                        }
                    } else if ty.is_file() {
                        async_std::fs::remove_file(entry.path()).await?;
                        pruned.fetch_add(1, atomic::Ordering::SeqCst);
                    } else if ty.is_symlink()
                        && async_std::fs::remove_file(entry.path()).await.is_err()
                    {
                        async_std::fs::remove_dir_all(entry.path()).await?;
                        pruned.fetch_add(1, atomic::Ordering::SeqCst);
                    }
                }
                Ok::<_, NodeMaintainerError>(())
            })
            .await?;

        let pruned = pruned.load(atomic::Ordering::SeqCst);
        if pruned == 0 {
            tracing::debug!(
                "Nothing to prune. Completed check in {}ms.",
                start.elapsed().as_micros() / 1000
            );
        } else {
            tracing::debug!(
                "Pruned {pruned} extraneous package{} in {}ms.",
                start.elapsed().as_micros() / 1000,
                if pruned == 1 { "" } else { "s" },
            );
        }
        Ok(pruned)
    }

    pub async fn extract(&self, graph: &Graph) -> Result<usize, NodeMaintainerError> {
        tracing::debug!("Applying node_modules/...");
        let start = std::time::Instant::now();

        let root = &self.0.root;
        let store = root.join("node_modules").join(STORE_DIR_NAME);
        let store_ref = &store;
        let stream = futures::stream::iter(graph.inner.node_indices());
        let concurrent_count = Arc::new(AtomicUsize::new(0));
        let actually_extracted = Arc::new(AtomicUsize::new(0));
        let total = graph.inner.node_count();
        let total_completed = Arc::new(AtomicUsize::new(0));
        let node_modules = root.join("node_modules");
        std::fs::create_dir_all(&node_modules)?;
        let prefer_copy = self.0.prefer_copy
            || match self.0.cache.as_deref() {
                Some(cache) => super::supports_reflink(cache, &node_modules),
                None => false,
            };
        let validate = self.0.validate;
        stream
            .map(|idx| Ok((idx, concurrent_count.clone(), total_completed.clone(), actually_extracted.clone())))
            .try_for_each_concurrent(
                self.0.concurrency,
                move |(child_idx, concurrent_count, total_completed, actually_extracted)| async move {
                    if child_idx == graph.root {
                        link_deps(graph, child_idx, store_ref, &root.join("node_modules")).await?;
                        return Ok(());
                    }

                    concurrent_count.fetch_add(1, atomic::Ordering::SeqCst);

                    let pkg = &graph[child_idx].package;

                    // Actual package contents are extracted to
                    // `node_modules/.oro-store/<package-name>-<hash>/node_modules/<package-name>`
                    let target_dir = store_ref.join(package_dir_name(graph, child_idx)).join("node_modules").join(pkg.name());

                    let start = std::time::Instant::now();

                    if !target_dir.exists() {
                        graph[child_idx]
                            .package
                            .extract_to_dir(&target_dir, prefer_copy, validate)
                            .await?;
                        actually_extracted.fetch_add(1, atomic::Ordering::SeqCst);
                    }

                    link_deps(graph, child_idx, store_ref, &target_dir.join("node_modules")).await?;

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

    async fn link_bins(&self, graph: &Graph) -> Result<usize, NodeMaintainerError> {
        tracing::debug!("Linking bins...");
        let start = std::time::Instant::now();
        let root = &self.0.root;
        let store = root.join("node_modules").join(STORE_DIR_NAME);
        let store_ref = &store;
        let linked = Arc::new(AtomicUsize::new(0));

        futures::stream::iter(graph.inner.node_indices())
            .map(|idx| Ok((idx, linked.clone())))
            .try_for_each_concurrent(self.0.concurrency, move |(idx, linked)| async move {
                if idx == graph.root {
                    let added = link_dep_bins(
                        graph,
                        idx,
                        store_ref,
                        &root.join("node_modules").join(".bin"),
                    )
                    .await?;
                    linked.fetch_add(added, atomic::Ordering::SeqCst);
                    return Ok(());
                }

                let pkg = &graph[idx].package;
                let pkg_bin_dir = store_ref
                    .join(package_dir_name(graph, idx))
                    .join("node_modules")
                    .join(pkg.name())
                    .join("node_modules")
                    .join(".bin");

                let added = link_dep_bins(graph, idx, store_ref, &pkg_bin_dir).await?;
                linked.fetch_add(added, atomic::Ordering::SeqCst);

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

    pub async fn rebuild(
        &self,
        graph: &Graph,
        ignore_scripts: bool,
    ) -> Result<(), NodeMaintainerError> {
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

    async fn run_scripts(&self, graph: &Graph, event: &str) -> Result<(), NodeMaintainerError> {
        tracing::debug!("Running {event} lifecycle scripts");
        let start = std::time::Instant::now();
        let root = &self.0.root;
        let store = root.join("node_modules").join(STORE_DIR_NAME);
        let store_ref = &store;
        futures::stream::iter(graph.inner.node_indices())
            .map(Ok)
            .try_for_each_concurrent(self.0.script_concurrency, move |idx| async move {
                let pkg_dir = if idx == graph.root {
                    root.clone()
                } else {
                    let pkg = &graph[idx].package;
                    store_ref
                        .join(package_dir_name(graph, idx))
                        .join("node_modules")
                        .join(pkg.name())
                };

                let is_optional = graph.is_optional(idx);

                let build_mani =
                    BuildManifest::from_path(pkg_dir.join("package.json")).map_err(|e| {
                        NodeMaintainerError::BuildManifestReadError(pkg_dir.join("package.json"), e)
                    })?;

                let name = graph[idx].package.name().to_string();
                if build_mani.scripts.contains_key(event) {
                    let package_dir = pkg_dir.clone();
                    let package_dir_clone = package_dir.clone();
                    let event = event.to_owned();
                    let span = tracing::info_span!("script::{name}::{event}");
                    let _span_enter = span.enter();
                    if let Some(on_script_start) = &self.0.on_script_start {
                        on_script_start(&graph[idx].package, &event);
                    }
                    std::mem::drop(_span_enter);
                    let mut script = match async_std::task::spawn_blocking(move || {
                        OroScript::new(package_dir, event)?
                            .workspace_path(package_dir_clone)
                            .spawn()
                    })
                    .await
                    {
                        Ok(script) => script,
                        Err(e) if is_optional => {
                            let e: NodeMaintainerError = e.into();
                            tracing::debug!("Error in optional dependency script: {}", e);
                            return Ok(());
                        }
                        Err(e) => return Err(e.into()),
                    };
                    let stdout = script.stdout.take();
                    let stderr = script.stderr.take();
                    let stdout_name = name.clone();
                    let stderr_name = name.clone();
                    let stdout_on_line = self.0.on_script_line.clone();
                    let stderr_on_line = self.0.on_script_line.clone();
                    let stdout_span = span;
                    let stderr_span = stdout_span.clone();
                    let join = futures::try_join!(
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
                    );
                    match join {
                        Ok(_) => {}
                        Err(e) if is_optional => {
                            tracing::debug!("Error in optional dependency script: {}", e);
                            return Ok(());
                        }
                        Err(e) => return Err(e),
                    }
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

fn package_dir_name(graph: &Graph, idx: NodeIndex) -> String {
    let pkg = &graph[idx].package;
    let subdir = graph
        .node_path(idx)
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join("/node_modules/");

    let mut name = pkg.name().to_string();
    name.push('@');
    let (_, mut hex) = Integrity::from(subdir).to_hex();
    hex.truncate(8);
    name.push_str(&hex);
    name
}

async fn link_deps(
    graph: &Graph,
    node: NodeIndex,
    store_ref: &Path,
    target_nm: &Path,
) -> Result<(), NodeMaintainerError> {
    // Then we symlink/junction all of the package's dependencies into its `node_modules` dir.
    for edge in graph.inner.edges_directed(node, Direction::Outgoing) {
        let dep_pkg = &graph[edge.target()].package;
        let dep_store_dir = store_ref
            .join(package_dir_name(graph, edge.target()))
            .join("node_modules")
            .join(dep_pkg.name());
        let dep_nm_entry = target_nm.join(dep_pkg.name());
        if dep_nm_entry.exists() {
            continue;
        }
        let relative = pathdiff::diff_paths(
            &dep_store_dir,
            dep_nm_entry.parent().expect("must have a parent"),
        )
        .expect("this should never fail");
        async_std::task::spawn_blocking(move || {
            std::fs::create_dir_all(dep_nm_entry.parent().expect("definitely has a parent"))?;
            if dep_nm_entry.symlink_metadata().is_err() {
                // We don't check the link target here because we assume prune() has already been run and removed any incorrect links.
                #[cfg(windows)]
                std::os::windows::fs::symlink_dir(&relative, &dep_nm_entry)
                    .or_else(|_| junction::create(&dep_store_dir, &dep_nm_entry))?;
                #[cfg(unix)]
                std::os::unix::fs::symlink(&relative, &dep_nm_entry)?;
            }
            Ok::<(), NodeMaintainerError>(())
        })
        .await?;
    }
    Ok(())
}

async fn link_dep_bins(
    graph: &Graph,
    node: NodeIndex,
    store_ref: &Path,
    target_bin: &Path,
) -> Result<usize, NodeMaintainerError> {
    let mut linked = 0;
    for edge in graph.inner.edges_directed(node, Direction::Outgoing) {
        let dep_pkg = &graph[edge.target()].package;
        let dep_store_dir = store_ref
            .join(package_dir_name(graph, edge.target()))
            .join("node_modules")
            .join(dep_pkg.name());
        let build_mani =
            BuildManifest::from_path(dep_store_dir.join("package.json")).map_err(|e| {
                NodeMaintainerError::BuildManifestReadError(dep_store_dir.join("package.json"), e)
            })?;
        for (name, path) in &build_mani.bin {
            let target_bin = target_bin.to_owned();
            let to = target_bin.join(name);
            let from = dep_store_dir.join(path);
            let name = name.clone();
            async_std::task::spawn_blocking(move || {
                // We only create a symlink if the target bin exists.
                if from.symlink_metadata().is_ok() {
                    std::fs::create_dir_all(target_bin)?;
                    if let Ok(meta) = to.symlink_metadata() {
                        if meta.is_dir() {
                            std::fs::remove_dir_all(&to)?;
                        } else {
                            std::fs::remove_file(&to)?;
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
            linked += 1;
        }
    }
    Ok(linked)
}
