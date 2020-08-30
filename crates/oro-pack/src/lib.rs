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
        let pkg_files = self.get_files();

        if !pkg_files.is_empty() {
            return pkg_files.iter().map(PathBuf::from).collect::<Vec<_>>();
        }

        let cwd = env::current_dir().unwrap();
        let mut results: Vec<DirEntry> = Vec::new();

        for result in WalkBuilder::new(&cwd).build() {
            match result {
                Ok(entry) => {
                    results.push(entry);
                }
                Err(err) => println!("ERROR: {}", err),
            }
        }

        let path_bufs = results
            .iter()
            .map(|x| x.path().to_path_buf())
            .collect::<Vec<_>>();
        let non_directories = path_bufs.iter().filter(|f| !f.is_dir()).collect::<Vec<_>>();
        let stripped_paths = non_directories
            .iter()
            .map(|p| p.strip_prefix(&cwd).unwrap().to_path_buf())
            .collect::<Vec<_>>();

        stripped_paths
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_no_files_field() {
        let mut cwd = env::current_dir().unwrap();
        cwd.push("fixtures/implicit_files");
        env::set_current_dir(cwd).unwrap();

        let mut pack = OroPack::new();

        let expected_paths = vec![
            Path::new("package.json"),
            Path::new("src/index.js"),
            Path::new("src/module.js"),
        ];

        pack.load_package_json();

        let files = pack.get_pkg_files();

        assert_eq!(expected_paths, files);
    }

    #[test]
    fn paths_respect_files() {
        let mut cwd = env::current_dir().unwrap();
        cwd.push("fixtures/explicit_files");
        env::set_current_dir(cwd).unwrap();

        let mut pack = OroPack::new();

        pack.load_package_json();

        let expected_paths = vec![Path::new("src/module.js")];

        let pkg_files = pack.get_pkg_files();

        assert_eq!(expected_paths, pkg_files);
    }
}
