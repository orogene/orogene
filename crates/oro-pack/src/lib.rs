use oro_manifest::OroManifest;
use std::env;
use std::path::Path;

fn read_package_json<P: AsRef<Path>>(pkg_path: P) -> OroManifest {
    match OroManifest::from_file(pkg_path) {
        Ok(pkg) => pkg,
        Err(e) => panic!("Problem loading package.json: {:?}", e),
    }
}

pub struct OroPack {
    pkg: Option<OroManifest>,
}

impl OroPack {
    pub fn new() -> Self {
        OroPack { pkg: None }
    }

    pub fn load_package_json_from<P: AsRef<Path>>(&mut self, pkg_path: P) {
        let mut path = env::current_dir().unwrap();

        path.push(pkg_path);

        self.pkg = Some(read_package_json(path));
    }

    pub fn load_package_json(&mut self) {
        self.load_package_json_from("package.json");
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
}
