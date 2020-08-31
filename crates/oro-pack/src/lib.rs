use ignore::{
    overrides::{Override, OverrideBuilder},
    WalkBuilder,
};
use oro_manifest::OroManifest;
use std::env;
use std::path::{Path, PathBuf};

const PKG_PATH: &str = "package.json";
const ALWAYS_IGNORED: [&str; 24] = [
    ".gitignore",
    ".npmignore",
    "**/.git",
    "**/.svn",
    "**/.hg",
    "**/CVS",
    "**/.git/**",
    "**/.svn/**",
    "**/.hg/**",
    "**/CVS/**",
    "/.lock-wscript",
    "/.wafpickle-*",
    "/build/config.gypi",
    "npm-debug.log",
    "**/.npmrc",
    ".*.swp",
    ".DS_Store",
    "**/.DS_Store/**",
    "._*",
    "**/._*/**",
    "*.orig",
    "/package-lock.json",
    "/yarn.lock",
    "/archived-packages/**",
];

fn read_package_json<P: AsRef<Path>>(pkg_path: P) -> OroManifest {
    match OroManifest::from_file(pkg_path) {
        Ok(pkg) => pkg,
        Err(e) => panic!("Problem loading package.json: {:?}", e),
    }
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

    /// Ignore cruft and always include paths specicied in the files field of package.json.
    /// Use reverse gitignore syntax.
    fn generate_overrides(&self, pkg_files: &Vec<String>) -> Override {
        let mut builder = OverrideBuilder::new(env::current_dir().unwrap());

        for ig in ALWAYS_IGNORED.iter() {
            let rev = format!("!{}", ig);
            builder.add(&rev).unwrap();
        }

        if !pkg_files.is_empty() {
            for f in pkg_files {
                builder.add(f).unwrap();
            }
        }

        builder.build().unwrap()
    }

    /// Get a list of all paths that will be included in a package.
    pub fn project_paths(&self) -> Vec<PathBuf> {
        let pkg_files = self.pkg_files();
        let overrides = self.generate_overrides(pkg_files);
        let force_include_pkg_json = overrides.num_whitelists() > 0;

        let mut paths = Vec::new();

        let cwd = env::current_dir().unwrap();

        for path in WalkBuilder::new(env::current_dir().unwrap())
            .overrides(overrides.clone())
            .add_custom_ignore_filename(".gitignore")
            .build()
        {
            if let Ok(entry) = path {
                paths.push(entry.path().to_owned());
            }
        }

        if force_include_pkg_json {
            paths.push(cwd.join(PathBuf::from("package.json")));
        }

        paths
            .iter()
            .filter(|f| !f.is_dir())
            .map(|p| p.strip_prefix(&cwd).unwrap().to_path_buf())
            .collect()
    }

    /// Load package.json.
    pub fn load(&mut self) {
        let mut path = env::current_dir().unwrap();

        path.push(PKG_PATH);

        self.pkg = Some(read_package_json(path));
    }

    fn pkg_files(&self) -> &Vec<String> {
        let pkg = self.pkg.as_ref().unwrap();

        &pkg.files
    }
}
