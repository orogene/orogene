use ignore::{DirEntry, WalkBuilder};
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
        let mut cwd = env::current_dir().unwrap();

        cwd.push("fixtures");

        let mut results: Vec<DirEntry> = Vec::new();

        for result in WalkBuilder::new(&cwd).build() {
            match result {
                Ok(entry) => {
                    results.push(entry);
                }
                Err(err) => println!("ERROR: {}", err),
            }
        }

        results.iter().map(|x| x.path().to_path_buf()).collect()
    }

    pub fn load_package_json_from<P: AsRef<Path>>(&mut self, pkg_path: P) {
        let mut path = env::current_dir().unwrap();

        path.push(pkg_path);

        self.pkg = Some(read_package_json(path));
    }

    pub fn load_package_json(&mut self) {
        self.load_package_json_from(PKG_PATH);
    }

    pub fn get_files(&self) -> &Vec<String> {
        let pkg = self.pkg.as_ref().unwrap();

        &pkg.files
    }

    pub fn get_package_name(&self) -> String {
        let pkg = self.pkg.as_ref().unwrap();

        match &pkg.name {
            Some(name) => name.clone(),
            None => panic!("package.json has no name!"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_package_name() {
        let mut pack = OroPack::new();
        let pkg_path = "fixtures/package.json";

        pack.load_package_json_from(pkg_path);

        assert_eq!(pack.get_package_name(), "testpackage");
    }

    #[test]
    fn paths_ignore_files() {
        let mut pack = OroPack::new();
        let mut cwd = env::current_dir().unwrap();
        cwd.push("fixtures");

        let pkg_path = "fixtures/package.json";

        let expected_paths = vec![
            Path::new("package.json"),
            Path::new("src/index.js"),
            Path::new("src/module.js"),
        ];

        pack.load_package_json_from(pkg_path);

        let files = pack.get_pkg_files();
        let non_directories = files.iter().filter(|f| !f.is_dir()).collect::<Vec<_>>();
        let stripped_paths = non_directories
            .iter()
            .map(|p| p.strip_prefix(&cwd).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(expected_paths, stripped_paths);
    }

    #[test]
    fn paths_respect_files() {
        let mut pack = OroPack::new();
        let mut cwd = env::current_dir().unwrap();
        cwd.push("fixtures");

        let pkg_path = "fixtures/package.json";

        pack.load_package_json_from(pkg_path);

        let expected_paths = vec![Path::new("src/module.js")];

        let pkg_files = pack.get_files();

        if !pkg_files.is_empty() {
            let paths = pkg_files.iter().map(Path::new).collect::<Vec<_>>();
            assert_eq!(expected_paths, paths);
        }
    }
}
