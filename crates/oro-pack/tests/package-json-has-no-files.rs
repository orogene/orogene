use oro_pack::*;
use std::env;
use std::path::Path;

#[test]
fn pkg_json_has_no_files() -> std::io::Result<()> {
    let mut cwd = env::current_dir()?;
    cwd.push("fixtures/implicit_files");
    env::set_current_dir(cwd)?;

    let mut pack = OroPack::new();
    let mut expected_paths = vec![
        Path::new("README.md"),
        Path::new("package.json"),
        Path::new("src/index.js"),
        Path::new("src/module.js"),
    ];

    pack.load();

    expected_paths.sort();

    assert_eq!(expected_paths, pack.project_paths());

    Ok(())
}
