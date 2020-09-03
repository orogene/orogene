use fs::File;
use oro_pack::OroPack;
use std::env;
use std::fs;
use tempfile::tempdir;

#[test]
#[should_panic]
fn pkg_json_empty() {
    let cwd = env::current_dir().unwrap();

    let dir = tempdir().unwrap();
    let dir_path = dir.path();
    let pkg_path = dir_path.join("package.json");

    let pkg_json = File::create(pkg_path).unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let mut pack = OroPack::new();

    pack.load();

    env::set_current_dir(cwd).unwrap();

    drop(pkg_json);

    dir.close().unwrap();
}
