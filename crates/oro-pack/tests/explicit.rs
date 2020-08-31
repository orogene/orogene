use oro_pack::*;
use std::env;
use std::path::Path;

#[test]
fn paths_respect_files() {
    let mut cwd = env::current_dir().unwrap();
    cwd.push("fixtures/explicit_files");
    env::set_current_dir(cwd).unwrap();

    let mut pack = OroPack::new();

    pack.load();

    let expected_paths = vec![Path::new("src/module.js")];

    let pkg_files = pack.project_paths();

    assert_eq!(expected_paths, pkg_files);
}
