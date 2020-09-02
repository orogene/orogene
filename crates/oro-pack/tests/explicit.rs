use oro_pack::*;
use std::env;
use std::path::Path;

#[test]
fn paths_respect_files() -> std::io::Result<()> {
    let mut cwd = env::current_dir()?;
    cwd.push("fixtures/explicit_files");
    env::set_current_dir(cwd)?;

    let mut pack = OroPack::new();

    pack.load();

    let mut expected_paths = vec![
        Path::new("src/module.js"),
        Path::new("package.json"),
        Path::new("README.md"),
    ];

    let mut pkg_files = pack.project_paths();

    expected_paths.sort();
    pkg_files.sort();

    assert_eq!(expected_paths, pkg_files);

    Ok(())
}
