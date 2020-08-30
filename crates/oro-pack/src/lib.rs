use ignore::{overrides::OverrideBuilder, WalkBuilder};
use oro_manifest::OroManifest;
use std::env;
use std::path::{Path, PathBuf};

const PKG_PATH: &str = "package.json";

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

    pub fn get_pkg_files(&self) -> Vec<PathBuf> {
        let pkg_files = self.get_files();
        let cwd = env::current_dir().unwrap();

        let mut overd = OverrideBuilder::new(&cwd);

        if !pkg_files.is_empty() {
            for f in pkg_files {
                overd.add(f).unwrap();
            }
        }

        let mut results = Vec::new();

        for result in WalkBuilder::new(&cwd)
            .overrides(overd.build().unwrap())
            .build()
        {
            if let Ok(entry) = result {
                results.push(entry.path().to_owned());
            }
        }

        results
            .iter()
            .filter(|f| !f.is_dir())
            .map(|p| p.strip_prefix(&cwd).unwrap().to_path_buf())
            .collect()
    }

    pub fn load_package_json(&mut self) {
        let mut path = env::current_dir().unwrap();

        path.push(PKG_PATH);

        self.pkg = Some(read_package_json(path));
    }

    fn get_files(&self) -> &Vec<String> {
        let pkg = self.pkg.as_ref().unwrap();

        &pkg.files
    }

    fn get_package_name(&self) -> String {
        let pkg = self.pkg.as_ref().unwrap();

        match &pkg.name {
            Some(name) => name.clone(),
            None => panic!("package.json has no name!"),
        }
    }
}
