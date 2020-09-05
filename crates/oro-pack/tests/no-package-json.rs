use oro_pack::OroPack;
use std::env;
use tempfile::tempdir;

#[test]
#[should_panic]
fn no_pkg_json() {
    let cwd = env::current_dir().unwrap();

    let dir = tempdir().unwrap();

    env::set_current_dir(dir.path()).unwrap();

    let mut pack = OroPack::new();

    pack.load();

    env::set_current_dir(cwd).unwrap();

    dir.close().unwrap();
}
