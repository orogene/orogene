use gitignored::Gitignore;
use oro_manifest::OroManifest;
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MANIFEST_PATH: &str = "package.json";

const ALWAYS_INCLUDED: [&str; 8] = [
    "/readme*",
    "/copying*",
    "/license*",
    "/licence*",
    "/notice*",
    "/changes*",
    "/changelog*",
    "/history*",
];

fn read_package_json<P: AsRef<Path>>(pkg_path: P) -> OroManifest {
    match OroManifest::from_file(pkg_path) {
        Ok(pkg) => pkg,
        Err(e) => panic!("Problem loading package.json: {:?}", e),
    }
}

fn find_pkg_paths(patterns: Vec<String>) -> Vec<PathBuf> {
    let cwd = env::current_dir().unwrap();
    let mut ig = Gitignore::default();

    let mut patterns_as_slice: Vec<&str> = patterns.iter().map(AsRef::as_ref).collect();
    let mut paths = Vec::new();

    // Always include certain files
    for inc in ALWAYS_INCLUDED.iter() {
        patterns_as_slice.push(inc);
    }

    for entry in WalkDir::new(&cwd).into_iter().filter_entry(|e| {
        let stripped = e.path().strip_prefix(&cwd).unwrap();

        // TODO: avoid converting stripped path to str for comparison
        let should_descend = patterns_as_slice
            .iter()
            .any(|p| p.starts_with(stripped.to_str().unwrap()));
        ig.ignores(&patterns_as_slice, e.path()) || e.path() == ig.root || should_descend
    }) {
        let entry = entry.unwrap();
        if !entry.path().is_dir() {
            paths.push(entry.path().to_path_buf());
        }
    }

    paths
}

pub struct OroPack {
    pkg: Option<OroManifest>,
}

impl Default for OroPack {
    fn default() -> Self {
        Self::new()
    }
}

impl OroPack {
    pub fn new() -> Self {
        OroPack { pkg: None }
    }

    /// Get a list of all paths that will be included in a package.
    pub fn project_paths(&self) -> Vec<PathBuf> {
        let pkg_files = self.pkg_files();
        let cwd = env::current_dir().unwrap();

        let mut pj_paths = find_pkg_paths(pkg_files);

        let pkg_json = PathBuf::from("package.json");

        if !pj_paths.contains(&pkg_json) {
            pj_paths.push(cwd.join(pkg_json));
        }

        pj_paths.sort();
        pj_paths.dedup();

        pj_paths
            .iter()
            .filter(|f| !f.is_dir())
            .map(|p| p.strip_prefix(&cwd).unwrap().to_path_buf())
            .collect()
    }

    /// Load package.json.
    pub fn load(&mut self) {
        let mut path = env::current_dir().unwrap();
        path.push(MANIFEST_PATH);
        self.pkg = Some(read_package_json(path));
    }

    fn pkg_files(&self) -> Vec<String> {
        let pkg = self.pkg.as_ref().unwrap();

        match &pkg.files {
            Some(files) => files.clone(),
            None => panic!("package.json must have files field!"),
        }
    }
}
