use fs::File;
use oro_pack::*;
use std::env;
use std::io::Write as _;
use std::{fs, path::Path};
use tempfile::tempdir;

#[test]
#[should_panic]
fn pkg_json_has_no_files() {
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
