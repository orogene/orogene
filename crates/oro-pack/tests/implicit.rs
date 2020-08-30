use oro_pack::*;
use std::env;
use std::path::Path;

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
