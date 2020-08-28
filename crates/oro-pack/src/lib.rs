use oro_manifest::OroManifest;
use std::env;
use std::path::Path;

fn read_package_json(pkg_path: &Path) -> OroManifest {
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

    pub fn load_package_json<P: AsRef<Path>>(&mut self, cwd: Option<P>) {
        let mut path = cwd
            .map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(|| env::current_dir().unwrap());

        path.push("package.json");

        let pkg = read_package_json(&path);

        self.pkg = Some(pkg);
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
        let mut cwd = env::current_dir().unwrap();

        cwd.push("fixtures");

        pack.load_package_json(Some(cwd));

        assert_eq!(pack.get_package_name(), "testpackage");
    }
}
