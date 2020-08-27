use oro_manifest::OroManifest;
use std::env;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_package_name() {
        let mut test_path = env::current_dir().unwrap();

        test_path.push("fixtures/package.json");

        let pkg = read_package_json(&test_path);

        let r = OroPack::get_package_name(pkg);
        assert_eq!(r, "testpackage");
    }
}

fn read_package_json(pkg_path: &Path) -> OroManifest {
    match OroManifest::from_file(pkg_path) {
        Ok(pkg) => pkg,
        Err(e) => panic!("Problem loading package.json: {:?}", e),
    }
}

pub struct OroPack;

impl OroPack {
    pub fn get_package_name(pkg: OroManifest) -> String {
        match pkg.name {
            Some(name) => name,
            None => panic!("Package has no name!"),
        }
    }
}
